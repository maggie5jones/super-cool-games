use assets_manager::{asset::Png, AssetCache};
use frenderer::{
    input::{Input, Key}, sprites::{Camera2D, SheetRegion, Transform}, wgpu, Immediate
};
use engine::level::Level;
use engine::Contact;
use engine::Dir;
use engine::Pos;
use engine::{geom::*, World};
use stopwatch::Stopwatch;
use std::io;

const PLAYER: [SheetRegion; 4] = [
    //n, e, s, w
    SheetRegion::rect(461 + 16 * 2, 39, 16, 16),
    SheetRegion::rect(461, 39, 16, 16),
    SheetRegion::rect(461 + 16 * 3, 39, 16, 16),
    SheetRegion::rect(461 + 16, 39, 16, 16),
];
const TILE_SZ: usize = 16;
const W: usize = 220; // 320
const H: usize = 140; // 240
const SCREEN_FAST_MARGIN: f32 = 64.0;

// pixels per second
const PLAYER_SPEED: f32 = 64.0;
const _KNOCKBACK_SPEED: f32 = 128.0;

const DT: f32 = 1.0 / 60.0;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let source =
        assets_manager::source::FileSystem::new("content").expect("Couldn't load resources");
    #[cfg(target_arch = "wasm32")]
    let source = assets_manager::source::Embedded::from(assets_manager::source::embed!("content"));
    let cache = assets_manager::AssetCache::with_source(source);

    engine::main_loop::<MazeGame>(cache);
}

struct MazeGame {
    stopwatch: Stopwatch,
    leaderboard: Vec<(String, String)>, // TODO: have this instead be something that lives in the file (not new with each game)
}

impl MazeGame {
    fn new(world: &mut World) -> Self {
        let game = MazeGame {
            stopwatch: Stopwatch::start_new(),
            leaderboard: vec![],
        };
        let player_start = *world.levels[world.current_level]
            .starts()
            .iter()
            .find(|(t, _)| t.name() == "player")
            .map(|(_, ploc)| ploc)
            .expect("Start level doesn't put the player anywhere");
        world.enter_level(player_start);
        game
    }

    fn draw_hud(&self, frend: &mut Immediate) {
        let timer_pos = Transform {
            w: TILE_SZ as u16, 
            h: TILE_SZ as u16,
            x: 6.0,
            y: 10.0,
            rot: 0.0,
        };

        let font = frenderer::bitfont::BitFont::with_sheet_region(
            ' '..='ÿ', 
            SheetRegion::new(0, 0, 143, 0, 288, 70).with_depth(0), 
            (8) as u16, 
            (8) as u16, 
            (1) as u16, 
            (2) as u16);
        
        let timer = self.stopwatch.elapsed().as_millis().to_string();
        let seconds;
        let milliseconds;

        if timer.len() >= 3 {
            seconds = timer[0..timer.len()-3].to_string();
            milliseconds = timer[timer.len()-3..timer.len()].to_string();
        }
        else {
            seconds = "0".to_string();
            milliseconds = timer[0..timer.len()].to_string();
        }

        let timer_str = format!("{}:{}", seconds, milliseconds);

        frend.draw_text(1, &font, &timer_str, [timer_pos.x, timer_pos.y], 0, (TILE_SZ/2) as f32);
    }

    fn draw_leaderboard(&mut self, frend: &mut Immediate, world: &mut World) {
        let nine_tiled = frenderer::nineslice::NineSlice::with_corner_edge_center(
            frenderer::nineslice::CornerSlice {
                w: 16.0,
                h: 16.0,
                region: SheetRegion::rect(628, 55, 16, 16).with_depth(1),
            }, 
            frenderer::nineslice::Slice {
                w: 16.0,
                h: 16.0,
                region: SheetRegion::rect(662, 55, 16, 16).with_depth(1),
                repeat: frenderer::nineslice::Repeat::Tile,
            }, 
            frenderer::nineslice::Slice {
                w: 16.0,
                h: 16.0,
                region: SheetRegion::rect(645, 55, 16, 16).with_depth(1),
                repeat: frenderer::nineslice::Repeat::Tile,
            }, 
            frenderer::nineslice::Slice {
                w: 16.0,
                h: 16.0,
                region: SheetRegion::rect(679, 55, 16, 16).with_depth(1),
                repeat: frenderer::nineslice::Repeat::Tile,
            });

        let pause_x = W as f32/2.0 - 4.0*TILE_SZ as f32; 
        let pause_y = H as f32/2.0 - 3.0*TILE_SZ as f32; 
        
        // game end, draw leaderboard
        world.paused = true;
        frend.draw_nineslice(1, &nine_tiled, pause_x, pause_y, 8.0*TILE_SZ as f32, 6.0*TILE_SZ as f32, 0);  

        let name: String = get_user_input();
            
        let timer = self.stopwatch.elapsed().as_millis().to_string();
        let seconds;
        let milliseconds;

        if timer.len() >= 3 {
            seconds = timer[0..timer.len()-3].to_string();
            milliseconds = timer[timer.len()-3..timer.len()].to_string();
        }
        else {
            seconds = "0".to_string();
            milliseconds = timer[0..timer.len()].to_string();
        }

        let timer_str = format!("{}:{}", seconds, milliseconds);
        //dbg!(timer_str);

        self.leaderboard.push((name, timer_str));
        self.leaderboard.sort_by(|a, b| a.1.cmp(&b.1));

        let font = frenderer::bitfont::BitFont::with_sheet_region(
            ' '..='ÿ', 
            SheetRegion::new(0, 0, 143, 0, 288, 70).with_depth(0), 
            (8) as u16, 
            (8) as u16, 
            (1) as u16, 
            (2) as u16);
                
        frend.draw_text(1, &font, "leaderboard", [(W/2) as f32 - 3.0*TILE_SZ as f32, (H/2) as f32 + TILE_SZ as f32], 0, (TILE_SZ/2) as f32);

        let max;
        if self.leaderboard.len() > 3 {
            max = 3;
        } else {
            max = self.leaderboard.len();
        }

        for i in 0..max {
            let text = format!("{}: {}", self.leaderboard[i].0, self.leaderboard[i].1);
            frend.draw_text(1, &font, &text, [(W/2) as f32 - 3.0*TILE_SZ as f32, (H/2) as f32 + TILE_SZ as f32 + 10.0], 0, (TILE_SZ/2) as f32);
        }
    }  

    fn simulate(&mut self, world: &mut World, input: &Input, _dt: f32) {
        if world.paused {
            self.stopwatch.stop();
        }
        if !world.paused && !self.stopwatch.is_running(){
            self.stopwatch.start();
        }
        if input.is_key_pressed(Key::Escape) {
            world.paused = !world.paused;
        }
        if input.is_key_pressed(Key::ShiftLeft) {
            world.paused = true;
            world.game_end = true;
        }

        let dx = input.key_axis(Key::ArrowLeft, Key::ArrowRight) * PLAYER_SPEED * DT;
        // now down means -y and up means +y!  beware!
        let dy = input.key_axis(Key::ArrowDown, Key::ArrowUp) * PLAYER_SPEED * DT;
        if dx > 0.0 {
            world.player.dir = Dir::E;
        }
        if dx < 0.0 {
            world.player.dir = Dir::W;
        }
        if dy > 0.0 {
            world.player.dir = Dir::N;
        }
        if dy < 0.0 {
            world.player.dir = Dir::S;
        }
        let dest = world.player.pos + Vec2 { x: dx, y: dy };
        if !world.level().get_tile_at(dest).unwrap().solid {
            world.player.pos = dest;
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

        let mut contacts: Vec<Contact> = Vec::new();
        let p_rect = Rect {
            x: world.player.pos.x - (TILE_SZ / 2) as f32,
            y: world.player.pos.y - (TILE_SZ / 2) as f32,
            w: (TILE_SZ) as u16,
            h: (TILE_SZ) as u16,
        };
        let player = [p_rect, Rect {x: 0.0, y: 0.0, w: 0, h: 0}];

        // Tile and Player contacts
        let mut tile_contacts = Vec::new();
        generate_tile_contact(&[player[0]], world.level(), &mut tile_contacts);

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

        // For deleting enemies, it's best to add the enemy to a "to_remove" vec, and then remove those enemies after this loop is all done.
        contacts.sort_by(|a, b| {
            b.displacement
                .mag_sq()
                .partial_cmp(&a.displacement.mag_sq())
                .unwrap()
        }); 
    }
}

impl engine::Game for MazeGame {
    fn update(&mut self, world: &mut engine::World, input: &Input) {
        self.simulate(world, input, DT);
    }
    fn render(&mut self, world: &mut engine::World, frend: &mut Immediate) {
        // make this exactly as big as we need
        frend.sprite_group_set_camera(0, world.camera);

        world.level().render_immediate(frend);

        self.draw_hud(frend);

        if world.game_end {
            self.draw_leaderboard(frend, world);
        }

        frend.draw_sprite(
            0,
            Transform {
                w: TILE_SZ as u16,
                h: TILE_SZ as u16,
                x: world.player.pos.x,
                y: world.player.pos.y,
                rot: 0.0,
            },
            PLAYER[world.player.dir as usize].with_depth(2),
        );

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
                    region: SheetRegion::rect(628, 55, 16, 16).with_depth(1),
                },
                frenderer::nineslice::Slice {
                    w: 16.0,
                    h: 16.0,
                    region: SheetRegion::rect(662, 55, 16, 16).with_depth(1),
                    repeat: frenderer::nineslice::Repeat::Tile,
                },
                frenderer::nineslice::Slice {
                    w: 16.0,
                    h: 16.0,
                    region: SheetRegion::rect(645, 55, 16, 16).with_depth(1),
                    repeat: frenderer::nineslice::Repeat::Tile,
                },
                frenderer::nineslice::Slice {
                    w: 16.0,
                    h: 16.0,
                    region: SheetRegion::rect(679, 55, 16, 16).with_depth(1),
                    repeat: frenderer::nineslice::Repeat::Tile,
                },
            );
            let pause_x = W as f32 / 2.0 - 4.0 * TILE_SZ as f32;
            let pause_y = H as f32 / 2.0 - 3.0 * TILE_SZ as f32;
            frend.draw_nineslice(
                1,
                &nine_tiled,
                pause_x,
                pause_y,
                8.0 * TILE_SZ as f32,
                6.0 * TILE_SZ as f32,
                0,
            );

            let font = frenderer::bitfont::BitFont::with_sheet_region(
                ' '..='ÿ',
                SheetRegion::new(0, 0, 143, 0, 288, 70).with_depth(0),
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
                    (H / 2) as f32 + TILE_SZ as f32,
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
                    (H / 2) as f32 - TILE_SZ as f32,
                ],
                0,
                (TILE_SZ / 2) as f32,
            );
        }
    }
    fn new(renderer: &mut Immediate, cache: AssetCache, world: &mut engine::World) -> Self {
        let tile_handle = cache
            .load::<Png>("texture")
            .expect("Couldn't load tilesheet img");
        let tile_img = tile_handle.read().0.to_rgba8();
        let tile_tex = renderer.create_array_texture(
            &[&tile_img],
            wgpu::TextureFormat::Rgba8UnormSrgb,
            tile_img.dimensions(),
            Some("tiles-sprites"),
        );


        let levels = vec![
            Level::from_str(
                &cache
                    .load::<String>("maze0")
                    .expect("Couldn't access maze0.txt")
                    .read(),
                0,
                0,
            ),
        ];
        let current_level = 0;
        let camera = Camera2D {
            screen_pos: [0.0, 0.0],
            screen_size: [W as f32, H as f32],
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
        MazeGame::new(world)
    }
}


pub fn get_user_input() -> String {
    let mut input = String::new();

    io::stdin()
        .read_line(&mut input)
        .expect("failed to read user input");

    input.trim().to_string()
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