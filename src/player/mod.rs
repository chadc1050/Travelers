use bevy::{
    app::{prelude::*, App, Plugin},
    asset::{AssetServer, Assets},
    ecs::{
        component::Component,
        query::Without,
        system::{Commands, Query, Res, ResMut},
    },
    input::{keyboard::KeyCode, Input},
    log::{debug, info},
    math::{Vec2, Vec3},
    prelude::default,
    render::{camera::Camera, color::Color},
    sprite::{Sprite, SpriteBundle, TextureAtlas},
    transform::components::Transform,
};

use crate::components::{Direction, Health, Velocity};

use crate::player::inventory::Inventory;

use self::inventory::InventoryPlugin;

mod inventory;

#[derive(Component)]
pub struct Player {
    max_speed: f32,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InventoryPlugin)
            .add_systems(Startup, player_spawn_system)
            .add_systems(Update, camera_follow)
            .add_systems(Update, player_movement);
    }
}

fn player_spawn_system(
    mut commands: Commands,
    _: ResMut<AssetServer>,
    _: ResMut<Assets<TextureAtlas>>,
) {
    let sprite = SpriteBundle {
        sprite: Sprite {
            color: Color::rgb(0.25, 0.25, 0.75),
            custom_size: Some(Vec2::new(20., 40.)),
            ..default()
        },
        ..default()
    };

    info!("Spawning player");
    commands
        .spawn(sprite)
        .insert(Player { max_speed: 100.0 })
        .insert(Velocity { dx: 0., dy: 0. })
        .insert(Transform::from_translation(Vec3::new(0., 0., 1.)))
        .insert(Direction::Right)
        .insert(Health {
            current: 100,
            max: 100,
        });
}

fn camera_follow(
    player_query: Query<(&Player, &Transform), Without<Camera>>,
    mut camera_query: Query<(&mut Transform, &Camera), Without<Player>>,
) {
    if let Ok((mut cam_transform, _)) = camera_query.get_single_mut() {
        if let Ok((_, player_transform)) = player_query.get_single() {
            cam_transform.translation = player_transform.translation;
        }
    }
}

fn player_movement(kb: Res<Input<KeyCode>>, mut query: Query<(&mut Velocity, &Player)>) {
    if let Ok((mut velocity, player_state)) = query.get_single_mut() {
        velocity.dx = 0.0;
        if kb.pressed(KeyCode::Left) || kb.pressed(KeyCode::A) {
            debug!("Player moved left!");
            velocity.dx -= player_state.max_speed;
        }
        if kb.pressed(KeyCode::Right) || kb.pressed(KeyCode::D) {
            debug!("Player moved right!");
            velocity.dx += player_state.max_speed;
        }

        velocity.dy = 0.0;
        if kb.pressed(KeyCode::Up) || kb.pressed(KeyCode::W) {
            debug!("Player moved up!");
            velocity.dy += player_state.max_speed;
        }
        if kb.pressed(KeyCode::Down) || kb.pressed(KeyCode::S) {
            debug!("Player moved down!");
            velocity.dy -= player_state.max_speed;
        }
    }
}
