use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    hash::{Hash, Hasher},
};

use bevy::log::{debug, info};

use crate::world::TILE_SIZE;

use super::{schematic::SchematicAsset, Adjacencies, Coords, CHUNK_TILE_LENGTH};

use rand::{Rng, SeedableRng};

// https://gist.github.com/jdah/ad997b858513a278426f8d91317115b9
// https://gamedev.stackexchange.com/questions/188719/deterministic-procedural-wave-function-collapse
pub struct WaveFunctionCollapse {
    hash: u64,
    coords: Coords,
    adj: Adjacencies,
    schematic: SchematicAsset,
    constraint_map: Vec<Vec<HashSet<u8>>>,
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
            coords: coords,
            adj: adj,
            schematic: schematic.clone(),
            constraint_map: vec![
                vec![
                    Self::init_constraints(schematic);
                    (CHUNK_TILE_LENGTH + 2) as usize
                ];
                (CHUNK_TILE_LENGTH + 2) as usize
            ],
            tiles: vec![
                vec![None; (CHUNK_TILE_LENGTH + 2) as usize];
                (CHUNK_TILE_LENGTH + 2) as usize
            ],
        }
    }

    pub fn collapse(&mut self) -> &Vec<Vec<Option<(u8, u8)>>> {
        // Generate bottom left of chunk
        self.tiles[1][1] = self.scratch();

        let mut has_next = true;

        // Collapse Chunk
        while has_next {
            if let Some(next) = self.find_chunk_lowest_entropy() {
                self.tiles[next.0][next.1] = self.collapse_chunk_tile(next);
            } else {
                has_next = false;
            }

            self.update_constraint_map();
        }

        has_next = true;

        // Collapse stitching
        while has_next {
            if let Some(next) = self.find_stitched_lowest_entropy() {
                self.tiles[next.0][next.1] = self.collapse_stitched_tile(next);
            } else {
                has_next = false;
            }

            self.update_constraint_map();
        }

        &self.tiles
    }

    fn init_constraints(schematic: SchematicAsset) -> HashSet<u8> {
        schematic
            .tiles
            .keys()
            .map(|key| key.parse::<u8>().unwrap())
            .collect()
    }

    fn update_constraint_map(&mut self) {
        info!("Updating constraint map");

        for x in 0..(CHUNK_TILE_LENGTH + 2) {
            for y in 0..(CHUNK_TILE_LENGTH + 2) {
                if self.tiles[x as usize][y as usize].is_some() {
                    self.constraint_map[x as usize][y as usize].clear();
                    continue;
                }

                if x - 1 >= 0 {
                    if let Some(left) = self.tiles[(x - 1) as usize][y as usize] {
                        let allowed = self.schematic.tiles[&left.0.to_string()].east.clone();

                        self.constraint_map[x as usize][y as usize]
                            .retain(|&to_retain| allowed.contains(&to_retain));
                    }
                } else if let Some(west) = &self.adj.3 {
                    for (tile, transform) in west.iter() {
                        // Convert tile to world coords
                        let x_world = self.coords.0 + (x * TILE_SIZE) - TILE_SIZE;
                        let y_world = self.coords.1 + (y * TILE_SIZE) - TILE_SIZE;

                        if (transform.translation.x - (TILE_SIZE as f32 / 2.)) as i64 + TILE_SIZE
                            == x_world
                            && (transform.translation.y - (TILE_SIZE as f32 / 2.)) as i64 == y_world
                        {
                            let allowed = self.schematic.tiles[&tile.texture_id.to_string()]
                                .east
                                .clone();

                            self.constraint_map[x as usize][y as usize]
                                .retain(|&to_retain| allowed.contains(&to_retain));
                        }
                    }
                } else {
                    // No chunk to stitch yet
                    self.constraint_map[x as usize][y as usize].clear();
                    continue;
                }

                if y - 1 >= 0 {
                    if let Some(down) = self.tiles[x as usize][(y - 1) as usize] {
                        let allowed = self.schematic.tiles[&down.0.to_string()].north.clone();

                        self.constraint_map[x as usize][y as usize]
                            .retain(|&to_retain| allowed.contains(&to_retain));
                    }
                } else if let Some(south) = &self.adj.2 {
                    for (tile, transform) in south.iter() {
                        // Convert tile to world coords
                        let x_world = self.coords.0 + (x * TILE_SIZE) - TILE_SIZE;
                        let y_world = self.coords.1 + (y * TILE_SIZE) - TILE_SIZE;

                        if (transform.translation.x - (TILE_SIZE as f32 / 2.)) as i64 == x_world
                            && (transform.translation.y - (TILE_SIZE as f32 / 2.)) as i64
                                + TILE_SIZE
                                == y_world
                        {
                            let allowed = self.schematic.tiles[&tile.texture_id.to_string()]
                                .north
                                .clone();

                            self.constraint_map[x as usize][y as usize]
                                .retain(|&to_retain| allowed.contains(&to_retain));
                        }
                    }
                } else {
                    // No chunk to stitch yet
                    self.constraint_map[x as usize][y as usize].clear();
                    continue;
                }

                if x + 1 < CHUNK_TILE_LENGTH {
                    if let Some(right) = self.tiles[(x + 1) as usize][y as usize] {
                        info!("{} {:?}", right.0, self.constraint_map);
                        let allowed = self.schematic.tiles[&right.0.to_string()].west.clone();

                        self.constraint_map[x as usize][y as usize]
                            .retain(|&to_retain| allowed.contains(&to_retain));
                    }
                } else if let Some(east) = &self.adj.1 {
                    for (tile, transform) in east.iter() {
                        // Convert tile to world coords
                        let x_world = self.coords.0 + (x * TILE_SIZE) - TILE_SIZE;
                        let y_world = self.coords.1 + (y * TILE_SIZE) - TILE_SIZE;

                        if (transform.translation.x - (TILE_SIZE as f32 / 2.)) as i64 - TILE_SIZE
                            == x_world
                            && (transform.translation.y - (TILE_SIZE as f32 / 2.)) as i64 == y_world
                        {
                            let allowed = self.schematic.tiles[&tile.texture_id.to_string()]
                                .west
                                .clone();

                            self.constraint_map[x as usize][y as usize]
                                .retain(|&to_retain| allowed.contains(&to_retain));
                        }
                    }
                } else {
                    // No chunk to stitch yet
                    self.constraint_map[x as usize][y as usize].clear();
                    continue;
                }

                if y + 1 < CHUNK_TILE_LENGTH {
                    if let Some(up) = self.tiles[x as usize][(y + 1) as usize] {
                        let allowed = self.schematic.tiles[&up.0.to_string()].south.clone();

                        self.constraint_map[x as usize][y as usize]
                            .retain(|&to_retain| allowed.contains(&to_retain));
                    }
                } else if let Some(north) = &self.adj.0 {
                    for (tile, transform) in north.iter() {
                        // Convert tile to world coords
                        let x_world = self.coords.0 + (x * TILE_SIZE) - TILE_SIZE;
                        let y_world = self.coords.1 + (y * TILE_SIZE) - TILE_SIZE;

                        if (transform.translation.x - (TILE_SIZE as f32 / 2.)) as i64 == x_world
                            && (transform.translation.y - (TILE_SIZE as f32 / 2.)) as i64
                                - TILE_SIZE
                                == y_world
                        {
                            let allowed = self.schematic.tiles[&tile.texture_id.to_string()]
                                .south
                                .clone();

                            self.constraint_map[x as usize][y as usize]
                                .retain(|&to_retain| allowed.contains(&to_retain));
                        }
                    }
                } else {
                    // No chunk to stitch yet
                    self.constraint_map[x as usize][y as usize].clear();
                    continue;
                }
            }
        }
    }

    // Finds lowest non-zero entry in constraint map and returns it's index.
    fn find_chunk_lowest_entropy(&self) -> Option<(usize, usize)> {
        info!("Calculating chunk entropy low");

        let mut index = None;
        let mut lowest = 0;

        for x in 1..=CHUNK_TILE_LENGTH {
            for y in 1..=CHUNK_TILE_LENGTH {
                let n_constraints = self.constraint_map[x as usize][y as usize].len();
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

    fn find_stitched_lowest_entropy(&self) -> Option<(usize, usize)> {
        info!("Calculating stitched entropy low");

        let mut index = None;
        let mut lowest = 0;

        for x in 0..(CHUNK_TILE_LENGTH + 2) {
            for y in 0..(CHUNK_TILE_LENGTH + 2) {
                // Only the perimeter
                if x == 0 || x == CHUNK_TILE_LENGTH + 1 || y == 0 || y == CHUNK_TILE_LENGTH + 1 {
                    let n_constraints = self.constraint_map[x as usize][y as usize].len();
                    if n_constraints > 0 && (lowest == 0 || n_constraints < lowest) {
                        lowest = n_constraints;
                        index = Some((x as usize, y as usize))
                    }
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
    fn scratch(&self) -> Option<(u8, u8)> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.hash);

        let keys: Vec<u8> = self
            .schematic
            .tiles
            .keys()
            .map(|key| key.parse::<u8>().unwrap())
            .collect();

        let idx = rng.gen_range(0..(keys.len() as u8));
        Some((keys[idx as usize], 1))
    }

    fn collapse_chunk_tile(&self, idx: (usize, usize)) -> Option<(u8, u8)> {
        info!("Collapsing tile");
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.hash);
        let available = self.constraint_map[idx.0][idx.1].clone();
        let rand = rng.gen_range(0..available.len() as u8);
        Some((available.iter().nth(rand.into()).unwrap().clone(), 1))
    }

    fn collapse_stitched_tile(&self, idx: (usize, usize)) -> Option<(u8, u8)> {
        info!("Collapsing stitched tile");
        let mut rng = rand::thread_rng();
        let available = self.constraint_map[idx.0][idx.1].clone();
        let rand = rng.gen_range(0..available.len() as u8);
        Some((available.iter().nth(rand.into()).unwrap().clone(), 1))
    }

    fn get_hash(world_seed: u64, coords: &Coords) -> u64 {
        let mut hasher = DefaultHasher::new();
        (coords.0 + coords.1 + world_seed as i64).hash(&mut hasher);
        hasher.finish()
    }
}
