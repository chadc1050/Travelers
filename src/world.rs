use std::{collections::{hash_map::DefaultHasher, HashMap}, hash::{Hash, Hasher}, io::ErrorKind};

use bevy::{asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext}, prelude::*, transform::commands, utils::BoxedFuture};
use rand::{Rng, SeedableRng};
use serde::Deserialize;

const CHUNK_TILE_LENGTH: i64 = 8;
const TILE_SIZE: i64 = 128;
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

    let handle = asset_server.load("schematic.json");
    commands.insert_resource(SchematicResource(handle));
    // TODO: Load textures
    
}

fn world_gen_system(
    mut commands: Commands,
    cam_pos: Query<&Transform, With<Camera>>,
    chunks: Query<(Entity, &Chunk)>, 
    asset_server: Res<AssetServer>,
    assets: Res<Assets<SchematicAsset>>
) {

    info!("Updating world [Current Chunks: {}]", chunks.iter().collect::<Vec<_>>().len());

    // Retrieve assets
    if let Some(schematic_handle) = asset_server.get_handle::<SchematicAsset>("schematic.json") {

        debug!("Scematic loaded");

        // Get Chunks in range
        let cam_coords = cam_pos.get_single()
            .expect("Could not get camera position!")
            .translation;

        info!("Player coordinates: ({}, {})", cam_coords.x, cam_coords.y);

        let player_coords = (cam_coords.x, cam_coords.y);

        let chunks_in_range = get_chunks_in_range(player_coords);

        // Handle creation of new chunks
        for in_range in &chunks_in_range {
            let mut present = false;
            for (_, chunk) in chunks.iter() {
                if chunk.coords.0 == in_range.0 && chunk.coords.1 == in_range.1 {
                    present = true;
                    break;
                }
            }
            
            if !present {

                info!("{}", format!("Found chunk needing to be generated ({},{})", in_range.0, in_range.1));
                
                let schematic = assets.get(&schematic_handle).expect("Error loading in schematic!");

                let wfc = WaveFunctionCollapse {
                    world_seed: 42,
                    schematic: schematic.clone()
                };

                let to_spawn = wfc.collapse(in_range, &get_adjacent(in_range, &chunks));

                info!("Spawning chunk");

                let sprite = SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgb(0., 0.4, 0.1),
                        custom_size: Some(Vec2::new(CHUNK_SIZE as f32, CHUNK_SIZE as f32)),
                        
                        ..default()
                    },
                    ..default()
                };
                
                commands.spawn(sprite)
                    .insert(to_spawn.clone())
                    .insert(Transform::from_translation(Vec3::new(to_spawn.coords.0 as f32, to_spawn.coords.1 as f32, 0.)));
            }
        }

        // Handle removing of chunks that are out of range
        for (entity, chunk) in chunks.iter() {
            let mut is_stale = true;
            for in_range in &chunks_in_range {
                if chunk.coords.0 == in_range.0 && chunk.coords.1 == in_range.1 {
                    is_stale = false;
                    break;
                }
            } 
            if is_stale {
                info!("Removing chunk that is no longer in range.");
                commands.entity(entity).despawn();
            }
        }
    }
}

fn get_adjacent(coords: &Coords, chunks: &Query<(Entity, &Chunk)>) -> Adjacencies {

    let (mut north, mut east, mut south, mut west) = (Option::None, Option::None, Option::None, Option::None);

    for (_, chunk) in chunks.iter() {
        let to_check = chunk.coords;
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
            coords.push(((offset_x + x as i64) * CHUNK_SIZE, (offset_y + y as i64) * CHUNK_SIZE));
        }
    }

    coords
}


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
    pub weight: u8,
    #[serde(rename = "0")]
    pub north: Vec<u8>,
    #[serde(rename = "1")]
    pub east: Vec<u8>,
    #[serde(rename = "2")]
    pub south: Vec<u8>,
    #[serde(rename = "3")]
    pub west: Vec<u8>
}

#[derive(Default)]
pub struct SchematicLoader;

impl AssetLoader for SchematicLoader {

    type Asset = SchematicAsset;

    type Settings = ();

    type Error = std::io::Error;

    fn load<'a>(
        &'a self, reader: &'a mut Reader, 
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
                },
                Err(err) => Err(Self::Error::new(ErrorKind::InvalidData, format!("Failed to deserialize Json File! Err {err}"))),
            }
        })
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

#[derive(Clone, Component)]
pub struct Chunk {
    pub coords: Coords,
    pub tiles: Vec<Vec<Tile>>
}

// https://gist.github.com/jdah/ad997b858513a278426f8d91317115b9
// https://gamedev.stackexchange.com/questions/188719/deterministic-procedural-wave-function-collapse
struct WaveFunctionCollapse {
    world_seed: u64,
    schematic: SchematicAsset
}

impl WaveFunctionCollapse {

    fn collapse(&self, coords: &Coords, adjacent: &Adjacencies) -> Chunk {
        let mut tiles = vec![vec![Tile::None; CHUNK_TILE_LENGTH as usize]; CHUNK_TILE_LENGTH as usize];

        // Generate bottom left
        tiles[0][0] = self.scratch(coords);

        return Chunk {
            coords: coords.clone(),
            tiles
        };
    }

    fn scratch(&self, coords: &Coords) -> Tile {
        let mut hasher = DefaultHasher::new();
        (coords.0 + coords.1).hash(&mut hasher);
        let hash = hasher.finish();

        let mut rng = rand::rngs::StdRng::seed_from_u64(hash);
        let rand: u8 = rng.gen_range(0..=255);

        if rand == 0 {
            Some((0, 1))
        } else {
            Some((TOTAL_TILES % rand, 1))
        }
    }
}