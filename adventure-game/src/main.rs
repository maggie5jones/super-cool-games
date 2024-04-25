use assets_manager::{asset::Png, AssetCache};
use frenderer::{
    input::{Input, Key}, sprites::{Camera2D, SheetRegion, Transform}, wgpu, Immediate
};
use rand::Rng;
use engine::{geom::*, World};
use engine::level::Level;

use engine::Contact;

use engine::Dir;

const PLAYER: [SheetRegion; 4] = [
    //n, e, s, w
    SheetRegion::rect(461 + 16 * 2, 39, 16, 16),
    SheetRegion::rect(461, 39, 16, 16),
    SheetRegion::rect(461 + 16 * 3, 39, 16, 16),
    SheetRegion::rect(461 + 16, 39, 16, 16),
];
const _PLAYER_ATK: [SheetRegion; 4] = [
    //n, e, s, w
    SheetRegion::rect(428, 0, 16, 8), // offset by 8px in direction
    SheetRegion::rect(349, 22, 8, 16),
    SheetRegion::rect(162, 13, 16, 8),
    SheetRegion::rect(549, 17, 8, 16),
];
const ENEMY: [SheetRegion; 4] = [
    SheetRegion::rect(533 + 16 * 2, 39, 16, 16),
    SheetRegion::rect(533 + 16, 39, 16, 16),
    SheetRegion::rect(533, 39, 16, 16),
    SheetRegion::rect(533 + 16 * 3, 39, 16, 16),
];
const HEART: SheetRegion = SheetRegion::rect(525, 35, 8, 8);
const EXPERIENCE: SheetRegion = SheetRegion::rect(525, 50, 8, 8);

const ATK: SheetRegion = SheetRegion::rect(525, 19, 8, 8);
const BLANK: SheetRegion = SheetRegion::rect(600, 600, 16, 16);

use engine::Pos;

// Feel free to change this if you use a different tilesheet
const TILE_SZ: usize = 16;
const W: usize = 220; // 320
const H: usize = 140; // 240
const SCREEN_FAST_MARGIN: f32 = 64.0;

// pixels per second
const PLAYER_SPEED: f32 = 64.0;
const ENEMY_SPEED: f32 = 32.0;
const _KNOCKBACK_SPEED: f32 = 128.0;

const ATTACK_MAX_TIME: f32 = 0.3;
const ATTACK_COOLDOWN_TIME: f32 = 0.1;
const KNOCKBACK_TIME: f32 = 1.0;

const DT: f32 = 1.0 / 60.0;

const LEVELUP: u8 = 5;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let source =
        assets_manager::source::FileSystem::new("content").expect("Couldn't load resources");
    #[cfg(target_arch = "wasm32")]
    let source = assets_manager::source::Embedded::from(assets_manager::source::embed!("content"));
    let cache = assets_manager::AssetCache::with_source(source);

    // from here
    
}
struct Game {
    pub attack_area: Rect,
    pub attack_range: f32,
    pub attack_timer: f32,
    pub knockback_timer: f32,
    pub health: u8,
    pub xp: u8,
    pub paused: bool,
    pub game_end: bool,
    pub upgrade: bool,
}

impl Game {
    fn new(world: &mut World, renderer: &mut Immediate, cache: &AssetCache) -> Self {
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
        
        let mut game = Game {
            attack_area: Rect {
                x: 0.0,
                y: 0.0,
                w: 0,
                h: 0,
            },
            knockback_timer: 0.0,
            attack_timer: 0.0,
            attack_range: 3.0,
            health: 3,
            xp: 0,
            paused: false,
            game_end: false,
            upgrade: false,
        };
        game.enter_level(player_start);
        game
    }
    fn level(&self) -> &Level {
        &self.levels[self.current_level]
    }
    fn enter_level(&mut self, player_pos: Vec2) {
        self.enemies.clear();
        self.player.pos = player_pos;
        for (etype, pos) in self.levels[self.current_level].starts().iter() {
            if etype.name() == "enemy" { self.enemies.push((Pos {
                pos: *pos,
                dir: Dir::S,
            }, 1)) };
        }
    }
    fn spawn_enemies(&mut self) {
        if self.paused || self.game_end { // stop generating enemies when paused/game ends
            return;
        }

        let mut rng = rand::thread_rng();
        let rand = rng.gen_range(0..1000);
        if rand > 960 {
            let mut randx = rng.gen_range(2..self.levels[self.current_level].width()*TILE_SZ);
            let mut randy = rng.gen_range(2..self.levels[self.current_level].height()*TILE_SZ);
            while ((randx as f32 - self.player.pos.x).abs() < 48.0) && ((randy as f32 - self.player.pos.y).abs() < 48.0)
            && !self.level().get_tile_at(Vec2{x:randx as f32, y:randy as f32}).unwrap().solid  {
                randx = rng.gen_range(2..self.levels[self.current_level].width()*TILE_SZ);
                randy = rng.gen_range(2..self.levels[self.current_level].height()*TILE_SZ);
            } 
            let monster = Pos {
                pos: Vec2{x: randx as f32, y: randy as f32},
                dir: Dir::S,
            };
            self.enemies.push((monster, 1));
        }
    }
    fn draw_hud(&self, frend: &mut Immediate) {
        // render an upgrade menu
        
        // draw UI with health and experience
        let heart_pos = Transform {
            w: TILE_SZ as u16, 
            h: TILE_SZ as u16,
            x: 10.0,
            y: 6.0,
            rot: 0.0,
        };
        for i in 0..self.health {
            // sprite_posns[k] = Transform {
            //     x: heart_pos.x + i as f32 * TILE_SZ as f32,
            //     ..heart_pos
            // };
            // sprite_gfx[k] = HEART.with_depth(0);
            frend.draw_sprite(1, Transform {
                x: heart_pos.x + i as f32 * TILE_SZ as f32,
                ..heart_pos
            }, HEART.with_depth(1));
        }
        let exp_pos = Transform {
            w: TILE_SZ as u16, 
            h: TILE_SZ as u16,
            x: W as f32 - 10.0,
            y: H as f32 - 10.0,
            rot: 0.0,
        };
        for i in 0..self.xp {
            // let l = i as usize + 3 + self.health as usize;
            // sprite_posns[l] = Transform {
            //     x: exp_pos.x - i as f32 * 12 as f32,
            //     ..exp_pos
            // };
            // sprite_gfx[l] = EXPERIENCE.with_depth(0);

            frend.draw_sprite(1, Transform {
                x: exp_pos.x - i as f32 * 12_f32,
                ..exp_pos
            }, EXPERIENCE.with_depth(1));
        }
        
        // render a game end menu
        // if self.game_end {
        //     let heart_pos = Transform {
        //         w: (TILE_SZ) as u16, 
        //         h: (TILE_SZ) as u16,
        //         x: self.camera.screen_pos[0] + 10 as f32,
        //         y: self.camera.screen_pos[1] + self.camera.screen_size[1] - 10 as f32, 
        //         rot: 0.0,
        //     };
        //     let j = 2 + self.health as usize;
        //     sprite_posns[j] = Transform {
        //         x: heart_pos.x as f32 * (TILE_SZ) as f32,
        //         ..heart_pos
        //     };
        //     sprite_gfx[j] = HEART.with_depth(0);
        // }
    }
    fn render(&mut self, frend: &mut Immediate) {
        // make this exactly as big as we need
        frend.sprite_group_set_camera(0, self.camera);

        self.level().render_immediate(frend);
        // println!("sprites used: {}", sprites_used);
        
        for enemy in self.enemies.iter()
        {
            if enemy.1 == 1 {
                frend.draw_sprite(0, Transform {
                    w: TILE_SZ as u16,
                    h: TILE_SZ as u16,
                    x: enemy.0.pos.x,
                    y: enemy.0.pos.y,
                    rot: 0.0,
                }, ENEMY[enemy.0.dir as usize].with_depth(3));
            }
            else {
                frend.draw_sprite(0, Transform::ZERO, SheetRegion::ZERO);
            }
        }

        // draw pause menu & HUD
        self.draw_hud(frend);
        
        if self.knockback_timer > 0.0 && self.knockback_timer % 0.5 < 0.25 {
            frend.draw_sprite(0, Transform {
                w: TILE_SZ as u16,
                h: TILE_SZ as u16,
                x: self.player.pos.x,
                y: self.player.pos.y,
                rot: 0.0,
            }, SheetRegion::ZERO);
        } else{
            frend.draw_sprite(0, Transform {
                w: TILE_SZ as u16,
                h: TILE_SZ as u16,
                x: self.player.pos.x,
                y: self.player.pos.y,
                rot: 0.0,
            }, PLAYER[self.player.dir as usize].with_depth(2));
        }
        if self.game_end { // player disappears when game ends (no more health)
            frend.draw_sprite(0, Transform {
                w: TILE_SZ as u16,
                h: TILE_SZ as u16,
                x: self.player.pos.x,
                y: self.player.pos.y,
                rot: 0.0,
            }, SheetRegion::ZERO);
        }
        
        if self.attack_area.is_empty() {
            // sprite_posns[1] = Transform::ZERO;
        } else {
            let (w, h) = match self.player.dir {
                Dir::N | Dir::S => (16, 8),
                _ => (8, 16),
            };
            let delta = self.player.dir.to_vec2() * 7.0;
            // sprite_posns[1] = Transform {
            //     w,
            //     h,
            //     x: self.player.pos.x + delta.x,
            //     y: self.player.pos.y + delta.y,
            //     rot: 0.0,
            // };
            // sprite_posns[self.health as usize + 2] = Transform {
            //     w: (self.attack_range as usize * TILE_SZ) as u16,
            //     h: (self.attack_range as usize * TILE_SZ) as u16,
            //     x: self.player.pos.x,
            //     y: self.player.pos.y,
            //     rot: 0.0,
            // };
            frend.draw_sprite(0, Transform {
                w,
                h,
                x: self.player.pos.x + delta.x,
                y: self.player.pos.y + delta.y,
                rot: 0.0,
            }, BLANK.with_depth(2));

            frend.draw_sprite(0, Transform {
                w: (self.attack_range as usize * TILE_SZ) as u16,
                h: (self.attack_range as usize * TILE_SZ) as u16,
                x: self.player.pos.x,
                y: self.player.pos.y,
                rot: 0.0,
            }, ATK.with_depth(2));    
            
        }
        // sprite_gfx[1] = BLANK.with_depth(0); //was player_atk[dir]
        // sprite_gfx[self.health as usize + 2] = ATK.with_depth(0); // commenting this out made menu show up consistently

        if self.paused {
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
            frend.draw_nineslice(1, &nine_tiled, pause_x, pause_y, 8.0*TILE_SZ as f32, 6.0*TILE_SZ as f32, 0);  

            let font = frenderer::bitfont::BitFont::with_sheet_region(
                ' '..='ÿ', 
                SheetRegion::new(0, 0, 143, 0, 288, 70).with_depth(0), 
                (8) as u16, 
                (8) as u16, 
                (1) as u16, 
                (2) as u16);
            
            if self.upgrade{
                let mut text = "upgrades time";
                frend.draw_text(1, &font, text, [(W/2) as f32 - 3.5*TILE_SZ as f32, (H/2) as f32 + TILE_SZ as f32], 0, (TILE_SZ/2) as f32);
                text = ":Q"; //TODO: update the positions of these and add heart for health upgrade and sword for attack radius upgrade
                frend.draw_text(1, &font, text, [(W/2) as f32 - 3.5*TILE_SZ as f32, (H/2) as f32 + TILE_SZ as f32], 0, (TILE_SZ/2) as f32);
                text = ":E";
                frend.draw_text(1, &font, text, [(W/2) as f32 - 3.5*TILE_SZ as f32, (H/2) as f32 + TILE_SZ as f32], 0, (TILE_SZ/2) as f32);
            } else {
                let mut text = "game paused!";
                frend.draw_text(1, &font, text, [(W/2) as f32 - 3.0*TILE_SZ as f32, (H/2) as f32 + TILE_SZ as f32], 0, (TILE_SZ/2) as f32);
                text = "unpause: Esc";
                frend.draw_text(1, &font, text, [(W/2) as f32 - 3.25*TILE_SZ as f32, (H/2) as f32 - TILE_SZ as f32], 0, (TILE_SZ/2) as f32);

            }
        }
        // done point: draw hearts
        // let heart_pos = Transform {
        //     w: (TILE_SZ/2) as u16, 
        //     h: (TILE_SZ/2) as u16,
        //     x: self.player.pos.x - (TILE_SZ/2) as f32,
        //     y: self.player.pos.y + TILE_SZ as f32, 
        //     rot: 0.0,
        // };
        // for i in 0..self.health {
        //     let j = i as usize + 2;
        //     sprite_posns[j] = Transform {
        //         x: heart_pos.x + i as f32 * (TILE_SZ/2) as f32 - 0.5,
        //         ..heart_pos
        //     };
        //     sprite_gfx[j] = HEART.with_depth(0);
        // }
    }
    fn simulate(&mut self, input: &Input, dt: f32) {
        self.spawn_enemies();
        
        if self.xp == LEVELUP { // upgrade menu showing up is a bit buggy but seems to work :)
            self.upgrade = true;
            self.paused = true;
            self.xp = 0;
        }
        if self.upgrade {
            if input.is_key_pressed(Key::KeyQ) {
                self.paused = false;
                self.upgrade = false;
                self.health += 2;
            }
            if input.is_key_pressed(Key::KeyE) {
                self.paused = false;
                self.upgrade = false;
                self.attack_range += 0.5;
            }
        }
        if input.is_key_pressed(Key::Escape) {
            self.paused = !self.paused;
        }
        if self.paused || self.game_end { // stop generating enemies when paused/game ends
            // self.simulate_pause(input, dt)
            
            return;
        }
        
        if self.attack_timer > 0.0 {
            self.attack_timer -= dt;
        }
        if self.knockback_timer > 0.0 {
            self.knockback_timer -= dt;
        }
        let dx = input.key_axis(Key::ArrowLeft, Key::ArrowRight) * PLAYER_SPEED * DT;
        // now down means -y and up means +y!  beware!
        let dy = input.key_axis(Key::ArrowDown, Key::ArrowUp) * PLAYER_SPEED * DT;
        let attacking = !self.attack_area.is_empty();
        let _knockback = self.knockback_timer > 0.0;
        if attacking {
            // while attacking we can't move
            // dx = 0.0;
            // dy = 0.0;
        } else {
            // not attacking, no knockback, do normal movement
            if dx > 0.0 {
                self.player.dir = Dir::E;
            }
            if dx < 0.0 {
                self.player.dir = Dir::W;
            }
            if dy > 0.0 {
                self.player.dir = Dir::N;
            }
            if dy < 0.0 {
                self.player.dir = Dir::S;
            }
        }
        if self.attack_timer <= 0.0 && input.is_key_pressed(Key::Space) {
            // compute the attack area's center based on the player's position and facing and some offset
            // For the spritesheet provided, the attack is placed 8px "forwards" from the player.
            self.attack_timer = ATTACK_MAX_TIME;
            self.attack_area = Rect {
                x: self.player.pos.x - (TILE_SZ as f32*(self.attack_range/2.0)),
                y: self.player.pos.y - (TILE_SZ as f32*(self.attack_range/2.0)),
                w: self.attack_range as u16 *TILE_SZ as u16,
                h: self.attack_range as u16 *TILE_SZ as u16,
            }
        } else if self.attack_timer <= ATTACK_COOLDOWN_TIME {
            // "turn off" the attack, but the cooldown is still going
            self.attack_area = Rect {
                x: 0.0,
                y: 0.0,
                w: 0,
                h: 0,
            };
        }
        // self.player.pos += Vec2 { x: dx, y: dy };
        let dest = self.player.pos + Vec2 { x: dx, y: dy };
        if !self.level().get_tile_at(dest).unwrap().solid {
            self.player.pos = dest;
        }
        let mut rng = rand::thread_rng();
        for enemy in self.enemies.iter_mut() {
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
            if (enemy_dest.x >= 0.0 && enemy_dest.x <= (self.levels[self.current_level].width()*TILE_SZ) as f32)
             && (enemy_dest.y > 0.0 && enemy_dest.y <= (self.levels[self.current_level].height()*TILE_SZ) as f32) 
            {
                enemy.0.pos = enemy_dest;
            }
        }

        let lw = self.level().width();
        let lh = self.level().height();

        while self.player.pos.x
            > self.camera.screen_pos[0] + self.camera.screen_size[0] - SCREEN_FAST_MARGIN
        {
            self.camera.screen_pos[0] += 1.0;
        }
        while self.player.pos.x < self.camera.screen_pos[0] + SCREEN_FAST_MARGIN {
            self.camera.screen_pos[0] -= 1.0;
        }
        while self.player.pos.y
            > self.camera.screen_pos[1] + self.camera.screen_size[1] - SCREEN_FAST_MARGIN
        {
            self.camera.screen_pos[1] += 1.0;
        }
        while self.player.pos.y < self.camera.screen_pos[1] + SCREEN_FAST_MARGIN {
            self.camera.screen_pos[1] -= 1.0;
        }
        self.camera.screen_pos[0] =
            self.camera.screen_pos[0].clamp(0.0, (lw * TILE_SZ).max(W) as f32 - W as f32);
        self.camera.screen_pos[1] =
            self.camera.screen_pos[1].clamp(0.0, (lh * TILE_SZ).max(H) as f32 - H as f32);

        // implement collision detection here. 
        // for collision with the tilemap, you can use Level::tiles_within to find the tiles touching a rectangle, 
        // and filter out the ones that are not solid.  Then you have rects you can test against your player/enemies.
        // let player_rect = Rect {
        //     x: self.player.pos.x,
        //     y: self.player.pos.y,
        //     w: TILE_SZ as u16,
        //     h: TILE_SZ as u16,
        // };

        // Check for collision with enemies
        // let enemy_rects = self
        //     .enemies
        //     .iter()
        //     .filter(|enemy| enemy.1 > 0)
        //     .map(|enemy| Rect {
        //         x: enemy.0.pos.x,
        //         y: enemy.0.pos.y,
        //         w: TILE_SZ as u16,
        //         h: TILE_SZ as u16,
        //     })
        //     .collect::<Vec<_>>();

        let mut contacts = Vec::new();
        let p_rect = Rect {
            x: self.player.pos.x - (TILE_SZ / 2) as f32,
            y: self.player.pos.y - (TILE_SZ / 2) as f32,
            w: (TILE_SZ) as u16,
            h: (TILE_SZ) as u16,
        };
        let player = [p_rect, self.attack_area];
        let enemy_rect: Vec<_> = self.enemies.iter().map(|e| make_rect(e.0.pos)).collect();
        generate_contact(&player, &enemy_rect, &mut contacts);
        // dbg!(contacts.len());

        // Tile and Player contacts
        let mut tile_contacts = Vec::new();
        generate_tile_contact(&[player[0]], self.level(), &mut tile_contacts);
        // dbg!(tile_contacts.len());

        // Tile and Enemy contacts
        let mut tile_enemy_contacts = Vec::new();
        generate_tile_contact(&enemy_rect, self.level(), &mut tile_enemy_contacts);
        // dbg!(tile_enemy_contacts.len());

        // TODO POINT: damage/destroy the enemy

        // I suggest gathering player-tile collisions and enemy-tile collisions, then doing collision response on those sets of contacts (it's OK to use two sets of contacts).
        // You could have helper functions like gather_contacts_tiles(&[rect], &Level, &mut Vec<Contact>) and gather_contacts(&[rect], &[rect], &mut Vec<Contact>) or do_collision_response(&[Contact], &mut [rect], &[rect]) or compute_displacement(rect, rect) -> Vec2.
        // A Contact struct is not strictly necessary but it's a good idea (with fields like displacement, a_index, a_rect, b_index, and b_rect fields).
        // Then, you can check for contacts between the player & their attack rectangle on one side, and the enemies on the other side (you can reuse gather_contacts for this).  These don't need to participate in collision response, but you can use them to determine whether the player or enemy should be damaged.

        // Contact Resolution for player vs. world
        tile_contacts.sort_by(|a, b| {
            b.displacement
                .mag_sq()
                .partial_cmp(&a.displacement.mag_sq())
                .unwrap()
        });
        for contact in tile_contacts {
            self.player.pos += find_displacement(p_rect, contact.b_rect);
        }

        // Contact Resolution for enemies vs. world
        tile_enemy_contacts.sort_by(|a, b| {
            b.displacement
                .mag_sq()
                .partial_cmp(&a.displacement.mag_sq())
                .unwrap()
        });
        for contact in tile_enemy_contacts {
            self.enemies[contact.a_index].0.pos +=
                find_displacement(enemy_rect[contact.a_index], contact.b_rect);
        }

        // For deleting enemies, it's best to add the enemy to a "to_remove" vec, and then remove those enemies after this loop is all done.
        contacts.sort_by(|a, b| {
            b.displacement
                .mag_sq()
                .partial_cmp(&a.displacement.mag_sq())
                .unwrap()
        });

        let mut removable = Vec::new();
        for contact in contacts {
            if contact.a_index == 1 && !removable.contains(&contact.b_index){
                self.enemies[contact.b_index].0.pos +=
                    find_displacement(p_rect, enemy_rect[contact.b_index]);
                removable.push(contact.b_index);
                self.xp += 1; // this might be wrong as it gives xp when an enemy dies in a wall
                // dbg!(self.xp);
            }
            if contact.a_index == 0 {
                if self.knockback_timer == 0.0 {
                    self.knockback_timer = KNOCKBACK_TIME;
                    self.health -= 1;
                    if self.health == 0 {
                        // panic!("Game Over!");
                        self.game_end = true;
                    }
                } else if self.knockback_timer < KNOCKBACK_TIME {
                    // "turn off" the ability to get hit
                    if self.knockback_timer.abs() < 0.02 {
                        self.knockback_timer = 0.0;
                    }
                }
            }
        }
        // Alternatively, you could "disable" an enemy by giving it an `alive` flag or similar and setting that to false, not drawing or updating dead enemies.
        removable.sort();
        for i in removable.iter().rev() {
            self.enemies.swap_remove(*i);
        }
    }

}

impl engine::Game for Game {
    fn update(&mut self, world:&mut engine::World) {
        
    }
    fn new(renderer: &mut Immediate, cache: &AssetCache, world:&mut engine::World) {
        let levels = vec![
            Level::from_str(
                &cache
                    .load::<String>("level3")
                    .expect("Couldn't access level3.txt")
                    .read(),
                    0,
                    0,
            ),
            Level::from_str(
                &cache
                    .load::<String>("level1")
                    .expect("Couldn't access level1.txt")
                    .read(),
                    0,
                    0,
            ),
            Level::from_str(
                &cache
                    .load::<String>("level2")
                    .expect("Couldn't access level2.txt")
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
        renderer.sprite_group_add(
            &tile_tex,
            sprite_estimate,
            //vec![Transform::ZERO; sprite_estimate],
            //vec![SheetRegion::ZERO; sprite_estimate],
            camera,
        );
        // HUD sprite group: 1
        renderer.sprite_group_add(
            &tile_tex,
            sprite_estimate,
            //vec![Transform::ZERO; sprite_estimate],
            //svec![SheetRegion::ZERO; sprite_estimate],
            camera,
        );
        let player_start = *levels[current_level]
            .starts()
            .iter()
            .find(|(t, _)| t.name() == "player")
            .map(|(_, ploc)| ploc)
            .expect("Start level doesn't put the player anywhere");
        world = &mut World {
            camera,
            current_level,
            levels,
            enemies: vec![],
            player: Pos {
                pos: player_start,
                dir: Dir::S,
            },
        }
    }
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
