//! Battle Tanks (Bevy 0.16.1): maze, enemies, collisions — crash-fixed

use bevy::prelude::*;
use std::time::Duration;

// === Arena & tiles ===
const ARENA_W: f32 = 800.0;
const ARENA_H: f32 = 600.0;
const TILE: f32 = 40.0; // 20x15 grid

// === Player ===
const PLAYER_SPEED: f32 = 300.0;
const PLAYER_SIZE: Vec2 = Vec2::new(32.0, 32.0);

// === Bullets ===
const BULLET_SPEED: f32 = 600.0;
const BULLET_SIZE: Vec2 = Vec2::new(6.0, 12.0);

// === Enemies ===
const ENEMY_SPEED: f32 = 180.0;
const ENEMY_SIZE: Vec2 = Vec2::new(28.0, 28.0);
const ENEMY_CAP: usize = 24;
const ENEMY_SPAWN_SECS: f32 = 1.25;

// === Components ===
#[derive(Component)] struct Player;
#[derive(Component)] struct Bullet;
#[derive(Component)] struct Enemy;
#[derive(Component)] struct Wall;

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Component)]
struct Size(Vec2);

// === Resources ===
#[derive(Resource)]
struct FireCooldown(Timer);

#[derive(Resource)]
struct EnemySpawnTimer(Timer);

#[derive(Resource, Debug)]
struct SpawnPoints {
    points: Vec<Vec2>,
    next: usize,
}

#[derive(Resource)]
struct PlayerStart(Vec2);

// 20x15 maze: exactly 20 chars per row
// '#' = wall, 'S' = enemy spawn, 'P' = player start, ' ' = floor
const MAZE: [&str; 15] = [
    "####################",
    "#P             #  S#",
    "### #### ####### ###",
    "#   #   #     #   ##",
    "# ### # # ### ###  #",
    "# #   #   # #     S#",
    "#         # # ######", // <- fixed (20 chars)
    "# #     #   #     ##",
    "# ##### ###     #  #",
    "#     #     #   #  #",
    "### # ### # ### ####",
    "# S #   # #   #    #",
    "### ### # ### # ####",
    "#      S#     #   S#",
    "####################",
];


fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Battle Tanks (Bevy 0.16.1)".into(),
                resolution: (ARENA_W, ARENA_H).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(FireCooldown(Timer::from_seconds(0.16, TimerMode::Once)))
        .insert_resource(EnemySpawnTimer(Timer::from_seconds(
            ENEMY_SPAWN_SECS,
            TimerMode::Repeating,
        )))
        // was: .add_systems(Startup, (setup_camera, build_maze, spawn_player))
        .add_systems(Startup, (setup_camera, build_maze, spawn_player).chain())
        .add_systems(
            Update,
            (
                player_input,
                handle_fire,
                move_with_collisions,
                bullet_hits,
                bullet_wall_cull,
                enemy_ai_seek_player,
                enemy_spawner,      // now mutably advances spawn index
                clamp_to_arena,
            ),
        )
        .run();
}

// === Setup ===
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

fn build_maze(mut commands: Commands) {
    // Validate all rows are equal width (defensive)
    let expected_cols = MAZE[0].len();
    for (i, row) in MAZE.iter().enumerate() {
        assert!(
            row.len() == expected_cols,
            "MAZE row {i} width {} != {}",
            row.len(),
            expected_cols
        );
    }

    let mut spawn_points = Vec::new();
    let mut player_start = Vec2::new(0.0, -ARENA_H * 0.35); // fallback

    let origin = Vec2::new(-ARENA_W * 0.5 + TILE * 0.5, ARENA_H * 0.5 - TILE * 0.5);

    for (r, line) in MAZE.iter().enumerate() {
        for (c, ch) in line.chars().enumerate() {
            let x = origin.x + c as f32 * TILE;
            let y = origin.y - r as f32 * TILE;

            match ch {
                '#' => {
                    commands.spawn((
                        Sprite {
                            color: Color::srgb(0.25, 0.25, 0.3),
                            custom_size: Some(Vec2::splat(TILE)),
                            ..default()
                        },
                        Transform::from_xyz(x, y, 0.0),
                        Wall,
                        Size(Vec2::splat(TILE)),
                    ));
                }
                'S' => spawn_points.push(Vec2::new(x, y)),
                'P' => player_start = Vec2::new(x, y),
                _ => {}
            }
        }
    }

    commands.insert_resource(SpawnPoints { points: spawn_points, next: 0 });
    commands.insert_resource(PlayerStart(player_start));
}

fn spawn_player(mut commands: Commands, start: Option<Res<PlayerStart>>) {
    let Some(start) = start else { return; }; // resource not ready yet
    commands.spawn((
        Sprite {
            color: Color::srgb(0.2, 0.9, 0.2),
            custom_size: Some(PLAYER_SIZE),
            ..default()
        },
        Transform::from_xyz(start.0.x, start.0.y, 1.0),
        Player,
        Velocity(Vec2::ZERO),
        Size(PLAYER_SIZE),
    ));
}


// === Systems ===

fn player_input(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut q_player: Query<(&mut Transform, &mut Velocity), With<Player>>,
) {
    let Ok((mut transform, mut vel)) = q_player.get_single_mut() else { return; };

    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) || input.pressed(KeyCode::ArrowUp) { dir.y += 1.0; }
    if input.pressed(KeyCode::KeyS) || input.pressed(KeyCode::ArrowDown) { dir.y -= 1.0; }
    if input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft) { dir.x -= 1.0; }
    if input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight){ dir.x += 1.0; }

    if dir.length_squared() > 0.0 {
        dir = dir.normalize();
        let angle = dir.y.atan2(dir.x);
        transform.rotation = Quat::from_rotation_z(angle);
        **vel = dir * PLAYER_SPEED;
    } else {
        **vel = Vec2::ZERO;
    }

    // Integration in move_with_collisions()
    let _ = time;
}

fn handle_fire(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut cooldown: ResMut<FireCooldown>,
    q_player: Query<(&Transform, &Size), With<Player>>,
    mut commands: Commands,
) {
    cooldown.0.tick(time.delta());
    if !input.just_pressed(KeyCode::Space) || !cooldown.0.finished() { return; }

    let Ok((t, psize)) = q_player.get_single() else { return; };
    let forward = t.rotation.mul_vec3(Vec3::X).truncate();
    if forward.length_squared() == 0.0 { return; }

    let spawn_pos = t.translation.truncate() + forward * (psize.0.x * 0.6);

    commands.spawn((
        Sprite {
            color: Color::WHITE,
            custom_size: Some(BULLET_SIZE),
            ..default()
        },
        Transform::from_xyz(spawn_pos.x, spawn_pos.y, 0.5).with_rotation(t.rotation),
        Bullet,
        Velocity(forward * BULLET_SPEED),
        Size(BULLET_SIZE),
    ));

    cooldown.0.reset();
}

fn enemy_ai_seek_player(
    mut q_enemies: Query<(&Transform, &mut Velocity), With<Enemy>>,
    q_player: Query<&Transform, (With<Player>, Without<Enemy>)>,
) {
    let Ok(player_t) = q_player.get_single() else { return; };
    let target = player_t.translation.truncate();

    for (t, mut v) in &mut q_enemies {
        let dir = (target - t.translation.truncate());
        **v = if dir.length_squared() > 1.0 { dir.normalize() * ENEMY_SPEED } else { Vec2::ZERO };
    }
}

fn move_with_collisions(
    time: Res<Time>,
    mut movers: Query<(Entity, &mut Transform, &Velocity, &Size), Without<Wall>>,
    walls: Query<(&Transform, &Size), With<Wall>>,
) {
    let dt = time.delta_secs();

    for (_e, mut t, v, s) in &mut movers {
        let mut pos = t.translation.truncate();
        let half = s.0 * 0.5;

        // Move X
        pos.x += v.x * dt;
        if overlaps_any(pos, half, &walls) {
            pos.x -= v.x * dt;
            pos.x += sweep_axis(pos, half, v.x * dt, Axis::X, &walls);
        }

        // Move Y
        pos.y += v.y * dt;
        if overlaps_any(pos, half, &walls) {
            pos.y -= v.y * dt;
            pos.y += sweep_axis(pos, half, v.y * dt, Axis::Y, &walls);
        }

        t.translation.x = pos.x;
        t.translation.y = pos.y;
    }
}

fn bullet_wall_cull(
    mut commands: Commands,
    q_bullets: Query<(Entity, &Transform, &Size), With<Bullet>>,
    walls: Query<(&Transform, &Size), With<Wall>>,
) {
    for (e, t, s) in &q_bullets {
        if overlaps_any(t.translation.truncate(), s.0 * 0.5, &walls) {
            commands.entity(e).despawn();
        }
    }
}

fn bullet_hits(
    mut commands: Commands,
    q_bullets: Query<(Entity, &Transform, &Size), With<Bullet>>,
    q_enemies: Query<(Entity, &Transform, &Size), With<Enemy>>,
) {
    for (b_e, b_t, b_s) in &q_bullets {
        let b_pos = b_t.translation.truncate();
        let b_half = b_s.0 * 0.5;

        for (e_e, e_t, e_s) in &q_enemies {
            if aabb_overlap(b_pos, b_half, e_t.translation.truncate(), e_s.0 * 0.5) {
                commands.entity(b_e).despawn();
                commands.entity(e_e).despawn();
                break;
            }
        }
    }
}

fn clamp_to_arena(mut q: Query<&mut Transform, Or<(With<Player>, With<Enemy>, With<Bullet>)>>) {
    let half_w = ARENA_W * 0.5;
    let half_h = ARENA_H * 0.5;
    for mut t in &mut q {
        t.translation.x = t.translation.x.clamp(-half_w, half_w);
        t.translation.y = t.translation.y.clamp(-half_h, half_h);
    }
}

fn enemy_spawner(
    time: Res<Time>,
    mut timer: ResMut<EnemySpawnTimer>,
    mut spawns: ResMut<SpawnPoints>, // <-- mutate safely
    q_enemies: Query<Entity, With<Enemy>>,
    mut commands: Commands,
) {
    timer.0.tick(time.delta());
    if !timer.0.finished() { return; }
    if q_enemies.iter().len() >= ENEMY_CAP { return; }
    if spawns.points.is_empty() { return; }

    let idx = spawns.next % spawns.points.len();
    let pos = spawns.points[idx];

    commands.spawn((
        Sprite {
            color: Color::srgb(0.9, 0.2, 0.2),
            custom_size: Some(ENEMY_SIZE),
            ..default()
        },
        Transform::from_xyz(pos.x, pos.y, 0.75),
        Enemy,
        Velocity(Vec2::ZERO),
        Size(ENEMY_SIZE),
    ));

    spawns.next = (spawns.next + 1) % spawns.points.len();
}

// === Math & Collision Helpers ===
#[inline]
fn aabb_overlap(a_pos: Vec2, a_half: Vec2, b_pos: Vec2, b_half: Vec2) -> bool {
    (a_pos.x - b_pos.x).abs() <= (a_half.x + b_half.x) &&
        (a_pos.y - b_pos.y).abs() <= (a_half.y + b_half.y)
}

fn overlaps_any(pos: Vec2, half: Vec2, walls: &Query<(&Transform, &Size), With<Wall>>) -> bool {
    for (wt, ws) in walls.iter() {
        if aabb_overlap(pos, half, wt.translation.truncate(), ws.0 * 0.5) {
            return true;
        }
    }
    false
}

enum Axis { X, Y }

fn sweep_axis(
    mut pos: Vec2,
    half: Vec2,
    delta: f32,
    axis: Axis,
    walls: &Query<(&Transform, &Size), With<Wall>>,
) -> f32 {
    if delta == 0.0 { return 0.0; }
    let steps = 6;
    let step = delta / steps as f32;
    let mut moved = 0.0;

    for _ in 0..steps {
        match axis {
            Axis::X => pos.x += step,
            Axis::Y => pos.y += step,
        }
        if overlaps_any(pos, half, walls) {
            match axis {
                Axis::X => pos.x -= step,
                Axis::Y => pos.y -= step,
            }
            break;
        } else {
            moved += step;
        }
    }
    moved
}
