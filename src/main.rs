use bevy::prelude::*;
use std::time::Duration;

const ARENA_W: f32 = 800.0;
const ARENA_H: f32 = 600.0;

const PLAYER_SPEED: f32 = 300.0;
const PLAYER_SIZE: Vec2 = Vec2::new(32.0, 32.0);

const BULLET_SPEED: f32 = 600.0;
const BULLET_SIZE: Vec2 = Vec2::new(6.0, 12.0);

#[derive(Component)] struct Player;
#[derive(Component)] struct Bullet;

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Component)]
struct Size(Vec2);

#[derive(Resource)]
struct FireCooldown(Timer);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Battle Tanks (Bevy 0.16)".into(),
                resolution: (ARENA_W, ARENA_H).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(FireCooldown(Timer::from_seconds(0.16, TimerMode::Once)))
        .add_systems(Startup, setup)
        .add_systems(Update, (player_input, handle_fire, move_entities, cull_offscreen))
        .run();
}

fn setup(mut commands: Commands) {
    // 2D camera (Bevy 0.16)
    commands.spawn((Camera2d::default(), Camera::default()));

    // Player sprite (singular: SpriteBundle)
    commands.spawn((
        Sprite {
            color: Color::srgb(0.2, 0.9, 0.2),
            custom_size: Some(PLAYER_SIZE),
            ..default()
        },
        Transform::from_xyz(0.0, -ARENA_H * 0.35, 1.0),
        Player,
        Velocity(Vec2::ZERO),
        Size(PLAYER_SIZE),
    ));
}

// Read keyboard and move + rotate the player
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

    transform.translation.x += vel.x * time.delta_secs();
    transform.translation.y += vel.y * time.delta_secs();

    let half_w = ARENA_W * 0.5 - PLAYER_SIZE.x * 0.5;
    let half_h = ARENA_H * 0.5 - PLAYER_SIZE.y * 0.5;
    transform.translation.x = transform.translation.x.clamp(-half_w, half_w);
    transform.translation.y = transform.translation.y.clamp(-half_h, half_h);
}

fn handle_fire(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut cooldown: ResMut<FireCooldown>,
    q_player: Query<&Transform, With<Player>>,
    mut commands: Commands,
) {
    cooldown.0.tick(time.delta());

    if input.just_pressed(KeyCode::Space) && cooldown.0.finished() {
        let Ok(t) = q_player.get_single() else { return; };
        let forward = t.rotation.mul_vec3(Vec3::X).truncate();
        let spawn_pos = t.translation.truncate() + forward * (PLAYER_SIZE.x * 0.6);

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
}

fn move_entities(time: Res<Time>, mut q: Query<(&mut Transform, &Velocity)>) {
    for (mut t, v) in &mut q {
        t.translation.x += v.x * time.delta_secs();
        t.translation.y += v.y * time.delta_secs();
    }
}

fn cull_offscreen(mut commands: Commands, q: Query<(Entity, &Transform), With<Bullet>>) {
    let half_w = ARENA_W * 0.5;
    let half_h = ARENA_H * 0.5;
    for (e, t) in &q {
        let p = t.translation.truncate();
        if p.x.abs() > half_w + 50.0 || p.y.abs() > half_h + 50.0 {
            commands.entity(e).despawn();
        }
    }
}
