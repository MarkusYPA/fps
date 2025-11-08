use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct World {
    pub map: Vec<Vec<u8>>,
}

impl World {
    pub fn new(id: Option<usize>, name: Option<&str>) -> Self {
        let map_id = id.unwrap_or(1);
        let map_name = name.unwrap_or("map1");
        match map_id {
            0 => Self::parse_from_file(&format!("maps/{}.toml", map_name)),
            1 => Self::parse_from_file("maps/map1.toml"),
            2 => Self::parse_from_file("maps/map2.toml"),
            3 => Self::parse_from_file("maps/map3.toml"),
            _ => panic!("Invalid map id: {}", map_id),
        }
    }

    pub fn parse_from_file(path: &str) -> Self {
        let contents = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Failed to read map file {}: {}", path, e));
        let world: Self = toml::from_str(&contents)
            .unwrap_or_else(|e| panic!("Failed to parse TOML map file {}: {}", path, e));
        world
    }

    // Unused for now
/*     pub fn generate_random_map() -> Self {
        let mut map = Vec::new();
        for _ in 0..10 {
            let mut row = Vec::new();
            for _ in 0..10 {
                row.push(rand::rng().random_range(0..2));
            }
            map.push(row);
        }
        Self { map }
    } */

    pub fn get_tile(&self, x: usize, y: usize) -> u8 {
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
