use bevy::{app::{Startup, Update}, core_pipeline::core_2d::Camera2dBundle, ecs::component::Component, prelude::*};

mod player;

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
        .add_systems(Startup, setup)
        .add_systems(Update, movement_system)
        .add_plugins(player::PlayerPlugin)
        .run();
}


fn setup(mut commands: Commands, _: Res<AssetServer>) {

    info!("Setting Up...");

    info!("Creating camera");
    let mut cam = Camera2dBundle::default();
    cam.projection.scale *= 1.5;
    commands.spawn(cam);
}

fn movement_system(time: Res<Time>,mut query: Query<(&mut Transform, &Velocity)>,) {

	for (mut transform, velocity) in query.iter_mut() {
		let translation: &mut Vec3 = &mut transform.translation;
		translation.x += velocity.dx * time.delta_seconds();
		translation.y += velocity.dy * time.delta_seconds();
	}
}

#[derive(Component)]
pub struct Velocity {
    pub dx: f32,
    pub dy: f32
}

#[derive(Component)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right
}

#[derive(Component)]
pub struct Health {
    pub current: u8,
    pub max: u8
}