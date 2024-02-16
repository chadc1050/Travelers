use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    io::ErrorKind,
};

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    render::render_resource::Texture,
    utils::BoxedFuture,
};
use rand::{Rng, SeedableRng};
use serde::Deserialize;

const CHUNK_TILE_LENGTH: i64 = 8;
const TILE_SIZE: i64 = 32;
const CHUNK_SIZE: i64 = CHUNK_TILE_LENGTH * TILE_SIZE;

const TOTAL_TILES: u8 = 2;

const RENDER_DISTANCE: i8 = 2;

type Tile = Option<(u8, u8)>;
type Coords = (i64, i64);
type Adjacencies = (Option<Chunk>, Option<Chunk>, Option<Chunk>, Option<Chunk>);

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
    chunks: Query<(Entity, &Chunk, &Transform)>,
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
                for (_, _, transform) in chunks.iter() {
                    if transform.translation.x as i64 == in_range.0
                        && transform.translation.y as i64 == in_range.1
                    {
                        present = true;
                        break;
                    }
                }

                if !present {
                    info!(
                        "{}",
                        format!(
                            "Found chunk needing to be generated ({},{})",
                            in_range.0, in_range.1
                        )
                    );

                    let schematic = schematic
                        .get(&schematic_handle)
                        .expect("Error loading in schematic!");

                    let wfc = WaveFunctionCollapse {
                        world_seed: 42,
                        schematic: schematic.clone(),
                    };

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

                    let chunk_bundle = (
                        Chunk {},
                        Transform::from_translation(Vec3::new(
                            in_range.0 as f32,
                            in_range.0 as f32,
                            0.,
                        )),
                        InheritedVisibility::default(),
                        GlobalTransform::default(),
                    );

                    let tiles = wfc.collapse(in_range, &get_adjacent(in_range, &chunks));

                    commands.spawn(chunk_bundle).with_children(|parent| {
                        for x in 0..CHUNK_TILE_LENGTH {
                            for y in 0..CHUNK_TILE_LENGTH {
                                if let Some(tile) = tiles[x as usize][y as usize] {
                                    let sprite_bundle = SpriteSheetBundle {
                                        texture_atlas: atlas_handle.clone(),
                                        ..Default::default()
                                    };

                                    parent.spawn(sprite_bundle).insert(
                                        Transform::from_translation(Vec3::new(
                                            (in_range.0 as f32) + (x as f32 * TILE_SIZE as f32),
                                            (in_range.1 as f32) + (y as f32 * TILE_SIZE as f32),
                                            0.,
                                        )),
                                    );
                                }
                            }
                        }
                    });
                }
            }

            // Handle removing of chunks that are out of range
            for (entity, chunk, transform) in chunks.iter() {
                let mut is_stale = true;
                for in_range in &chunks_in_range {
                    if transform.translation.x as i64 == in_range.0
                        && transform.translation.y as i64 == in_range.1
                    {
                        is_stale = false;
                        break;
                    }
                }
                if is_stale {
                    info!("Removing chunk that is no longer in range.");
                    commands.entity(entity).despawn_recursive();
                }
            }
        }
    }
}

fn get_adjacent(coords: &Coords, chunks: &Query<(Entity, &Chunk, &Transform)>) -> Adjacencies {
    let (mut north, mut east, mut south, mut west) =
        (Option::None, Option::None, Option::None, Option::None);

    for (_, chunk, transform) in chunks.iter() {
        let to_check = (
            transform.translation.x as i64,
            transform.translation.y as i64,
        );
        if coords.0 == to_check.0 && coords.1 + CHUNK_SIZE == to_check.1 {
            north = Some(chunk.clone());
        } else if coords.0 + CHUNK_SIZE == to_check.0 && coords.1 == to_check.1 {
            east = Some(chunk.clone());
        } else if coords.0 - CHUNK_SIZE == to_check.0 && coords.1 == to_check.1 {
            south = Some(chunk.clone());
        } else if coords.0 == to_check.0 && coords.1 - CHUNK_SIZE == to_check.1 {
            west = Some(chunk.clone())
        }
    }

    (north, east, south, west)
}

fn get_chunks_in_range(pos: (f32, f32)) -> Vec<Coords> {
    let offset_x = pos.0 as i64 / CHUNK_SIZE;

    let offset_y = pos.1 as i64 / CHUNK_SIZE;

    let mut coords = vec![Coords::default(); ((2 * RENDER_DISTANCE) ^ 2) as usize];

    for x in -RENDER_DISTANCE..=RENDER_DISTANCE {
        for y in -RENDER_DISTANCE..=RENDER_DISTANCE {
            coords.push((
                (offset_x + x as i64) * CHUNK_SIZE,
                (offset_y + y as i64) * CHUNK_SIZE,
            ));
        }
    }

    coords
}

#[derive(Resource)]
pub struct ImageResource(Handle<Image>);

#[derive(Resource)]
pub struct AtlasResource(Handle<TextureAtlas>);

#[derive(Resource)]
pub struct SchematicResource(Handle<SchematicAsset>);

#[derive(Asset, Clone, Debug, TypePath, Deserialize)]
pub struct SchematicAsset {
    #[serde(flatten)]
    pub tiles: HashMap<String, TileSchematic>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TileSchematic {
    pub name: String,
    pub sheet: String,
    pub x: u8,
    pub y: u8,
    pub weight: u8,
    #[serde(rename = "0")]
    pub north: Vec<u8>,
    #[serde(rename = "1")]
    pub east: Vec<u8>,
    #[serde(rename = "2")]
    pub south: Vec<u8>,
    #[serde(rename = "3")]
    pub west: Vec<u8>,
}

#[derive(Default)]
pub struct SchematicLoader;

impl AssetLoader for SchematicLoader {
    type Asset = SchematicAsset;

    type Settings = ();

    type Error = std::io::Error;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _: &'a Self::Settings,
        _: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            _ = reader.read_to_end(&mut bytes).await;
            let serialized = serde_json::from_slice::<SchematicAsset>(&bytes);

            match serialized {
                Ok(data) => {
                    info!("Successfully loaded asset");
                    Ok(data)
                }
                Err(err) => Err(Self::Error::new(
                    ErrorKind::InvalidData,
                    format!("Failed to deserialize Json File! Err {err}"),
                )),
            }
        })
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

#[derive(Clone, Component)]
pub struct Chunk;

// https://gist.github.com/jdah/ad997b858513a278426f8d91317115b9
// https://gamedev.stackexchange.com/questions/188719/deterministic-procedural-wave-function-collapse
struct WaveFunctionCollapse {
    world_seed: u64,
    schematic: SchematicAsset,
}

impl WaveFunctionCollapse {
    fn collapse(&self, coords: &Coords, adjacent: &Adjacencies) -> Vec<Vec<Option<(u8, u8)>>> {
        let mut tiles =
            vec![vec![Tile::None; CHUNK_TILE_LENGTH as usize]; CHUNK_TILE_LENGTH as usize];

        // Generate bottom left
        tiles[0][0] = self.scratch(coords);

        tiles
    }

    fn scratch(&self, coords: &Coords) -> Tile {
        let mut hasher = DefaultHasher::new();
        (coords.0 + coords.1).hash(&mut hasher);
        let hash = hasher.finish();

        let mut rng = rand::rngs::StdRng::seed_from_u64(hash);
        Some((rng.gen_range(0..TOTAL_TILES), 1))
    }
}
