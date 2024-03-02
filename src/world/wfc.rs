use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    hash::{Hash, Hasher},
};

use bevy::{
    log::{debug, info},
    transform::components::Transform,
};

use crate::world::TILE_SIZE;

use super::{schematic::SchematicAsset, Adjacencies, Coords, Tile, CHUNK_TILE_LENGTH};

use rand::{Rng, SeedableRng};

// https://gist.github.com/jdah/ad997b858513a278426f8d91317115b9
// https://gamedev.stackexchange.com/questions/188719/deterministic-procedural-wave-function-collapse
pub struct WaveFunctionCollapse {
    hash: u64,
    coords: Coords,
    schematic: SchematicAsset,
    constraint_map: Vec<Vec<HashSet<u8>>>,
    tiles: Vec<Vec<Option<(u8, u8)>>>,
}

impl WaveFunctionCollapse {
    pub fn init(
        world_seed: u64,
        schematic: &SchematicAsset,
        coords: Coords,
    ) -> WaveFunctionCollapse {
        WaveFunctionCollapse {
            hash: get_hash(world_seed, &coords),
            coords: coords,
            schematic: schematic.clone(),
            constraint_map: vec![
                vec![
                    init_constraints(schematic.clone());
                    CHUNK_TILE_LENGTH as usize
                ];
                CHUNK_TILE_LENGTH as usize
            ],
            tiles: vec![vec![None; CHUNK_TILE_LENGTH as usize]; CHUNK_TILE_LENGTH as usize],
        }
    }

    pub fn collapse(&mut self) -> &Vec<Vec<Option<(u8, u8)>>> {
        // Generate bottom left of chunk
        self.tiles[0][0] = self.scratch();

        let mut has_next = true;

        // Collapse Chunk
        while has_next {
            if let Some(next) = self.lowest_entropy() {
                self.tiles[next.0][next.1] = self.collapse_tile(next);
            } else {
                has_next = false;
            }

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
                        let allowed = self.schematic.tiles[&left.0.to_string()].east.clone();

                        self.constraint_map[x as usize][y as usize]
                            .retain(|&to_retain| allowed.contains(&to_retain));
                    }
                }

                if y - 1 >= 0 {
                    if let Some(down) = self.tiles[x as usize][(y - 1) as usize] {
                        let allowed = self.schematic.tiles[&down.0.to_string()].north.clone();

                        self.constraint_map[x as usize][y as usize]
                            .retain(|&to_retain| allowed.contains(&to_retain));
                    }
                }

                if x + 1 < CHUNK_TILE_LENGTH {
                    if let Some(right) = self.tiles[(x + 1) as usize][y as usize] {
                        let allowed = self.schematic.tiles[&right.0.to_string()].west.clone();

                        self.constraint_map[x as usize][y as usize]
                            .retain(|&to_retain| allowed.contains(&to_retain));
                    }
                }

                if y + 1 < CHUNK_TILE_LENGTH {
                    if let Some(up) = self.tiles[x as usize][(y + 1) as usize] {
                        let allowed = self.schematic.tiles[&up.0.to_string()].south.clone();

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

    fn collapse_tile(&self, idx: (usize, usize)) -> Option<(u8, u8)> {
        info!("Collapsing tile");
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.hash);
        let available = self.constraint_map[idx.0][idx.1].clone();
        let rand = rng.gen_range(0..available.len() as u8);
        Some((available.iter().nth(rand.into()).unwrap().clone(), 1))
    }
}

pub struct Stitcher {
    hash: u64,
    coords: Coords,
    schematic: SchematicAsset,
    chunk: Vec<(Tile, Transform)>,
    adj: Adjacencies,
    constraint_map: Vec<HashSet<u8>>,
    tiles: Vec<Option<(u8, u8)>>,
}

impl Stitcher {
    pub fn init(
        world_seed: u64,
        schematic: &SchematicAsset,
        coords: Coords,
        chunk: Vec<(Tile, Transform)>,
        adj: Adjacencies,
    ) -> Stitcher {
        Stitcher {
            hash: get_hash(world_seed, &coords),
            coords: coords,
            schematic: schematic.clone(),
            chunk: chunk,
            adj: adj.clone(),
            constraint_map: Stitcher::init_stitching_constaints(schematic, adj),
            tiles: vec![None; (4 * CHUNK_TILE_LENGTH + 4) as usize],
        }
    }

    pub fn stitch(&mut self) -> &Vec<Option<(u8, u8)>> {
        let mut has_next = true;

        // Collapse Chunk
        while has_next {
            if let Some(next) = self.lowest_entropy() {
                self.tiles[next] = self.collapse_tile(next);
            } else {
                has_next = false;
            }

            self.update_constraint_map();
        }

        info!("{:?}", self.tiles);
        &self.tiles
    }

    fn lowest_entropy(&self) -> Option<usize> {
        info!("Calculating stitched entropy low");

        let mut index = None;
        let mut lowest = 0;

        for (idx, constraint) in self.constraint_map.iter().enumerate() {
            let n_constraints = constraint.len();
            if n_constraints > 0 && (lowest == 0 || n_constraints < lowest) {
                lowest = n_constraints;
                index = Some(idx);
            }
        }

        if index.is_some() {
            //info!("{:?}\n{:?}", self.constraint_map, self.adj);
            info!("Entropy minima: ({})", index.unwrap());
        }

        index
    }

    // Checks for chunk adjacencies, connected adjacencies and stitched ajacencies
    fn update_constraint_map(&mut self) {
        for (idx, constraint) in self.constraint_map.iter_mut().enumerate() {
            if constraint.is_empty() {
                continue;
            }

            if self.tiles[idx].is_some() {
                constraint.clear();
                continue;
            }

            let side = idx / (CHUNK_TILE_LENGTH + 1) as usize;

            let rank = idx % (CHUNK_TILE_LENGTH + 1) as usize;

            // Check chunk and connecting chunks
            if side == 0 || (side == 1 && rank == 0) {
                if let Some(north) = &self.adj.0 {
                    let perim_world_coords =
                        super::get_perimeter_world_coord(&self.coords, side as i64, rank as i64);

                    for (tile, transform) in north.iter() {
                        // Convert tile to world coords
                        if (transform.translation.x - (TILE_SIZE as f32 / 2.)) as i64
                            == perim_world_coords.0
                            && (transform.translation.y - (TILE_SIZE as f32 / 2.)) as i64
                                - TILE_SIZE
                                == perim_world_coords.1
                        {
                            let allowed = self.schematic.tiles[&tile.texture_id.to_string()]
                                .south
                                .clone();

                            constraint.retain(|&to_retain| allowed.contains(&to_retain));
                        }
                    }
                }

                if rank != 0 {
                    // Not a corner, check the chunk
                    for (tile, transform) in self.chunk.iter() {
                        let perim_world_coords = super::get_perimeter_world_coord(
                            &self.coords,
                            side as i64,
                            rank as i64,
                        );

                        if (transform.translation.x - (TILE_SIZE as f32 / 2.)) as i64
                            == perim_world_coords.0
                            && (transform.translation.y - (TILE_SIZE as f32 / 2.)) as i64
                                + TILE_SIZE
                                == perim_world_coords.1
                        {
                            let allowed = self.schematic.tiles[&tile.texture_id.to_string()]
                                .south
                                .clone();

                            constraint.retain(|&to_retain| allowed.contains(&to_retain));
                        }
                    }
                }
            } else if side == 1 || (side == 2 && rank == 0) {
                if let Some(east) = &self.adj.1 {
                    let perim_world_coords =
                        super::get_perimeter_world_coord(&self.coords, side as i64, rank as i64);

                    for (tile, transform) in east.iter() {
                        // Convert tile to world coords
                        if (transform.translation.x - (TILE_SIZE as f32 / 2.)) as i64 - TILE_SIZE
                            == perim_world_coords.0
                            && (transform.translation.y - (TILE_SIZE as f32 / 2.)) as i64
                                == perim_world_coords.1
                        {
                            let allowed = self.schematic.tiles[&tile.texture_id.to_string()]
                                .west
                                .clone();

                            constraint.retain(|&to_retain| allowed.contains(&to_retain));
                        }
                    }
                }

                if rank != 0 {
                    // Not a corner, check the chunk
                    for (tile, transform) in self.chunk.iter() {
                        let perim_world_coords = super::get_perimeter_world_coord(
                            &self.coords,
                            side as i64,
                            rank as i64,
                        );

                        if (transform.translation.x - (TILE_SIZE as f32 / 2.)) as i64 + TILE_SIZE
                            == perim_world_coords.0
                            && (transform.translation.y - (TILE_SIZE as f32 / 2.)) as i64
                                == perim_world_coords.1
                        {
                            let allowed = self.schematic.tiles[&tile.texture_id.to_string()]
                                .south
                                .clone();

                            constraint.retain(|&to_retain| allowed.contains(&to_retain));
                        }
                    }
                }
            } else if side == 2 || (side == 3 && rank == 0) {
                if let Some(south) = &self.adj.2 {
                    let perim_world_coords =
                        super::get_perimeter_world_coord(&self.coords, side as i64, rank as i64);

                    for (tile, transform) in south.iter() {
                        // Convert tile to world coords
                        if (transform.translation.x - (TILE_SIZE as f32 / 2.)) as i64
                            == perim_world_coords.0
                            && (transform.translation.y - (TILE_SIZE as f32 / 2.)) as i64
                                + TILE_SIZE
                                == perim_world_coords.1
                        {
                            let allowed = self.schematic.tiles[&tile.texture_id.to_string()]
                                .north
                                .clone();

                            constraint.retain(|&to_retain| allowed.contains(&to_retain));
                        }
                    }
                }

                if rank != 0 {
                    // Not a corner, check the chunk
                    for (tile, transform) in self.chunk.iter() {
                        let perim_world_coords = super::get_perimeter_world_coord(
                            &self.coords,
                            side as i64,
                            rank as i64,
                        );

                        if (transform.translation.x - (TILE_SIZE as f32 / 2.)) as i64
                            == perim_world_coords.0
                            && (transform.translation.y - (TILE_SIZE as f32 / 2.)) as i64
                                - TILE_SIZE
                                == perim_world_coords.1
                        {
                            let allowed = self.schematic.tiles[&tile.texture_id.to_string()]
                                .south
                                .clone();

                            constraint.retain(|&to_retain| allowed.contains(&to_retain));
                        }
                    }
                }
            } else if side == 3 || (side == 0 && rank == 0) {
                if let Some(west) = &self.adj.3 {
                    let perim_world_coords =
                        super::get_perimeter_world_coord(&self.coords, side as i64, rank as i64);

                    for (tile, transform) in west.iter() {
                        // Convert tile to world coords
                        if (transform.translation.x - (TILE_SIZE as f32 / 2.)) as i64
                            == perim_world_coords.0 + TILE_SIZE
                            && (transform.translation.y - (TILE_SIZE as f32 / 2.)) as i64
                                == perim_world_coords.1
                        {
                            let allowed = self.schematic.tiles[&tile.texture_id.to_string()]
                                .east
                                .clone();

                            constraint.retain(|&to_retain| allowed.contains(&to_retain));
                        }
                    }
                }

                if rank != 0 {
                    // Not a corner, check the chunk
                    for (tile, transform) in self.chunk.iter() {
                        let perim_world_coords = super::get_perimeter_world_coord(
                            &self.coords,
                            side as i64,
                            rank as i64,
                        );

                        if (transform.translation.x - (TILE_SIZE as f32 / 2.)) as i64 - TILE_SIZE
                            == perim_world_coords.0
                            && (transform.translation.y - (TILE_SIZE as f32 / 2.)) as i64
                                == perim_world_coords.1
                        {
                            let allowed = self.schematic.tiles[&tile.texture_id.to_string()]
                                .south
                                .clone();

                            constraint.retain(|&to_retain| allowed.contains(&to_retain));
                        }
                    }
                }
            }

            // Check before and after idx
            if side == 0 {
                if rank == 0 {
                    if self.tiles[self.tiles.len() - 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[self.tiles.len() - 1].unwrap().0.to_string()]
                            .north
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx + 1].unwrap().0.to_string()]
                            .west
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                } else {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx - 1].unwrap().0.to_string()]
                            .east
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx + 1].unwrap().0.to_string()]
                            .west
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                }
            } else if side == 1 {
                if rank == 0 {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx - 1].unwrap().0.to_string()]
                            .north
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx + 1].unwrap().0.to_string()]
                            .north
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                } else {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx - 1].unwrap().0.to_string()]
                            .south
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx + 1].unwrap().0.to_string()]
                            .north
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                }
            } else if side == 1 {
                if rank == 0 {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx - 1].unwrap().0.to_string()]
                            .east
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx + 1].unwrap().0.to_string()]
                            .north
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                } else {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx - 1].unwrap().0.to_string()]
                            .south
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx + 1].unwrap().0.to_string()]
                            .north
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                }
            } else if side == 2 {
                if rank == 0 {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx - 1].unwrap().0.to_string()]
                            .south
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx + 1].unwrap().0.to_string()]
                            .east
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                } else {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx - 1].unwrap().0.to_string()]
                            .west
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx + 1].unwrap().0.to_string()]
                            .east
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                }
            } else if side == 3 {
                if rank == 0 {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx - 1].unwrap().0.to_string()]
                            .north
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[0].unwrap().0.to_string()]
                            .west
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                } else if rank == CHUNK_TILE_LENGTH as usize {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx - 1].unwrap().0.to_string()]
                            .north
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[0].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[0].unwrap().0.to_string()]
                            .south
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                } else {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx - 1].unwrap().0.to_string()]
                            .north
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles
                            [&self.tiles[idx + 1].unwrap().0.to_string()]
                            .south
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                }
            }
        }
    }

    fn collapse_tile(&self, idx: usize) -> Option<(u8, u8)> {
        info!("Collapsing stitched tile");
        let mut rng = rand::thread_rng();
        let available = self.constraint_map[idx].clone();
        let rand = rng.gen_range(0..available.len() as u8);
        Some((available.iter().nth(rand.into()).unwrap().clone(), 1))
    }

    fn init_stitching_constaints(schematic: &SchematicAsset, adj: Adjacencies) -> Vec<HashSet<u8>> {
        let unconstrained = init_constraints(schematic.clone());
        let mut constraints = vec![HashSet::new(); (4 * CHUNK_TILE_LENGTH + 4) as usize];

        for idx in 0..(4 * CHUNK_TILE_LENGTH + 4) {
            let side = idx / (CHUNK_TILE_LENGTH + 1);

            let rank = idx % (CHUNK_TILE_LENGTH + 1);

            if adj.0.is_some() && (side == 0 || (side == 1 && rank == 0)) {
                constraints[idx as usize] = unconstrained.clone();
            } else if adj.1.is_some() && (side == 1 || (side == 2 && rank == 0)) {
                constraints[idx as usize] = unconstrained.clone();
            } else if adj.2.is_some() && (side == 2 || (side == 3 && rank == 0)) {
                constraints[idx as usize] = unconstrained.clone();
            } else if adj.3.is_some() && (side == 3 || (side == 0 && rank == 0)) {
                constraints[idx as usize] = unconstrained.clone();
            }
        }

        constraints
    }
}

fn init_constraints(schematic: SchematicAsset) -> HashSet<u8> {
    // TODO: This can be simplified if the schematic is serialized to u8 rather than String value
    schematic
        .tiles
        .keys()
        .map(|key| key.parse::<u8>().unwrap())
        .collect()
}

fn get_hash(world_seed: u64, coords: &Coords) -> u64 {
    let mut hasher = DefaultHasher::new();
    (coords.0 + coords.1 + world_seed as i64).hash(&mut hasher);
    hasher.finish()
}
