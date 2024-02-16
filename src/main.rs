use bevy::{
    app::{Startup, Update},
    core_pipeline::core_2d::Camera2dBundle,
    prelude::*,
};
use components::{Dead, Health, Velocity};

mod player;

mod components;

mod world;

mod debug;

fn main() {
    info!("Starting Travelers...");
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Travelers".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(debug::DebugPlugin)
        .add_plugins(world::WorldPlugin)
        .add_plugins(player::PlayerPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, movement_system)
        .add_systems(Update, check_death)
        .run();
}

fn setup(mut commands: Commands, _: Res<AssetServer>) {
    info!("Setting up");

    info!("Creating camera");
    let mut cam = Camera2dBundle::default();
    cam.projection.scale *= 1.0;
    commands.spawn(cam);
}

fn movement_system(time: Res<Time>, mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in query.iter_mut() {
        let translation: &mut Vec3 = &mut transform.translation;
        translation.x += velocity.dx * time.delta_seconds();
        translation.y += velocity.dy * time.delta_seconds();
    }
}

fn check_death(mut commands: Commands, query: Query<(Entity, &Health), Without<Dead>>) {
    for (entity, health) in query.iter() {
        if health.current <= 0 {
            commands.entity(entity).insert(components::Dead);
        }
    }
}
