use crate::consts::{DEFAULT_MAP_HEIGHT, DEFAULT_MAP_INCLUDE_CORNERS, DEFAULT_MAP_WIDTH};
use crate::utils::carve_path;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct World {
    pub map: Vec<Vec<u8>>,
}

impl World {
    pub fn new(
        id: Option<usize>,
        name: Option<&str>,
        random: bool,
        width: Option<usize>,
        height: Option<usize>,
    ) -> Self {
        let map_id = id.unwrap_or(1);
        let map_name = name.unwrap_or("map1");
        if random {
            let x_size = width.unwrap_or(DEFAULT_MAP_WIDTH);
            let y_size = height.unwrap_or(DEFAULT_MAP_HEIGHT);
            Self::generate_random_map(x_size, y_size)
        } else {
            match map_id {
                0 => Self::parse_from_file(&format!("maps/{}.toml", map_name)),
                1 => Self::parse_from_file("maps/map1.toml"),
                2 => Self::parse_from_file("maps/map2.toml"),
                3 => Self::parse_from_file("maps/map3.toml"),
                _ => panic!("Invalid map id: {}", map_id),
            }
        }
    }

    pub fn parse_from_file(path: &str) -> Self {
        let contents = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Failed to read map file {}: {}", path, e));
        let world: Self = toml::from_str(&contents)
            .unwrap_or_else(|e| panic!("Failed to parse TOML map file {}: {}", path, e));
        world
    }

    pub fn generate_random_map(x_size: usize, y_size: usize) -> Self {
        let mut world = World { map: vec![vec![1; x_size]; y_size] };
        let current_tile = (x_size / 2, y_size / 2);

        carve_path(&mut world, current_tile, DEFAULT_MAP_INCLUDE_CORNERS, None);

        println!("Generated random map: ");
        for y in 0..world.map.len() {
            for x in 0..world.map[y].len() {
                print!("{}", world.map[y][x]);
            }
            println!();
        };

        world
    }

    pub fn get_tile(&self, y: usize, x: usize) -> u8 {
        if self.map.is_empty() {
            return 1;
        }

        if y >= self.map.len() {
            return 1;
        }

        let row = &self.map[y];

        if x >= row.len() {
            return 1;
        }

        row[x]
    }
}
