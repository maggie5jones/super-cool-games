use assets_manager::{asset::Png, AssetCache};
use engine::level::Level;
use engine::Contact;
use engine::Dir;
use engine::Pos;
use engine::{geom::*, World};
use frenderer::{
    input::{Input, Key},
    sprites::{Camera2D, SheetRegion, Transform},
    wgpu, Immediate,
};
use rand::Rng;

const PLAYER: SheetRegion = SheetRegion::rect(51, 0, 16, 16);
const KNIGHT1: SheetRegion = SheetRegion::rect(119, 17, 16, 16);
const ENEMY: SheetRegion = SheetRegion::rect(17, 0, 16, 16);

const TILE_SZ: usize = 16;
const W: usize = 516; // 320
const H: usize = 240; // 240
const SCREEN_FAST_MARGIN: f32 = 64.0;

// pixels per second
const PLAYER_SPEED: f32 = 64.0;
const ENEMY_SPEED: f32 = 32.0;

const DT: f32 = 1.0 / 60.0;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let source =
        assets_manager::source::FileSystem::new("content").expect("Couldn't load resources");
    #[cfg(target_arch = "wasm32")]
    let source = assets_manager::source::Embedded::from(assets_manager::source::embed!("content"));
    let cache = assets_manager::AssetCache::with_source(source);

    engine::main_loop::<SimGame>(cache, 516.0, 240.0);
}
struct SimGame {
    pub humans: Vec<Knight>,
}

struct Knight {
    pub health: u8,
    pub pos: Vec2,
}

impl Knight {
    fn find_enemy(&mut self, world: &mut World) -> Vec2 {
        let mut closest = Vec2 {
            x: W as f32,
            y: H as f32,
        };
        for enemy in world.enemies.iter_mut() {
            if (enemy.0.pos + (self.pos * -1.0)).mag_sq().sqrt()
                < (closest + (self.pos * -1.0)).mag_sq().sqrt()
            {
                closest = enemy.0.pos;
            }
        }
        closest
    }
}

impl SimGame {
    fn new(world: &mut World) -> Self {
        let game = SimGame { humans: vec![] };
        let player_start = *world.levels[world.current_level]
            .starts()
            .iter()
            .find(|(t, _)| t.name() == "player")
            .map(|(_, ploc)| ploc)
            .expect("Start level doesn't put the player anywhere");
        world.enter_level(player_start);
        game
    }
    fn simulate(&mut self, world: &mut World, input: &Input, dt: f32) {
        if input.is_key_pressed(Key::KeyQ) {
            world.spawn_enemies();
        }
        if input.is_key_pressed(Key::KeyE) {
            spawn_humans(world, self);
        }
        if input.is_key_pressed(Key::Escape) {
            world.pause();
        }
        if world.paused || world.game_end {
            return;
        }

        let dx = input.key_axis(Key::ArrowLeft, Key::ArrowRight) * PLAYER_SPEED * DT;
        // now down means -y and up means +y!  beware!
        let dy = input.key_axis(Key::ArrowDown, Key::ArrowUp) * PLAYER_SPEED * DT;
        let dest = world.player.pos + Vec2 { x: dx, y: dy };
        if !world.level().get_tile_at(dest).unwrap().solid {
            world.player.pos = dest;
        }

        // for enemy in world.enemies.iter_mut() {
        //     let player_pos = world.player.pos;
        //     let enemy_pos = enemy.0.pos;
        //     let mut direction = Vec2 { x: 0.0, y: 0.0 };
        //     direction.x  = player_pos.x - enemy_pos.x;
        //     direction.y = player_pos.y - enemy_pos.y;
        //     let normalized_direction = direction.normalize();
        //     enemy.0.pos += normalized_direction * ENEMY_SPEED * dt;
        // }
        let mut rng = rand::thread_rng();
        for enemy in world.enemies.iter_mut() {
            if rng.gen_bool(0.05) {
                enemy.0.dir = match rng.gen_range(0..4) {
                    0 => Dir::N,
                    1 => Dir::E,
                    2 => Dir::S,
                    3 => Dir::W,
                    _ => panic!(),
                };
            }
            let enemy_dest = enemy.0.pos + (enemy.0.dir.to_vec2() * ENEMY_SPEED * dt);
            if (enemy_dest.x >= 0.0
                && enemy_dest.x <= (world.levels[world.current_level].width() * TILE_SZ) as f32)
                && (enemy_dest.y > 0.0
                    && enemy_dest.y
                        <= (world.levels[world.current_level].height() * TILE_SZ) as f32)
            {
                enemy.0.pos = enemy_dest;
            }
        }

        for human in self.humans.iter_mut() {
            let monster_pos = human.find_enemy(world);
            let human_pos = human.pos;
            let mut direction = Vec2 { x: 0.0, y: 0.0 };
            direction.x = monster_pos.x - human_pos.x;
            direction.y = monster_pos.y - human_pos.y;
            let normalized_direction = direction.normalize();
            human.pos += normalized_direction * ENEMY_SPEED * dt;
        }

        let lw = world.level().width();
        let lh = world.level().height();

        while world.player.pos.x
            > world.camera.screen_pos[0] + world.camera.screen_size[0] - SCREEN_FAST_MARGIN
        {
            world.camera.screen_pos[0] += 1.0;
        }
        while world.player.pos.x < world.camera.screen_pos[0] + SCREEN_FAST_MARGIN {
            world.camera.screen_pos[0] -= 1.0;
        }
        while world.player.pos.y
            > world.camera.screen_pos[1] + world.camera.screen_size[1] - SCREEN_FAST_MARGIN
        {
            world.camera.screen_pos[1] += 1.0;
        }
        while world.player.pos.y < world.camera.screen_pos[1] + SCREEN_FAST_MARGIN {
            world.camera.screen_pos[1] -= 1.0;
        }
        world.camera.screen_pos[0] =
            world.camera.screen_pos[0].clamp(0.0, (lw * TILE_SZ).max(W) as f32 - W as f32);
        world.camera.screen_pos[1] =
            world.camera.screen_pos[1].clamp(0.0, (lh * TILE_SZ).max(H) as f32 - H as f32);

        let mut contacts = Vec::new();
        let p_rect = Rect {
            x: world.player.pos.x - (TILE_SZ / 2) as f32,
            y: world.player.pos.y - (TILE_SZ / 2) as f32,
            w: (TILE_SZ) as u16,
            h: (TILE_SZ) as u16,
        };
        let player = [p_rect];
        let enemy_rect: Vec<_> = world.enemies.iter().map(|e| make_rect(e.0.pos)).collect();
        generate_contact(&player, &enemy_rect, &mut contacts);

        let knight_rect: Vec<_> = self.humans.iter().map(|e| make_rect(e.pos)).collect();
        let mut knight_contacts: Vec<Contact>;
        knight_contacts = vec![];
        generate_contact(&knight_rect, &enemy_rect, &mut knight_contacts);

        // Tile and Player contacts
        let mut tile_contacts = Vec::new();
        generate_tile_contact(&[player[0]], world.level(), &mut tile_contacts);

        // Tile and Enemy contacts
        let mut tile_enemy_contacts = Vec::new();
        generate_tile_contact(&enemy_rect, world.level(), &mut tile_enemy_contacts);

        // Tile and Knight contacts
        let mut tile_knight_contacts = Vec::new();
        generate_tile_contact(&knight_rect, world.level(), &mut tile_knight_contacts);

        // Contact Resolution for player vs. world
        tile_contacts.sort_by(|a, b| {
            b.displacement
                .mag_sq()
                .partial_cmp(&a.displacement.mag_sq())
                .unwrap()
        });
        for contact in tile_contacts {
            world.player.pos += find_displacement(p_rect, contact.b_rect);
        }

        // Contact Resolution for enemies vs. world
        tile_enemy_contacts.sort_by(|a, b| {
            b.displacement
                .mag_sq()
                .partial_cmp(&a.displacement.mag_sq())
                .unwrap()
        });
        for contact in tile_enemy_contacts {
            world.enemies[contact.a_index].0.pos +=
                find_displacement(enemy_rect[contact.a_index], contact.b_rect);
        }

        // Contact Resolution for knight vs. world
        tile_knight_contacts.sort_by(|a, b| {
            b.displacement
                .mag_sq()
                .partial_cmp(&a.displacement.mag_sq())
                .unwrap()
        });
        for contact in tile_knight_contacts {
            self.humans[contact.a_index].pos +=
                find_displacement(knight_rect[contact.a_index], contact.b_rect);
        }

        // For deleting enemies, it's best to add the enemy to a "to_remove" vec, and then remove those enemies after this loop is all done.
        contacts.sort_by(|a, b| {
            b.displacement
                .mag_sq()
                .partial_cmp(&a.displacement.mag_sq())
                .unwrap()
        });

        let mut removable = Vec::new();
        let mut knight_removable = Vec::new();
        for contact in knight_contacts {
            if !removable.contains(&contact.b_index) {
                world.enemies[contact.b_index].0.pos +=
                    find_displacement(knight_rect[contact.a_index], enemy_rect[contact.b_index]);
                removable.push(contact.b_index);
                self.humans[contact.a_index].health -= 1;
                if self.humans[contact.a_index].health == 0 {
                    knight_removable.push(contact.a_index);
                }
            }
        }
        // Alternatively, you could "disable" an enemy by giving it an `alive` flag or similar and setting that to false, not drawing or updating dead enemies.
        removable.sort();
        for i in removable.iter().rev() {
            world.enemies.swap_remove(*i);
        }

        knight_removable.sort();
        for i in knight_removable.iter().rev() {
            self.humans.swap_remove(*i);
        }
    }
}

impl engine::Game for SimGame {
    fn update(&mut self, world: &mut engine::World, input: &Input) {
        self.simulate(world, input, DT);
    }
    fn render(&mut self, world: &mut engine::World, frend: &mut Immediate) {
        // make this exactly as big as we need
        frend.sprite_group_set_camera(0, world.camera);

        world.level().render_immediate(frend);

        frend.draw_sprite(
            0,
            Transform {
                w: TILE_SZ as u16,
                h: TILE_SZ as u16,
                x: world.player.pos.x,
                y: world.player.pos.y,
                rot: 0.0,
            },
            PLAYER.with_depth(2),
        );

        for enemy in world.enemies.iter() {
            if enemy.1 == 1 {
                frend.draw_sprite(
                    0,
                    Transform {
                        w: TILE_SZ as u16,
                        h: TILE_SZ as u16,
                        x: enemy.0.pos.x,
                        y: enemy.0.pos.y,
                        rot: 0.0,
                    },
                    ENEMY.with_depth(3),
                );
            } else {
                frend.draw_sprite(0, Transform::ZERO, SheetRegion::ZERO);
            }
        }

        for knight in self.humans.iter() {
            frend.draw_sprite(
                0,
                Transform {
                    w: TILE_SZ as u16,
                    h: TILE_SZ as u16,
                    x: knight.pos.x,
                    y: knight.pos.y,
                    rot: 0.0,
                },
                KNIGHT1.with_depth(2),
            );
        }

        if world.game_end {
            // player disappears when game ends (no more health)
            frend.draw_sprite(
                0,
                Transform {
                    w: TILE_SZ as u16,
                    h: TILE_SZ as u16,
                    x: world.player.pos.x,
                    y: world.player.pos.y,
                    rot: 0.0,
                },
                SheetRegion::ZERO,
            );
        }

        if world.paused {
            let nine_tiled = frenderer::nineslice::NineSlice::with_corner_edge_center(
                frenderer::nineslice::CornerSlice {
                    w: 16.0,
                    h: 16.0,
                    region: SheetRegion::rect(0, 306, 16, 16).with_depth(1),
                },
                frenderer::nineslice::Slice {
                    w: 16.0,
                    h: 16.0,
                    region: SheetRegion::rect(34, 306, 16, 16).with_depth(1),
                    repeat: frenderer::nineslice::Repeat::Tile,
                },
                frenderer::nineslice::Slice {
                    w: 16.0,
                    h: 16.0,
                    region: SheetRegion::rect(17, 306, 16, 16).with_depth(1),
                    repeat: frenderer::nineslice::Repeat::Tile,
                },
                frenderer::nineslice::Slice {
                    w: 16.0,
                    h: 16.0,
                    region: SheetRegion::rect(51, 306, 16, 16).with_depth(1),
                    repeat: frenderer::nineslice::Repeat::Tile,
                },
            );
            let pause_x = W as f32 / 2.0 - 8.0 * TILE_SZ as f32;
            let pause_y = H as f32 / 2.0 - 3.0 * TILE_SZ as f32;
            frend.draw_nineslice(
                1,
                &nine_tiled,
                pause_x,
                pause_y,
                16.0 * TILE_SZ as f32,
                6.0 * TILE_SZ as f32,
                0,
            );

            let font = frenderer::bitfont::BitFont::with_sheet_region(
                ' '..='ÿ',
                SheetRegion::new(0, 0, 385, 0, 288, 70).with_depth(0),
                8_u16,
                8_u16,
                1_u16,
                2_u16,
            );

            let mut text = "game paused!";
            frend.draw_text(
                1,
                &font,
                text,
                [
                    (W / 2) as f32 - 3.0 * TILE_SZ as f32,
                    (H / 2) as f32 + 2.5 * TILE_SZ as f32,
                ],
                0,
                (TILE_SZ / 2) as f32,
            );
            text = "unpause: Esc";
            frend.draw_text(
                1,
                &font,
                text,
                [
                    (W / 2) as f32 - 3.25 * TILE_SZ as f32,
                    (H / 2) as f32 - 2.0 * TILE_SZ as f32,
                ],
                0,
                (TILE_SZ / 2) as f32,
            );
            text = "q: spawn skeletons";
            frend.draw_text(
                1,
                &font,
                text,
                [
                    (W / 2) as f32 - 4.5 * TILE_SZ as f32,
                    (H / 2) as f32 + 1.0 * TILE_SZ as f32,
                ],
                0,
                (TILE_SZ / 2) as f32,
            );
            text = "e: spawn knights";
            frend.draw_text(
                1,
                &font,
                text,
                [
                    (W / 2) as f32 - 4.5 * TILE_SZ as f32,
                    (H / 2) as f32 + 0.0 * TILE_SZ as f32,
                ],
                0,
                (TILE_SZ / 2) as f32,
            );
        }
    }
    fn new(renderer: &mut Immediate, cache: AssetCache, world: &mut engine::World) -> Self {
        let tile_handle = cache
            .load::<Png>("tilemap")
            .expect("Couldn't load tilesheet img");
        let tile_img = tile_handle.read().0.to_rgba8();
        let tile_tex = renderer.create_array_texture(
            &[&tile_img],
            wgpu::TextureFormat::Rgba8UnormSrgb,
            tile_img.dimensions(),
            Some("tiles-sprites"),
        );

        let levels = vec![Level::from_str(
            &cache
                .load::<String>("level3")
                .expect("Couldn't access level3.txt")
                .read(),
            0,
            0,
        )];
        let current_level = 0;
        let camera = Camera2D {
            screen_pos: [0.0, 0.0],
            screen_size: [W as f32, H as f32], // 512x240
        };
        let sprite_estimate =
            levels[current_level].sprite_count() + levels[current_level].starts().len();
        // tile sprite group: 0
        renderer.sprite_group_add(&tile_tex, sprite_estimate, camera);
        // HUD sprite group: 1
        renderer.sprite_group_add(&tile_tex, sprite_estimate, camera);
        let player_start = *levels[current_level]
            .starts()
            .iter()
            .find(|(t, _)| t.name() == "player")
            .map(|(_, ploc)| ploc)
            .expect("Start level doesn't put the player anywhere");
        world.set_camera(camera);
        world.set_levels(levels);
        world.set_current_level(current_level);
        world.set_enemies(vec![]);
        world.set_player(Pos {
            pos: player_start,
            dir: Dir::S,
        });
        SimGame::new(world)
    }
}

fn spawn_humans(world: &mut World, game: &mut SimGame) {
    let mut rng = rand::thread_rng();
    let mut randx = rng
        .gen_range(2 * TILE_SZ..world.levels[world.current_level].width() * TILE_SZ - 2 * TILE_SZ);
    let mut randy = rng
        .gen_range(2 * TILE_SZ..world.levels[world.current_level].height() * TILE_SZ - 2 * TILE_SZ);
    while ((randx as f32 - world.player.pos.x).abs() < 48.0)
        && ((randy as f32 - world.player.pos.y).abs() < 48.0)
        && !world
            .level()
            .get_tile_at(Vec2 {
                x: randx as f32,
                y: randy as f32,
            })
            .unwrap()
            .solid
    {
        randx = rng.gen_range(2..world.levels[world.current_level].width() * TILE_SZ);
        randy = rng.gen_range(2..world.levels[world.current_level].height() * TILE_SZ);
    }
    let knight_data = Knight {
        health: 3,
        pos: Vec2 {
            x: randx as f32,
            y: randy as f32,
        },
    };
    game.humans.push(knight_data);
}

fn generate_contact(group_a: &[Rect], group_b: &[Rect], contacts: &mut Vec<Contact>) {
    for (a_i, a_rect) in group_a.iter().enumerate() {
        for (b_i, b_rect) in group_b.iter().enumerate() {
            if let Some(overlap) = a_rect.overlap(*b_rect) {
                contacts.push(Contact {
                    displacement: overlap,
                    a_index: a_i,
                    _a_rect: *a_rect,
                    b_index: b_i,
                    b_rect: *b_rect,
                });
            }
        }
    }
}

fn generate_tile_contact(group_a: &[Rect], lvl: &Level, contacts: &mut Vec<Contact>) {
    for (a_i, a_rect) in group_a.iter().enumerate() {
        for (_, b_rect, _) in lvl.tiles_within(*a_rect).filter(|(_, _r, td)| td.solid) {
            if let Some(overlap) = a_rect.overlap(b_rect) {
                contacts.push(Contact {
                    displacement: overlap,
                    a_index: a_i,
                    _a_rect: *a_rect,
                    b_index: 0,
                    b_rect,
                });
            }
        }
    }
}

fn find_displacement(a: Rect, b: Rect) -> Vec2 {
    if let Some(mut overlap) = a.overlap(b) {
        if overlap.x < overlap.y {
            overlap.y = 0.0;
        } else {
            overlap.x = 0.0;
        }
        if a.x < b.x {
            overlap.x *= -1.0;
        }
        if a.y < b.y {
            overlap.y *= -1.0;
        }
        overlap
    } else {
        Vec2 { x: 0.0, y: 0.0 }
    }
}

fn make_rect(position: Vec2) -> Rect {
    Rect {
        x: position.x - (TILE_SZ / 2) as f32,
        y: position.y - (TILE_SZ / 2) as f32,
        w: TILE_SZ as u16,
        h: TILE_SZ as u16,
    }
}
