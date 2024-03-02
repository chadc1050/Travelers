use bevy::{prelude::*, render::view::Layer};

use super::Player;

#[derive(Clone, Copy, Component)]
pub struct Inventory;

#[derive(Clone, Copy, Component)]
pub struct Item<'a> {
    name: &'a str,
}

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, initialize_inventory)
            .add_systems(Update, inventory_position_system)
            .add_systems(Update, toggle_inventory_system);
    }
}

fn initialize_inventory(mut commands: Commands, assets: Res<AssetServer>) {
    info!("Initializing inventory");

    let texture_handle = assets.load::<Image>("sprites/display/items/inventory.png");

    let bundle = SpriteBundle {
        texture: texture_handle,
        transform: Transform {
            translation: Vec3::new(0., 0., 10.),
            scale: Vec3::new(1.5, 1.5, 1.0),
            ..Default::default()
        },
        ..Default::default()
    };

    commands
        .spawn(bundle)
        .insert(Visibility::Hidden)
        .insert(Inventory {});
}

fn inventory_position_system(
    mut inventory_query: Query<(Entity, &mut Transform), With<Inventory>>,
    camera_query: Query<(&mut Transform, &Camera), Without<Inventory>>,
) {
    if let Ok((cam_transform, _)) = camera_query.get_single() {
        if let Ok((_, mut inventoy_transform)) = inventory_query.get_single_mut() {
            inventoy_transform.translation = cam_transform.translation;
        }
    }
}

fn toggle_inventory_system(
    mut commands: Commands,
    mut inventory_query: Query<(Entity, &mut Visibility), With<Inventory>>,
    input: Res<Input<KeyCode>>,
) {
    if input.just_pressed(KeyCode::E) {
        let (entity, visibility) = inventory_query.get_single_mut().unwrap();

        let updated: Visibility;
        if *visibility == Visibility::Hidden {
            updated = Visibility::Visible;
        } else {
            updated = Visibility::Hidden;
        }

        commands.entity(entity).insert(updated);
    }
}
