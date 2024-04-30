# super-cool-games

This is our final project for CSCI181G!

We have developed a **game engine** and three standalone games that can be run on our engine!

## engine

This engine was not super game specific, it supports basic level rendering, player and enemy movement, collision detection, menu rendering, etc. We chose to keep this engine pretty all-purpose as we wanted to see just how much work goes into building out an engine that can support really different kinds of games (like two of the ones we ended up developing)


## adventure game

In this game, the player fights randomly spawning enemies in order to gain XP and level up! On each level up (achieved after killing 5 enemies with an AOE attack), the player is given a choice between increasing their health or their attack radius. This game was a fun exploration of how to deal with different sprite groups (tiles, menus, etc.) and also a look into how some of our favorite game features can actually be implemented! 

## maze game

This game is a classic maze speedrunner game. With a leaderboard and a running stopwatch (that is visible to the player), we aim to push the player to go faster and faster each time they play! We had a lot of fun working on building the maze map for this game and really making sure the time pressure is felt by including the stopwatch in the HUD. This was also a cool opportunity to look at user keyboard inpute to allow the player to customize their leaderboard spot.

## simulation game

Finally, our simulation game was an experimentation with enemy AI. This game is a sort of "choose your own difficulty" experience as the player is the one spawning the enemies. Once enemies are spawned, they will slowly start moving towards the player based on the player's current position. While this game was a bit more difficult to execute than the others, we are excited to add more features in the future and think that the realm of enemy pathfinding has many interesting tools for us to explore.
