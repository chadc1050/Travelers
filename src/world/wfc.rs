use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    hash::{Hash, Hasher},
};

use bevy::log::{debug, info};

use super::{schematic::SchematicAsset, Adjacencies, Coords, CHUNK_TILE_LENGTH};

use rand::{Rng, SeedableRng};

// https://gist.github.com/jdah/ad997b858513a278426f8d91317115b9
// https://gamedev.stackexchange.com/questions/188719/deterministic-procedural-wave-function-collapse
pub struct WaveFunctionCollapse {
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
        debug!("Updating constraint map");

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
        debug!("Calculating entropy low");

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
            debug!(
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
        Some((rng.gen_range(0..(self.schematic.tiles.len() as u8)), 1))
    }

    fn collapse_tile(&self, idx: (usize, usize)) -> Option<(u8, u8)> {
        debug!("Collapsing tile");
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
