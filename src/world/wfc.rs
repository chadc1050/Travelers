use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    hash::{Hash, Hasher},
};

use bevy::log::info;

use super::{schematic::SchematicAsset, ChunkCoords, CHUNK_TILE_LENGTH};

use rand::{Rng, SeedableRng};

pub struct WaveFunctionCollapse {
    hash: u64,
    schematic: SchematicAsset,
    constraint_map: Vec<Vec<HashSet<u8>>>,
    tiles: Vec<Vec<Option<u8>>>,
}

impl WaveFunctionCollapse {
    pub fn init(
        world_seed: u64,
        schematic: &SchematicAsset,
        coords: ChunkCoords,
    ) -> WaveFunctionCollapse {
        WaveFunctionCollapse {
            hash: Self::get_hash(world_seed, &coords),
            schematic: schematic.clone(),
            constraint_map: vec![
                vec![
                    schematic.tiles.clone().into_keys().collect();
                    CHUNK_TILE_LENGTH as usize
                ];
                CHUNK_TILE_LENGTH as usize
            ],
            tiles: vec![vec![None; CHUNK_TILE_LENGTH as usize]; CHUNK_TILE_LENGTH as usize],
        }
    }

    pub fn collapse(&mut self) -> &Vec<Vec<Option<u8>>> {
        // Generate bottom left of tile of chunk
        self.tiles[0][0] = self.scratch();

        // Collapse Chunk
        while let Some(next) = self.lowest_entropy() {
            self.tiles[next.0][next.1] = self.collapse_tile(next);
            self.update_constraint_map();
        }

        &self.tiles
    }

    fn update_constraint_map(&mut self) {
        info!("Updating constraint map");

        for x in 0..CHUNK_TILE_LENGTH {
            for y in 0..CHUNK_TILE_LENGTH {
                if self.tiles[x as usize][y as usize].is_some() {
                    self.constraint_map[x as usize][y as usize].clear();
                    continue;
                }

                if x - 1 >= 0 {
                    if let Some(left) = self.tiles[(x - 1) as usize][y as usize] {
                        let allowed = self.schematic.tiles[&left].east.clone();

                        self.constraint_map[x as usize][y as usize]
                            .retain(|&to_retain| allowed.contains(&to_retain));
                    }
                }

                if y - 1 >= 0 {
                    if let Some(down) = self.tiles[x as usize][(y - 1) as usize] {
                        let allowed = self.schematic.tiles[&down].north.clone();

                        self.constraint_map[x as usize][y as usize]
                            .retain(|&to_retain| allowed.contains(&to_retain));
                    }
                }

                if x + 1 < CHUNK_TILE_LENGTH {
                    if let Some(right) = self.tiles[(x + 1) as usize][y as usize] {
                        let allowed = self.schematic.tiles[&right].west.clone();

                        self.constraint_map[x as usize][y as usize]
                            .retain(|&to_retain| allowed.contains(&to_retain));
                    }
                }

                if y + 1 < CHUNK_TILE_LENGTH {
                    if let Some(up) = self.tiles[x as usize][(y + 1) as usize] {
                        let allowed = self.schematic.tiles[&up].south.clone();

                        self.constraint_map[x as usize][y as usize]
                            .retain(|&to_retain| allowed.contains(&to_retain));
                    }
                }
            }
        }
    }

    // Finds lowest non-zero entry in constraint map and returns it's index.
    fn lowest_entropy(&self) -> Option<(usize, usize)> {
        info!("Calculating chunk entropy low");

        let mut index = None;
        let mut lowest = 0;

        for x in 0..CHUNK_TILE_LENGTH {
            for y in 0..CHUNK_TILE_LENGTH {
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

    // From scratch
    fn scratch(&self) -> Option<u8> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.hash);

        let keys: Vec<u8> = self.schematic.tiles.clone().into_keys().collect();

        let idx = rng.gen_range(0..(keys.len() as u8));
        Some(keys[idx as usize])
    }

    fn collapse_tile(&self, idx: (usize, usize)) -> Option<u8> {
        info!("Collapsing tile");
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.hash);
        let available = self.constraint_map[idx.0][idx.1].clone();
        let rand = rng.gen_range(0..available.len() as u8);
        Some(available.iter().nth(rand.into()).unwrap().clone())
    }

    fn get_hash(world_seed: u64, coords: &ChunkCoords) -> u64 {
        let mut hasher = DefaultHasher::new();
        (coords.0 + coords.1 + world_seed as i64).hash(&mut hasher);
        hasher.finish()
    }
}
