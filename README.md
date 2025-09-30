# Budget Battle City

A classic Battle City-inspired tank combat game built with Rust and Bevy 0.16.1.

## About

This is a simplified clone of the classic Battle City arcade game featuring maze-based combat, enemy AI, and bullet mechanics. Navigate through a maze, destroy enemy tanks, and survive as long as possible!

## Features

- **Player-controlled tank** with 360-degree movement
- **Enemy AI** that seeks and shoots at the player
- **Maze-based level** with walls and spawn points
- **Collision detection** with smooth sliding against walls
- **Bullet mechanics** for both player and enemies
- **Dynamic enemy spawning** with configurable spawn rate and cap
- **Restart system** when the player is hit

## Controls

- **Movement**: `W/A/S/D` or Arrow Keys
- **Fire**: `Space`

## Requirements

- Rust (latest stable version recommended)
- Cargo

## Building and Running

```bash
# Clone the repository
git clone <repository-url>
cd BudgetBattleCity

# Run in development mode
cargo run

# Run in release mode (better performance)
cargo run --release
```