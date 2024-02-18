use bevy::{
    prelude::*,
    transform::{self, commands},
};

use crate::world::wfc::WaveFunctionCollapse;

use self::schematic::{SchematicAsset, SchematicLoader, SchematicResource};

mod schematic;

mod wfc;

const CHUNK_TILE_LENGTH: i64 = 8;
const TILE_SIZE: i64 = 32;
const CHUNK_SIZE: i64 = CHUNK_TILE_LENGTH * TILE_SIZE;

const RENDER_DISTANCE: i8 = 3;

type Coords = (i64, i64);
type Adjacencies = (
    Option<Vec<(Tile, Transform)>>,
    Option<Vec<(Tile, Transform)>>,
    Option<Vec<(Tile, Transform)>>,
    Option<Vec<(Tile, Transform)>>,
);

#[derive(Resource)]
pub struct ImageResource(Handle<Image>);

#[derive(Resource)]
pub struct AtlasResource(Handle<TextureAtlas>);

#[derive(Clone, Component, Debug)]
pub struct Chunk;

#[derive(Clone, Component, Debug)]
pub struct Tile {
    texture_id: u8,
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<SchematicAsset>();
        app.init_asset_loader::<SchematicLoader>();
        app.add_systems(Startup, load_schematic);
        app.add_systems(Update, world_gen_system);
    }
}

fn load_schematic(asset_server: Res<AssetServer>, mut commands: Commands) {
    info!("Loading world generation assets");

    // Load schematic
    let schematic_handle = asset_server.load("schematic.json");
    commands.insert_resource(SchematicResource(schematic_handle));

    // Load textures
    let sprite_sheet_handle = asset_server.load::<Image>("world/terrain_1.png");
    commands.insert_resource(ImageResource(sprite_sheet_handle));
}

fn world_gen_system(
    mut commands: Commands,
    cam_pos: Query<&Transform, With<Camera>>,
    chunks: Query<(Entity, &Chunk, &Transform, &Children)>,
    tiles: Query<(Entity, &Tile, &Transform)>,
    asset_server: Res<AssetServer>,
    schematic: Res<Assets<SchematicAsset>>,
    mut atlas_asset: ResMut<Assets<TextureAtlas>>,
) {
    debug!("Updating world");

    // Retrieve assets
    if let Some(schematic_handle) = asset_server.get_handle::<SchematicAsset>("schematic.json") {
        if let Some(image_handle) = asset_server.get_handle::<Image>("world/terrain_1.png") {
            debug!("Scematic loaded");

            // Get Chunks in range
            let cam_coords = cam_pos
                .get_single()
                .expect("Could not get camera position!")
                .translation;

            debug!("Player coordinates: ({}, {})", cam_coords.x, cam_coords.y);

            let player_coords = (cam_coords.x, cam_coords.y);

            let chunks_in_range = get_chunks_in_range(player_coords);

            // Handle creation of new chunks
            for in_range in &chunks_in_range {
                let mut present = false;
                for (_, _, transform, _) in chunks.iter() {
                    if in_range.0 == (transform.translation.x - (CHUNK_SIZE as f32 / 2.)) as i64
                        && in_range.1 == (transform.translation.y - (CHUNK_SIZE as f32 / 2.)) as i64
                    {
                        present = true;
                        break;
                    }
                }

                if !present {
                    info!(
                        "{}",
                        format!(
                            "Found chunk needing to be generated: ({},{})",
                            in_range.0, in_range.1
                        )
                    );

                    let schematic = schematic
                        .get(&schematic_handle)
                        .expect("Error loading in schematic!");

                    info!("Spawning chunk");

                    let atlas = TextureAtlas::from_grid(
                        image_handle.clone(),
                        Vec2::new(TILE_SIZE as f32, TILE_SIZE as f32),
                        10,
                        16,
                        None,
                        None,
                    );

                    let atlas_handle = atlas_asset.add(atlas);

                    let mut wfc = WaveFunctionCollapse::init(
                        42,
                        schematic.clone(),
                        in_range.clone(),
                        get_adjacent(in_range, &chunks, &tiles),
                    );

                    let tiles = wfc.collapse();

                    let chunk_bundle = (
                        Chunk {},
                        Transform::from_translation(Vec3::new(
                            in_range.0 as f32 + (CHUNK_SIZE as f32 / 2.),
                            in_range.1 as f32 + (CHUNK_SIZE as f32 / 2.),
                            0.,
                        )),
                        InheritedVisibility::default(),
                        GlobalTransform::default(),
                    );

                    commands.spawn(chunk_bundle).with_children(|parent| {
                        for x in 0..CHUNK_TILE_LENGTH {
                            for y in 0..CHUNK_TILE_LENGTH {
                                if let Some(tile) = tiles[x as usize][y as usize] {
                                    let sprite_bundle = SpriteSheetBundle {
                                        texture_atlas: atlas_handle.clone(),
                                        sprite: TextureAtlasSprite::new(tile.0 as usize),
                                        ..Default::default()
                                    };

                                    parent
                                        .spawn(sprite_bundle)
                                        .insert(Transform::from_translation(Vec3::new(
                                            (x as f32 * TILE_SIZE as f32) - (TILE_SIZE / 2) as f32,
                                            (y as f32 * TILE_SIZE as f32) - (TILE_SIZE / 2) as f32,
                                            0.,
                                        )))
                                        .insert(Visibility::Inherited)
                                        .insert(Tile { texture_id: tile.0 });
                                }
                            }
                        }
                    });
                }
            }

            // Handle removing of chunks that are out of range
            for (entity, _, transform, _) in chunks.iter() {
                let mut is_stale = true;
                for in_range in &chunks_in_range {
                    if (transform.translation.x - (CHUNK_SIZE as f32 / 2.)) as i64 == in_range.0
                        && (transform.translation.y - (CHUNK_SIZE as f32 / 2.)) as i64 == in_range.1
                    {
                        is_stale = false;
                        break;
                    }
                }
                if is_stale {
                    info!(
                        "Removing out of range chunk: ({},{})",
                        (transform.translation.x - (CHUNK_SIZE as f32 / 2.)) as i64,
                        (transform.translation.y - (CHUNK_SIZE as f32 / 2.)) as i64
                    );
                    commands.entity(entity).despawn_recursive();
                }
            }
        }
    }
}

fn get_adjacent(
    coords: &Coords,
    chunks: &Query<(Entity, &Chunk, &Transform, &Children)>,
    tiles: &Query<(Entity, &Tile, &Transform)>,
) -> Adjacencies {
    let (mut north, mut east, mut south, mut west) =
        (Option::None, Option::None, Option::None, Option::None);

    // TODO: Need to fix timing of query, chunks may be generated during this step so adjacency check is only accurate as of beginning of step

    for (entity, _, transform, children) in chunks.iter() {
        let to_check = (
            (transform.translation.x - (CHUNK_SIZE as f32 / 2.)) as i64,
            (transform.translation.y - (CHUNK_SIZE as f32 / 2.)) as i64,
        );

        info!("Checking adjacenties for ({},{})", to_check.0, to_check.1);

        if coords.0 == to_check.0 && coords.1 + CHUNK_SIZE + TILE_SIZE == to_check.1 {
            north = Some(get_chunk_tiles((entity, children), tiles));
        } else if coords.0 + CHUNK_SIZE + TILE_SIZE == to_check.0 && coords.1 == to_check.1 {
            east = Some(get_chunk_tiles((entity, children), tiles));
        } else if coords.0 - CHUNK_SIZE - TILE_SIZE == to_check.0 && coords.1 == to_check.1 {
            south = Some(get_chunk_tiles((entity, children), tiles));
        } else if coords.0 == to_check.0 && coords.1 - CHUNK_SIZE - TILE_SIZE == to_check.1 {
            west = Some(get_chunk_tiles((entity, children), tiles));
        }
    }

    (north, east, south, west)
}

fn get_chunk_tiles(
    chunk_children: (Entity, &Children),
    tiles: &Query<(Entity, &Tile, &Transform)>,
) -> Vec<(Tile, Transform)> {
    let mut containing: Vec<(Tile, Transform)> = Vec::new();

    for child in chunk_children.1.iter() {
        info!("Found child");
        if let Ok((_, tile, transform)) = tiles.get(*child) {
            containing.push((tile.clone(), transform.clone()));
        }
    }

    containing
}

// Get coords of chunks that are in the range of the camera, should account for chunk stitching
fn get_chunks_in_range(pos: (f32, f32)) -> Vec<Coords> {
    // Inverse linear equation to get offset with floor
    let offset_x = ((pos.0 as f32 - TILE_SIZE as f32) / (CHUNK_SIZE + TILE_SIZE) as f32).floor();
    let offset_y = ((pos.1 as f32 - TILE_SIZE as f32) / (CHUNK_SIZE + TILE_SIZE) as f32).floor();

    let mut coords = vec![Coords::default(); ((2 * RENDER_DISTANCE) ^ 2) as usize];

    // Feed offset back into linear equation and extrapolate to the render distance
    for x in -RENDER_DISTANCE..=RENDER_DISTANCE {
        for y in -RENDER_DISTANCE..=RENDER_DISTANCE {
            coords.push((
                ((offset_x as i64 + x as i64) * (CHUNK_SIZE + TILE_SIZE)) - TILE_SIZE,
                ((offset_y as i64 + y as i64) * (CHUNK_SIZE + TILE_SIZE)) - TILE_SIZE,
            ));
        }
    }

    coords
}
