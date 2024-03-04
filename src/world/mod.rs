use bevy::prelude::*;

use crate::{components::Dirty, world::stitcher::Stitcher, world::wfc::WaveFunctionCollapse};

use self::schematic::{SchematicAsset, SchematicLoader, SchematicResource};

mod schematic;

mod wfc;

mod stitcher;

const CHUNK_TILE_LENGTH: i64 = 8;
const TILE_SIZE: i64 = 32;
const CHUNK_SIZE: i64 = CHUNK_TILE_LENGTH * TILE_SIZE;

const RENDER_DISTANCE: i8 = 2;

#[derive(Copy, Clone, Debug, Default)]
struct ChunkCoords(i64, i64);

impl From<&Transform> for ChunkCoords {
    fn from(value: &Transform) -> Self {
        ChunkCoords(
            (value.translation.x - (CHUNK_SIZE / 2) as f32) as i64,
            (value.translation.y - (CHUNK_SIZE / 2) as f32) as i64,
        )
    }
}

impl PartialEq<Transform> for ChunkCoords {
    fn eq(&self, transform: &Transform) -> bool {
        self.0 == (transform.translation.x - (CHUNK_SIZE as f32 / 2.)) as i64
            && self.1 == (transform.translation.y - (CHUNK_SIZE as f32 / 2.)) as i64
    }
}

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

#[derive(Copy, Clone, Component, Debug)]
pub struct Chunk;

#[derive(Copy, Clone, Component, Debug)]
pub struct Tile {
    texture_id: u8,
}

// TODO: Refactor staged generation
enum WorldState {
    AssetLoad,
    WorldGeneration,
    Complete,
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<SchematicAsset>()
            .init_asset_loader::<SchematicLoader>()
            .add_systems(Startup, load_schematic)
            .add_systems(Update, gen_chunks)
            .add_systems(Update, gen_chunk_stitches);
    }
}

fn load_schematic(asset_server: Res<AssetServer>, mut commands: Commands) {
    info!("Loading world generation assets");

    // Load schematic
    let schematic_handle = asset_server.load("schematic.json");
    commands.insert_resource(SchematicResource(schematic_handle));

    // Load textures
    let sprite_sheet_handle = asset_server.load::<Image>("sprites/world/terrain/terrain_1.png");
    commands.insert_resource(ImageResource(sprite_sheet_handle));
}

fn gen_chunks(
    mut commands: Commands,
    cam_pos: Query<&Transform, With<Camera>>,
    chunks: Query<(Entity, &Transform, &Children), With<Chunk>>,
    asset_server: Res<AssetServer>,
    schematic: Res<Assets<SchematicAsset>>,
    atlas_asset: ResMut<Assets<TextureAtlas>>,
) {
    debug!("Updating chunk");

    // Retrieve assets
    if let Some(schematic_handle) = asset_server.get_handle::<SchematicAsset>("schematic.json") {
        if let Some(image_handle) =
            asset_server.get_handle::<Image>("sprites/world/terrain/terrain_1.png")
        {
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
            create_chunks(
                &chunks_in_range,
                &chunks,
                schematic,
                schematic_handle,
                image_handle,
                atlas_asset,
                &mut commands,
            );

            // Handle removing of chunks that are out of range
            remove_stale_chunks(&chunks_in_range, &chunks, &mut commands)
        }
    }
}

fn gen_chunk_stitches(
    mut commands: Commands,
    chunks_query: Query<(Entity, &Transform, &Children), With<Chunk>>,
    dirty_chunks_query: Query<(Entity, &Transform, &Children), (With<Dirty>, With<Chunk>)>,
    tiles_query: Query<(Entity, &Tile, &Transform)>,
    asset_server: Res<AssetServer>,
    schematic: Res<Assets<SchematicAsset>>,
    mut atlas_asset: ResMut<Assets<TextureAtlas>>,
) {
    debug!("Stitching chunks");

    // Retrieve assets
    if let Some(schematic_handle) = asset_server.get_handle::<SchematicAsset>("schematic.json") {
        if let Some(image_handle) =
            asset_server.get_handle::<Image>("sprites/world/terrain/terrain_1.png")
        {
            if dirty_chunks_query.is_empty() {
                debug!("No chunks needing to be stitched.");
                return;
            }

            let schematic = schematic
                .get(&schematic_handle)
                .expect("Error loading in schematic!");

            for (entity, transform, children) in dirty_chunks_query.iter() {
                // Get adjacencies to chunks

                let coords = ChunkCoords::from(transform);

                let chunk = get_chunk_tiles(children, &tiles_query);

                let adj = get_connected_chunks(
                    &ChunkCoords::from(transform),
                    &chunks_query,
                    &tiles_query,
                );

                // Stitch together chunk with neighbors
                let mut stitcher = Stitcher::init(schematic, coords, chunk, adj);
                let edges = stitcher.stitch();

                let atlas = TextureAtlas::from_grid(
                    image_handle.clone(),
                    Vec2::new(TILE_SIZE as f32, TILE_SIZE as f32),
                    10,
                    16,
                    None,
                    None,
                );

                let atlas_handle = atlas_asset.add(atlas);

                commands
                    .entity(entity)
                    .with_children(|parent| {
                        
                        // Add tiles to chunk
                        for (idx, tile) in edges.iter().enumerate() {

                            let tile_id: u8;

                            let side = idx / (CHUNK_TILE_LENGTH + 1) as usize;
                            let rank = idx % (CHUNK_TILE_LENGTH + 1) as usize;

                            debug!("Side: {:?}, Rank: {:?}", side, rank);

                            // North, East, South, West
                            let perim_tile_coords = get_perimeter_world_coord(&coords, side as i64, rank as i64);

                            let x_rel = (perim_tile_coords.0 - coords.0) as f32
                                + (TILE_SIZE as f32 / 2.)
                                - (CHUNK_SIZE as f32 / 2.);

                            let y_rel = (perim_tile_coords.1 - coords.1) as f32
                                + (TILE_SIZE as f32 / 2.)
                                - (CHUNK_SIZE as f32 / 2.);

                            if tile.is_some() {
    
                                tile_id = tile.unwrap();
            
                                debug!("Spawning stitched tile to chunk ({}, {}) at relative coordinates: ({},{})", coords.0, coords.1, x_rel, y_rel);
        
                            } else {
            
                                tile_id = schematic.not_found;

                                warn!(
                                    "Spawning stitched tile without texture to chunk ({}, {}) at relative coordinates: ({},{})",
                                    coords.0, coords.1, x_rel, y_rel
                                );
                            }

                            let sprite_bundle = SpriteSheetBundle {
                                texture_atlas: atlas_handle.clone(),
                                sprite: TextureAtlasSprite::new(tile_id as usize),
                                ..Default::default()
                            };

                            parent
                                .spawn(sprite_bundle)
                                .insert(Transform::from_translation(Vec3::new(
                                    x_rel, y_rel, 0.,
                                )))
                                .insert(Visibility::Inherited)
                                .insert(Tile {
                                    texture_id: tile_id,
                            });
                        }
                    })
                    .remove::<Dirty>();
            }
        }
    }
}

fn create_chunks(
    chunks_in_range: &Vec<ChunkCoords>,
    chunks: &Query<(Entity, &Transform, &Children), With<Chunk>>,
    schematic: Res<Assets<SchematicAsset>>,
    schematic_handle: Handle<SchematicAsset>,
    image_handle: Handle<Image>,
    mut atlas_asset: ResMut<Assets<TextureAtlas>>,
    commands: &mut Commands,
) {
    for in_range in chunks_in_range {
        let mut present = false;
        for (_, transform, _) in chunks.iter() {
            if in_range == transform {
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

            let mut wfc = WaveFunctionCollapse::init(42, schematic, in_range.clone());

            // Tiles is CHUNK_TILE_LENGTH x CHUNK_TILE_LENGTH
            let tiles = wfc.collapse();

            let chunk_bundle = (
                Chunk {},
                Dirty {},
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
                        let x_rel = (x as f32 * TILE_SIZE as f32) + (TILE_SIZE as f32 / 2.)
                            - (CHUNK_SIZE as f32 / 2.);

                        let y_rel = (y as f32 * TILE_SIZE as f32) + (TILE_SIZE as f32 / 2.)
                            - (CHUNK_SIZE as f32 / 2.);

                        let tile_id: u8;

                        let collapsed = tiles[x as usize][y as usize];
                        if collapsed.is_some() {

                            tile_id = collapsed.unwrap();

                            debug!(
                                "Spawning tile to chunk ({}, {}) at relative coordinates: ({},{})",
                                in_range.0, in_range.1, x_rel, y_rel
                            );

                        } else {

                            tile_id = schematic.not_found;

                            warn!(
                                "Spawning tile without texture to chunk ({}, {}) at relative coordinates: ({},{})",
                                in_range.0, in_range.1, x_rel, y_rel
                            );
                        }

                        let sprite_bundle = SpriteSheetBundle {
                            texture_atlas: atlas_handle.clone(),
                            sprite: TextureAtlasSprite::new(tile_id as usize),
                            ..Default::default()
                        };

                        parent
                            .spawn(sprite_bundle)
                            .insert(Transform::from_translation(Vec3::new(x_rel, y_rel, 0.)))
                            .insert(Visibility::Inherited)
                            .insert(Tile {
                                texture_id: tile_id,
                            });
                    }
                }
            });
        }
    }
}

fn remove_stale_chunks(
    chunks_in_range: &Vec<ChunkCoords>,
    chunks: &Query<(Entity, &Transform, &Children), With<Chunk>>,
    commands: &mut Commands,
) {
    for (entity, transform, _) in chunks.iter() {
        let is_stale = chunks_in_range.iter().all(|in_range| in_range != transform);

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

fn get_connected_chunks(
    coords: &ChunkCoords,
    chunks: &Query<(Entity, &Transform, &Children), With<Chunk>>,
    tiles: &Query<(Entity, &Tile, &Transform)>,
) -> Adjacencies {
    let (mut north, mut east, mut south, mut west) =
        (Option::None, Option::None, Option::None, Option::None);

    for (_, transform, children) in chunks.iter() {
        let to_check = ChunkCoords::from(transform);

        debug!("Checking adjacenties for ({},{})", to_check.0, to_check.1);

        if coords.0 == to_check.0 && coords.1 + CHUNK_SIZE + TILE_SIZE == to_check.1 {
            north = Some(get_chunk_tiles(children, tiles));
        } else if coords.0 + CHUNK_SIZE + TILE_SIZE == to_check.0 && coords.1 == to_check.1 {
            east = Some(get_chunk_tiles(children, tiles));
        } else if coords.0 - CHUNK_SIZE - TILE_SIZE == to_check.0 && coords.1 == to_check.1 {
            south = Some(get_chunk_tiles(children, tiles));
        } else if coords.0 == to_check.0 && coords.1 - CHUNK_SIZE - TILE_SIZE == to_check.1 {
            west = Some(get_chunk_tiles(children, tiles));
        }
    }

    (north, east, south, west)
}

fn get_chunk_tiles(
    chunk_children: &Children,
    tiles: &Query<(Entity, &Tile, &Transform)>,
) -> Vec<(Tile, Transform)> {
    let mut containing: Vec<(Tile, Transform)> = Vec::new();

    for child in chunk_children.iter() {
        debug!("Found child");
        if let Ok((_, tile, transform)) = tiles.get(*child) {
            containing.push((tile.clone(), transform.clone()));
        }
    }

    containing
}

// Get coords of chunks that are in the range of the camera, should account for chunk stitching
fn get_chunks_in_range(pos: (f32, f32)) -> Vec<ChunkCoords> {
    // Inverse linear equation to get offset with floor
    let offset_x = ((pos.0 as f32 - TILE_SIZE as f32) / (CHUNK_SIZE + TILE_SIZE) as f32).floor();
    let offset_y = ((pos.1 as f32 - TILE_SIZE as f32) / (CHUNK_SIZE + TILE_SIZE) as f32).floor();

    let mut coords = vec![ChunkCoords::default(); ((2 * RENDER_DISTANCE) ^ 2) as usize];

    // Feed offset back into linear equation and extrapolate to the render distance
    for x in -RENDER_DISTANCE..=RENDER_DISTANCE {
        for y in -RENDER_DISTANCE..=RENDER_DISTANCE {
            coords.push(ChunkCoords(
                ((offset_x as i64 + x as i64) * (CHUNK_SIZE + TILE_SIZE)) - TILE_SIZE,
                ((offset_y as i64 + y as i64) * (CHUNK_SIZE + TILE_SIZE)) - TILE_SIZE,
            ));
        }
    }

    coords
}

fn get_perimeter_world_coord(coords: &ChunkCoords, side: i64, rank: i64) -> ChunkCoords {
    match side {
        0 => ChunkCoords(
            coords.0 - TILE_SIZE + (rank * TILE_SIZE),
            coords.1 + CHUNK_SIZE,
        ),
        1 => ChunkCoords(
            coords.0 + CHUNK_SIZE,
            coords.1 + CHUNK_SIZE - (rank * TILE_SIZE),
        ),
        2 => ChunkCoords(
            coords.0 + CHUNK_SIZE - (rank * TILE_SIZE),
            coords.1 - TILE_SIZE,
        ),
        _ => ChunkCoords(
            coords.0 - TILE_SIZE,
            coords.1 - TILE_SIZE + (rank * TILE_SIZE),
        ),
    }
}
