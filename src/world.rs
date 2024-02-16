use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::{Hash, Hasher},
    io::ErrorKind,
};

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    utils::BoxedFuture,
};
use rand::{Rng, SeedableRng};
use serde::Deserialize;

const CHUNK_TILE_LENGTH: i64 = 8;
const TILE_SIZE: i64 = 32;
const CHUNK_SIZE: i64 = CHUNK_TILE_LENGTH * TILE_SIZE;

const RENDER_DISTANCE: i8 = 3;

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

                    let mut wfc = WaveFunctionCollapse::init(
                        42,
                        schematic.clone(),
                        in_range.clone(),
                        get_adjacent(in_range, &chunks),
                    );

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
                            in_range.0 as f32 + (CHUNK_SIZE as f32 / 2.),
                            in_range.1 as f32 + (CHUNK_SIZE as f32 / 2.),
                            0.,
                        )),
                        InheritedVisibility::default(),
                        GlobalTransform::default(),
                    );

                    let tiles = wfc.collapse();

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
                                        .insert(Visibility::Inherited);
                                }
                            }
                        }
                    });
                }
            }

            // Handle removing of chunks that are out of range
            for (entity, _, transform) in chunks.iter() {
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
    hash: u64,
    adj: Adjacencies,
    schematic: SchematicAsset,
    constatint_map: Vec<Vec<HashSet<u8>>>,
    tiles: Vec<Vec<Option<(u8, u8)>>>,
}

impl WaveFunctionCollapse {
    pub fn init(
        world_seed: u64,
        schematic: SchematicAsset,
        coords: Coords,
        adj: Adjacencies,
    ) -> WaveFunctionCollapse {
        WaveFunctionCollapse {
            hash: Self::get_hash(world_seed, &coords),
            adj: adj,
            schematic: schematic.clone(),
            constatint_map: vec![
                vec![
                    (0..(schematic.tiles.len() as u8)).collect();
                    CHUNK_TILE_LENGTH as usize
                ];
                CHUNK_TILE_LENGTH as usize
            ],
            tiles: vec![vec![None; CHUNK_TILE_LENGTH as usize]; CHUNK_TILE_LENGTH as usize],
        }
    }

    pub fn collapse(&mut self) -> &Vec<Vec<Option<(u8, u8)>>> {
        // Generate bottom left
        self.tiles[0][0] = self.scratch();

        let mut has_next = true;

        while has_next {
            self.update_constraint_map();

            if let Some(next) = self.find_lowest_entropy() {
                self.tiles[next.0][next.1] = self.collapse_tile(next);
            } else {
                has_next = false;
            }
        }

        &self.tiles
    }

    fn update_constraint_map(&mut self) {
        info!("Updating constraint map");

        for x in 0..CHUNK_TILE_LENGTH {
            for y in 0..CHUNK_TILE_LENGTH {
                if self.tiles[x as usize][y as usize].is_some() {
                    self.constatint_map[x as usize][y as usize].clear();
                    continue;
                }

                if x - 1 >= 0 {
                    if let Some(left) = self.tiles[(x - 1) as usize][y as usize] {
                        let allowed = self.schematic.tiles[&left.0.to_string()].east.clone();

                        self.constatint_map[x as usize][y as usize]
                            .retain(|&x| allowed.contains(&x));
                    }
                }

                if y - 1 >= 0 {
                    if let Some(down) = self.tiles[x as usize][(y - 1) as usize] {
                        let allowed = self.schematic.tiles[&down.0.to_string()].north.clone();

                        self.constatint_map[x as usize][y as usize]
                            .retain(|&x| allowed.contains(&x));
                    }
                }

                if x + 1 < CHUNK_TILE_LENGTH {
                    if let Some(right) = self.tiles[(x + 1) as usize][y as usize] {
                        let allowed = self.schematic.tiles[&right.0.to_string()].west.clone();

                        self.constatint_map[x as usize][y as usize]
                            .retain(|&x| allowed.contains(&x));
                    }
                }

                if y + 1 < CHUNK_TILE_LENGTH {
                    if let Some(up) = self.tiles[x as usize][(y + 1) as usize] {
                        let allowed = self.schematic.tiles[&up.0.to_string()].south.clone();

                        self.constatint_map[x as usize][y as usize]
                            .retain(|&x| allowed.contains(&x));
                    }
                }
            }
        }
    }

    // Finds lowest non-zero entry in constraint map and returns it's index.
    fn find_lowest_entropy(&self) -> Option<(usize, usize)> {
        info!("Calculating entropy low");

        let mut index = None;
        let mut lowest = 0;

        for x in 0..CHUNK_TILE_LENGTH {
            for y in 0..CHUNK_TILE_LENGTH {
                let n_constraints = self.constatint_map[x as usize][y as usize].len();
                if n_constraints > 0 && (lowest == 0 || n_constraints < lowest) {
                    lowest = n_constraints;
                    index = Some((x as usize, y as usize))
                }
            }
        }

        if index.is_some() {
            info!(
                "Entropy minima: ({}, {})",
                index.unwrap().0,
                index.unwrap().1
            );
        }

        index
    }

    // From scratch
    fn scratch(&self) -> Tile {
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.hash);
        Some((rng.gen_range(0..(self.schematic.tiles.len() as u8)), 1))
    }

    fn collapse_tile(&self, idx: (usize, usize)) -> Tile {
        info!("Collapsing tile");
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.hash);
        let available = self.constatint_map[idx.0][idx.1].clone();
        let rand = rng.gen_range(0..available.len() as u8);
        Some((available.iter().nth(rand.into()).unwrap().clone(), 1))
    }

    fn get_hash(world_seed: u64, coords: &Coords) -> u64 {
        let mut hasher = DefaultHasher::new();
        (coords.0 + coords.1 + world_seed as i64).hash(&mut hasher);
        hasher.finish()
    }
}
