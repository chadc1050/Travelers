use std::{collections::{hash_map::DefaultHasher, HashMap}, hash::{Hash, Hasher}, io::ErrorKind};

use bevy::{asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext, LoadedFolder}, prelude::*, utils::BoxedFuture};
use rand::{Rng, SeedableRng};
use serde::Deserialize;

const CHUNK_SIZE: usize = 16;
const TILE_SIZE: usize = 256;
const TOTAL_TILES: u8 = 2;
const RENDER_DISTANCE: u8 = 2 * CHUNK_SIZE as u8;

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

fn load_schematic(asset_server: Res<AssetServer>) {

    info!("Loading world generation assets");

    _ = asset_server.load::<SchematicAsset>("schematic.json");

    // TODO: Load textures
    
}

fn world_gen_system(
    mut commands: Commands,
    cam_pos: Query<&Transform, With<Camera>>,
    chunks: Query<(Entity, &Chunk)>, 
    asset_server: Res<AssetServer>, 
    assets: Res<Assets<SchematicAsset>>
) {

    info!("Updating world");

    // Retrieve assets
    if let Some(schematic_handle) = asset_server.get_handle("schematic.json") {

        let schematic = assets.get(schematic_handle);

        // Get Chunks in range
        let cam_coords = cam_pos.get_single()
            .expect("Could not get camera position!")
            .translation;

        let player_coords = (cam_coords.x, cam_coords.y);
        let chunks_in_range = get_chunks_in_range(player_coords);

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
                commands.entity(entity).despawn();
            }
        }

        // Render chunks now in range
        todo!()
    }
}

fn get_chunks_in_range(pos: (f32, f32)) -> Vec<Coords> {
    todo!()
}

#[derive(Asset, Debug, TypePath, Deserialize)]
pub struct SchematicAsset {

    #[serde(flatten)]
    pub tiles: HashMap<String, TileSchematic>,
}

#[derive(Debug, Deserialize)]
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
                Ok(data) => Ok(data),
                Err(err) => Err(Self::Error::new(ErrorKind::InvalidData, format!("Failed to deserialize Json File! Err {err}"))),
            }
        })
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

#[derive(Component)]
pub struct Chunk {
    pub coords: Coords,
    pub tiles: Vec<Vec<Tile>>
}

// https://gist.github.com/jdah/ad997b858513a278426f8d91317115b9
// https://gamedev.stackexchange.com/questions/188719/deterministic-procedural-wave-function-collapse
struct WaveFunctionCollapse {
    seed: u64
}

impl WaveFunctionCollapse {

    fn collapse(&self, coords: Coords, adjacent: Adjacencies) -> Chunk {
        let mut tiles = vec![vec![Tile::None; CHUNK_SIZE]; CHUNK_SIZE];

        // Generate bottom left
        tiles[0][0] = self.scratch(coords);

        todo!()
    }

    fn scratch(&self, coords: Coords) -> Tile {
        let mut hasher = DefaultHasher::new();
        (coords.0 + coords.1).hash(&mut hasher);
        let hash = hasher.finish();

        let mut rng = rand::rngs::StdRng::seed_from_u64(hash);
        let rand: u8 = rng.gen_range(0..=255);

        Some((TOTAL_TILES % rand, 1))
    }
}
