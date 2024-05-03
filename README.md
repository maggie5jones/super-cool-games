# super-cool-games

This is our final project for CSCI181G!

We have developed a **game engine** and three standalone games that can be run on our engine!

## engine

This engine was not super game specific, it supports basic level rendering, player and enemy movement, collision detection, menu rendering, etc. We chose to keep this engine pretty all-purpose as we wanted to see just how much work goes into building out an engine that can support really different kinds of games (like two of the ones we ended up developing)

Delving a little deeper, let's look at the different `mod`'s or engine uses!

`level.rs`

This level rendering file reads in a text file with different symbols and other indicators to allow the level to be rendered using different sheet regions in different positions

`grid.rs`

This code helps us deal with collisions in that it makes drawing and dealing with sprite rects much easier since the whole game world is built into this iterable grid. 

`geom.rs`

This file contains all of our engine's support for rectangle and vector geometries. It supports certain simple geometric/linear algebraic calculations allowing for less repetetive code in the main engine file. 

### engine features

Our engine supports the creation of a `World` that holds a lot of the metadata that is needed for any game. 

```rust
pub struct World {
    pub camera: Camera2D,
    pub current_level: usize,
    pub levels: Vec<Level>,
    pub enemies: Vec<(Pos, usize)>,
    pub player: Pos,
    pub paused: bool,
    pub game_end: bool,
}
```

Some of the functionalty of a `World` includes spawning enemies, loading new levels, and running the main loop that actually causes a game to be instantiated.

We also have a `Game` trait in our engine that will allow each new game type to extend the engine and utilize all of its helpful functions.

```rust
pub trait Game {
    fn update(&mut self, world: &mut World, input: &Input);
    fn render(&mut self, world: &mut World, frend: &mut Immediate);
    fn new(renderer: &mut Immediate, cache: AssetCache, world: &mut World) -> Self;
}
```

## adventure game

In this game, the player fights randomly spawning enemies in order to gain XP and level up! On each level up (achieved after killing 5 enemies with an AOE attack), the player is given a choice between increasing their health or their attack radius. This game was a fun exploration of how to deal with different sprite groups (tiles, menus, etc.) and also a look into how some of our favorite game features can actually be implemented! 

This game extends the `Game` trait from the engine, and thus has the required `update`, `render`, and `new` functions. The `update` function is supported by our `simulate` function which takes in user keyboard input and updates the game world accordingly. This function also supports contact resolution between the player, enemies, and solid tiles (all the contacts between each individual one). Finally, this function also removes enemies when they leave the map or the player kills an enemy.

## maze game

This game is a classic maze speedrunner game. With a leaderboard and a running stopwatch (that is visible to the player), we aim to push the player to go faster and faster each time they play! We had a lot of fun working on building the maze map for this game and really making sure the time pressure is felt by including the stopwatch in the HUD. This was also a cool opportunity to look at user keyboard inpute to allow the player to customize their leaderboard spot.

This game also extends the `Game` trait from the engine, and thus has the required `update`, `render`, and `new` functions. The `update` function is supported by our `simulate` function which takes in user keyboard input and updates the player position accordingly. This function also supports contact resolution between the player and solid tiles. This function also supports the leaderboard being drawn with the stopwatch we simulate to encourage the player to get through the maze as quick as possible.

## simulation game

Finally, our simulation game was an experimentation with enemy AI. This game is a sort of simulation experience as the player is the one spawning the enemies and the knights to fight those eneimes. Once enemies are spawned, they will move completely at random around the map. However knights (when spawned) will chase down the nearest enemy and kill them on contact. After three enemies are killed by a knight, the knight will then despawn. While this game was a bit more difficult to execute than the others, we are excited to add more features in the future and think that the realm of different enemy types and pathfinding has many interesting tools for us to explore.

This game also extends the `Game` trait from the engine, and thus has the required `update`, `render`, and `new` functions. The `update` function is supported by our `simulate` function which takes in user keyboard input and updates the player position accordingly. This function also supports contact resolution between the player and solid tiles. This function is unique as it allows the player to spawn their own enemies and thus resize the local memory for enemies accordingly.
