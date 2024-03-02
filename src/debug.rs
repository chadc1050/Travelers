use std::vec;

use bevy::prelude::*;

use crate::{player::Player, world::Chunk};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_font);
        app.add_systems(Update, toggle_debug_info);
        app.add_systems(Update, update_debug_info);
    }
}

#[derive(Resource)]
pub struct FontResource(Handle<Font>);

#[derive(Component)]
pub struct DebugInfo;

fn setup_font(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load::<Font>("fonts/FiraMono-Medium.ttf");
    commands.insert_resource(FontResource(handle));
}

fn toggle_debug_info(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    input: Res<Input<KeyCode>>,
    query: Query<Entity, With<DebugInfo>>,
) {
    if let Some(font_handle) = asset_server.get_handle::<Font>("fonts/FiraMono-Medium.ttf") {
        if input.just_pressed(KeyCode::F3) {
            if let Ok(entity) = query.get_single() {
                // Delete it
                commands.entity(entity).despawn();
            } else {
                // Add marker

                let text_bundle = TextBundle {
                    text: Text {
                        sections: vec![
                            TextSection {
                                style: TextStyle {
                                    font_size: 20.0,
                                    color: Color::WHITE,
                                    font: font_handle
                                },
                                value: "".into()
                            };
                            4 as usize
                        ],
                        alignment: TextAlignment::Left,
                        ..Default::default()
                    },
                    ..Default::default()
                };

                commands.spawn(text_bundle).insert(DebugInfo {});
            }
        }
    }
}

fn update_debug_info(
    mut debug_query: Query<(Entity, &mut Text, &DebugInfo)>,
    player_query: Query<&Transform, With<Player>>,
    chunk_query: Query<(Entity, &Chunk)>,
    entities_query: Query<Entity>,
    time: Res<Time>,
) {
    if let Ok((_, mut text, _)) = debug_query.get_single_mut() {
        let player_coords = player_query.get_single().unwrap().translation;

        text.sections[0].value = format!("FPS: {:.2}", 1.0 / time.delta_seconds());

        text.sections[1].value = format!(
            "\nPlayer Coordinates: [{},{}]",
            player_coords.x, player_coords.y
        );

        let n_entities = entities_query.iter().collect::<Vec<_>>().len();
        text.sections[2].value = format!("\nTotal Entities: {}", n_entities);

        let n_chunks = chunk_query.iter().collect::<Vec<_>>().len();
        text.sections[3].value = format!("\nChunks Rendered: {}", n_chunks);
    }
}
