pub mod geom;
pub mod level;
pub mod grid;
use std::vec;

use rand::Rng;
use geom::Vec2;
use geom::Rect;
use level::Level;
use assets_manager::AssetCache;
use frenderer::{
    input::Input, sprites::Camera2D, Immediate
};
const DT: f32 = 1.0 / 60.0;
const TILE_SZ: usize = 16;

#[derive(Clone, Debug)]
pub struct Contact {
    pub displacement: Vec2,
    pub a_index: usize,
    pub _a_rect: Rect,
    pub b_index: usize,
    pub b_rect: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Dir {
    N,
    E,
    S,
    W,
}

impl Dir {
    pub fn to_vec2(self) -> Vec2 {
        match self {
            Dir::N => Vec2 { x: 0.0, y: 1.0 },
            Dir::E => Vec2 { x: 1.0, y: 0.0 },
            Dir::S => Vec2 { x: 0.0, y: -1.0 },
            Dir::W => Vec2 { x: -1.0, y: 0.0 },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pos {
    pub pos: Vec2,
    pub dir: Dir,
}
pub struct World {
    pub camera: Camera2D,
    pub current_level: usize,
    pub levels: Vec<Level>,
    pub enemies: Vec<(Pos, usize)>,
    pub player: Pos,
    pub paused: bool,
    pub game_end: bool,
}

impl World {
    pub fn level(&self) -> &Level {
        &self.levels[self.current_level]
    }
    pub fn enter_level(&mut self, player_pos: Vec2) {
        self.enemies.clear();
        self.player.pos = player_pos;
        for (etype, pos) in self.levels[self.current_level].starts().iter() {
            if etype.name() == "enemy" { self.enemies.push((Pos {
                pos: *pos,
                dir: Dir::S,
            }, 1)) };
        }
    }
    pub fn spawn_enemies(&mut self) {
        // if self.paused || self.game_end { // stop generating enemies when paused/game ends
        //     return;
        // } // we can probably deal with this condition in game not engine

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
    pub fn set_camera(&mut self, camera: Camera2D) {
        self.camera = camera;
    }
    pub fn set_levels(&mut self, levels: Vec<Level>) {
        self.levels = levels;
    }
    pub fn set_current_level(&mut self, level: usize) {
        self.current_level = level;
    }
    pub fn set_enemies(&mut self, enemies: Vec<(Pos, usize)>) {
        self.enemies = enemies;
    }
    pub fn set_player(&mut self, player: Pos) {
        self.player = player;
    }
    pub fn pause(&mut self) {
        self.paused = !self.paused;
    }
    pub fn game_over(&mut self) {
        self.game_end = true;
    }
}

pub trait Game {
    fn update(&mut self, world: &mut World, input: &Input);
    fn render(&mut self, world: &World, frend: &mut Immediate);
    fn new(renderer: &mut Immediate, cache: AssetCache, world: &mut World) -> Self;
}


pub fn main_loop<G:Game> (cache: AssetCache) where G: Game + 'static  {
    let drv = frenderer::Driver::new(
        winit::window::WindowBuilder::new()
            .with_title("test")
            .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0)),
        Some((220, 140)),
    );

    let mut input = Input::default();

    let mut now = frenderer::clock::Instant::now();
    let mut acc = 0.0;
    drv.run_event_loop::<(), _>(
        move |window, frend| {
            let mut frend = Immediate::new(frend);
            let mut world = World {
                camera: Camera2D {
                    screen_pos: [0.0, 0.0],
                    screen_size: [220.0, 140.0],
                },
                current_level: 0,
                levels: vec![],
                enemies: vec![],
                player: Pos {
                    pos: Vec2 {x: 0.0, y: 0.0},
                    dir: Dir::S,
                },
                paused: false,
                game_end: false,
            };
            let game = G::new(&mut frend, cache, &mut world);
            (window, game, world, frend)
        },
        move |event, target, (window, ref mut game, ref mut world, ref mut frend)| {
            use winit::event::{Event, WindowEvent};
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    target.exit();
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    if !frend.gpu().is_web() {
                        frend.resize_surface(size.width, size.height);
                    }
                    window.request_redraw();
                }
                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => {
                    let elapsed = now.elapsed().as_secs_f32();
                    // You can add the time snapping/death spiral prevention stuff here if you want.
                    // I'm not using it here to keep the starter code small.
                    acc += elapsed;
                    now = std::time::Instant::now();
                    // While we have time to spend
                    while acc >= DT {
                        // simulate a frame
                        acc -= DT;
                        game.update(world, &input);
                        input.next_frame();
                    }
                    game.render(world, frend);
                    frend.render();
                    window.request_redraw();
                }
                event => {
                    input.process_input_event(&event);
                }
            }
        },
    )
    .expect("event loop error");
    
}
