pub mod geom;
pub mod level;
pub mod grid;
use geom::Vec2;
use geom::Rect;
use level::Level;
use assets_manager::{asset::Png, AssetCache};
use frenderer::{
    input::{Input, Key}, sprites::{Camera2D, SheetRegion, Transform}, wgpu, Immediate
};
const DT: f32 = 1.0 / 60.0;

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
}

pub trait Game {
    fn update(&mut self, world: &mut World);
    fn new(renderer: &mut Immediate, cache: &AssetCache, world: &mut World);
}

pub fn main_loop(frend: &mut Immediate, cache: &AssetCache) {
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
    let player_start = *levels[current_level]
            .starts()
            .iter()
            .find(|(t, _)| t.name() == "player")
            .map(|(_, ploc)| ploc)
            .expect("Start level doesn't put the player anywhere");
    let mut world = World {
        camera: Camera2D {
            screen_pos: [0.0, 0.0],
            screen_size: [220 as f32, 140 as f32],
        },
        current_level,
        levels,
        enemies: vec![],
        player: Pos {
            pos: player_start,
            dir: Dir::S,
        },
    };

    let drv = frenderer::Driver::new(
        winit::window::WindowBuilder::new()
            .with_title("test")
            .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0)),
        Some((220 as u32, 140 as u32)),
    );

    let mut input = Input::default();

    let mut now = frenderer::clock::Instant::now();
    let mut acc = 0.0;
    drv.run_event_loop::<(), _>(
        move |window, frend| {
            let mut frend = Immediate::new(frend);
            let game = Game::new(&mut frend, &cache, &mut world);
            (window, game, frend)
        },
        move |event, target, (window, ref mut game, ref mut frend)| {
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
                        game.update(&world);
                        input.next_frame();
                    }
                    game.render(frend);
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
    //to here
}
