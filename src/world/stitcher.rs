use std::collections::HashSet;

use bevy::{log::info, transform::components::Transform};

use crate::world::TILE_SIZE;

use super::{schematic::SchematicAsset, Adjacencies, ChunkCoords, Tile, CHUNK_TILE_LENGTH};

use rand::Rng;

pub struct Stitcher {
    coords: ChunkCoords,
    schematic: SchematicAsset,
    chunk: Vec<(Tile, Transform)>,
    adj: Adjacencies,
    constraint_map: Vec<HashSet<u8>>,
    tiles: Vec<Option<u8>>,
}

impl Stitcher {
    pub fn init(
        schematic: &SchematicAsset,
        coords: ChunkCoords,
        chunk: Vec<(Tile, Transform)>,
        adj: Adjacencies,
    ) -> Stitcher {
        Stitcher {
            coords: coords,
            schematic: schematic.clone(),
            chunk: chunk,
            adj: adj.clone(),
            constraint_map: Self::init_stitching_constaints(schematic, adj),
            tiles: vec![None; (4 * CHUNK_TILE_LENGTH + 4) as usize],
        }
    }

    pub fn stitch(&mut self) -> &Vec<Option<u8>> {
        // Collapse Chunk
        while let Some(next) = self.lowest_entropy() {
            self.tiles[next] = self.collapse_tile(next);
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
                            let allowed = self.schematic.tiles[&tile.texture_id].south.clone();

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
                            let allowed = self.schematic.tiles[&tile.texture_id].south.clone();

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
                            let allowed = self.schematic.tiles[&tile.texture_id].west.clone();

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
                            let allowed = self.schematic.tiles[&tile.texture_id].south.clone();

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
                            let allowed = self.schematic.tiles[&tile.texture_id].north.clone();

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
                            let allowed = self.schematic.tiles[&tile.texture_id].south.clone();

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
                            let allowed = self.schematic.tiles[&tile.texture_id].east.clone();

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
                            let allowed = self.schematic.tiles[&tile.texture_id].south.clone();

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
                            [&self.tiles[self.tiles.len() - 1].unwrap()]
                            .north
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx + 1].unwrap()]
                            .west
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                } else {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx - 1].unwrap()]
                            .east
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx + 1].unwrap()]
                            .west
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                }
            } else if side == 1 {
                if rank == 0 {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx - 1].unwrap()]
                            .north
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx + 1].unwrap()]
                            .north
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                } else {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx - 1].unwrap()]
                            .south
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx + 1].unwrap()]
                            .north
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                }
            } else if side == 1 {
                if rank == 0 {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx - 1].unwrap()]
                            .east
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx + 1].unwrap()]
                            .north
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                } else {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx - 1].unwrap()]
                            .south
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx + 1].unwrap()]
                            .north
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                }
            } else if side == 2 {
                if rank == 0 {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx - 1].unwrap()]
                            .south
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx + 1].unwrap()]
                            .east
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                } else {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx - 1].unwrap()]
                            .west
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx + 1].unwrap()]
                            .east
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                }
            } else if side == 3 {
                if rank == 0 {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx - 1].unwrap()]
                            .north
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[0].unwrap()].west.clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                } else if rank == CHUNK_TILE_LENGTH as usize {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx - 1].unwrap()]
                            .north
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[0].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[0].unwrap()].south.clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                } else {
                    if self.tiles[idx - 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx - 1].unwrap()]
                            .north
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }

                    if self.tiles[idx + 1].is_some() {
                        let allowed = self.schematic.tiles[&self.tiles[idx + 1].unwrap()]
                            .south
                            .clone();

                        constraint.retain(|&to_retain| allowed.contains(&to_retain));
                    }
                }
            }
        }
    }

    fn collapse_tile(&self, idx: usize) -> Option<u8> {
        info!("Collapsing stitched tile");
        let mut rng = rand::thread_rng();
        let available = self.constraint_map[idx].clone();
        let rand = rng.gen_range(0..available.len() as u8);
        Some(available.iter().nth(rand.into()).unwrap().clone())
    }

    fn init_stitching_constaints(schematic: &SchematicAsset, adj: Adjacencies) -> Vec<HashSet<u8>> {
        let unconstrained: HashSet<u8> = schematic.tiles.clone().into_keys().collect();
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
