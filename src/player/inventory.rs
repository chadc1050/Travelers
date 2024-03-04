use bevy::prelude::*;

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
            // .add_systems(Update, inventory_position_system)
            .add_systems(Update, toggle_inventory_system);
    }
}

fn initialize_inventory(mut commands: Commands, assets: Res<AssetServer>) {
    info!("Initializing inventory");

    let texture_handle = assets.load::<Image>("sprites/display/items/inventory.png");

    let container_node = NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        ..default()
    };

    let image_bundle = ImageBundle {
        image: UiImage {
            texture: texture_handle,
            ..Default::default()
        },
        style: Style {
            width: Val::Vw(20.),
            ..Default::default()
        },
        ..Default::default()
    };

    let container = commands
        .spawn(container_node)
        .insert(Visibility::Hidden)
        .insert(Inventory {})
        .id();

    let sprite: Entity = commands
        .spawn(image_bundle)
        .insert(Visibility::Inherited)
        .id();

    commands.entity(container).push_children(&[sprite]);
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
