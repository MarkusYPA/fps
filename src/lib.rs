use crate::player::Player;
use crate::{consts::RESPAWN_DELAY, map::World};
use rand::{Rng, rng};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, f32::MAX, time::Duration};

use crate::consts::{
    CAMERA_HEIGHT_OFFSET, SHOT_MAX_DISTANCE, SPRITE_NPC_HEIGHT, SPRITE_NPC_WIDTH,
    SPRITE_OTHER_PLAYER_HEIGHT, SPRITE_OTHER_PLAYER_WIDTH,
};

pub mod consts;
pub mod flags;
pub mod map;
pub mod minimap;
pub mod player;
pub mod renderer;
pub mod spritesheet;
pub mod textures;

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessage {
    Connect(String),
    Input(Input),
    Ping,
    Shot,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage {
    Welcome(Welcome),
    GameUpdate(HashMap<String, PlayerUpdate>),
    InitialState(GameState),
    UsernameRejected(String),
    PlayerLeft(u64),
    ShotHit(Hit),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hit {
    pub shooter_id: u64,
    pub shooter_name: String,
    pub target_id: u64,
    pub target_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Welcome {
    pub id: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AnimationState {
    Idle,
    Walking,
    Shooting,
    Dying,
    Dead,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Direction {
    Front,
    FrontRight,
    Right,
    BackRight,
    Back,
    BackLeft,
    Left,
    FrontLeft,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerUpdate {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub angle: f32,
    pub pitch: f32,
    pub texture: String,
    pub animation_state: AnimationState,
    pub shooting: bool,
    pub health: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Input {
    pub forth: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub turn: f32,
    pub pitch: f32,
    pub jump: bool,
    pub sprint: bool,
    pub shoot: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sprite {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub texture: String,
    pub width: f32,
    pub height: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameState {
    pub players: HashMap<String, Player>,
    pub world: World,
    pub sprites: Vec<Sprite>,
}

impl GameState {
    pub fn new(map_identifier: Option<crate::flags::MapIdentifier>) -> Self {
        let world = match map_identifier {
            Some(crate::flags::MapIdentifier::Id(id)) => World::new(Some(id), None),
            Some(crate::flags::MapIdentifier::Name(name)) => World::new(Some(0), Some(&name)),
            None => World::new(Some(1), None),
        };
        let sprites = vec![
            Sprite {
                x: 3.2,
                y: 4.3,
                z: 0.0,
                texture: "character2".to_string(),
                width: SPRITE_NPC_WIDTH,
                height: SPRITE_NPC_HEIGHT,
            },
            Sprite {
                x: 4.2,
                y: 4.3,
                z: 0.0,
                texture: "character3".to_string(),
                width: SPRITE_NPC_WIDTH,
                height: SPRITE_NPC_HEIGHT,
            },
        ];
        GameState {
            players: HashMap::new(),
            world,
            sprites,
        }
    }

    pub fn update(&mut self, id: String, input: &Input, dt: Duration) {

        // generate respawn position before mutable borrow
        let respawn_pos = if self
            .players
            .get(&id)
            .map(|p| p.health == 0 && p.death_timer.is_zero())
            .unwrap_or(false)
        {
            Some(self.random_square())
        } else {
            None
        };

        if let Some(player) = self.players.get_mut(&id) {
            player.take_input(input, &self.world);

            if player.dying {
                player.animation_state = AnimationState::Dying;
                player.death_timer = player.death_timer.saturating_sub(dt);
                if player.death_timer < RESPAWN_DELAY {
                    player.dying = false;
                }
            } else if player.health == 0 {
                player.animation_state = AnimationState::Dead;
                player.death_timer = player.death_timer.saturating_sub(dt);
                if player.death_timer.is_zero() {
                    // respawn player with gamestate?
                    println!("respawn player now?");
                    if let Some((map_x, map_y)) = respawn_pos {
                        player.respawn(map_x, map_y);
                    }
                }
            } else if player.shooting {
                player.animation_state = AnimationState::Shooting;
                player.shoot_timer = player.shoot_timer.saturating_sub(dt);
                if player.shoot_timer.is_zero() {
                    player.shooting = false;
                }
            } else if input.forth || input.back || input.left || input.right {
                player.animation_state = AnimationState::Walking;
            } else {
                player.animation_state = AnimationState::Idle;
            }
        }
    }

    pub fn measure_shot(&self, shooter_id: &u64) -> Option<u64> {
        if let Some(shooter) = self.players.get(&shooter_id.to_string()) {
            if shooter.health == 0 {
                return None;
            }

            let shot_dir_x = shooter.angle.cos();
            let shot_dir_y = shooter.angle.sin();

            let wall_dist_sq = self.nearest_wall_distance_squared(shooter, shot_dir_x, shot_dir_y);
            let mut closest_hit_distance: f32 = MAX;
            let mut target_id_opt = None;

            for (target_id_str, target) in &self.players {
                if &shooter_id.to_string() != target_id_str {
                    let dx = target.x - shooter.x;
                    let dy = target.y - shooter.y;
                    let dist_sq = dx * dx + dy * dy;

                    if dist_sq < wall_dist_sq && dist_sq < SHOT_MAX_DISTANCE {
                        // Calculate the dot product of the vector from shooter to target and the shot direction.
                        // A positive dot product means the target is generally in front of the shooter.
                        let dot = dx * shot_dir_x + dy * shot_dir_y;
                        if dot > 0.0 {
                            // Squared length of the projection of the shooter-to-target vector onto the shot direction vector.
                            // How far along the shot's path the target is.
                            let proj_len_sq =
                                dot * dot / (shot_dir_x * shot_dir_x + shot_dir_y * shot_dir_y);

                            // Squared perpendicular distance from the target to the shot ray: how far off-axis the target is from the shot's line of fire.
                            let perp_dist_sq = dist_sq - proj_len_sq;

                            let target_width = SPRITE_OTHER_PLAYER_WIDTH * 0.5; // Player hitbox width
                            if perp_dist_sq < target_width * target_width {
                                // Vertical check
                                let dist = dist_sq.sqrt();
                                let shot_height_at_target =
                                    shooter.z + CAMERA_HEIGHT_OFFSET + shooter.pitch * dist * 0.5; // pitch is a vertical offset, not an angle

                                // Corpse lies low
                                let target_height = if target.health == 0 {
                                    SPRITE_OTHER_PLAYER_HEIGHT * 0.4
                                } else {
                                    SPRITE_OTHER_PLAYER_HEIGHT
                                };

                                // Shot hits someone
                                if shot_height_at_target > target.z - 0.5
                                    && shot_height_at_target < target.z + target_height - 0.5
                                {
                                    let target_id = target_id_str.parse::<u64>().unwrap();

                                    // Update closest hit so far
                                    if dist < closest_hit_distance {
                                        closest_hit_distance = dist;
                                        target_id_opt = Some(target_id);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            return target_id_opt;
        }
        None
    }

    fn nearest_wall_distance_squared(&self, player: &Player, dir_x: f32, dir_y: f32) -> f32 {
        // Map position
        let mut map_x = player.x as isize;
        let mut map_y = player.y as isize;

        // Delta distance for each step
        let delta_dist_x = if dir_x == 0.0 {
            f32::INFINITY
        } else {
            (1.0 + (dir_y / dir_x).powi(2)).sqrt()
        };
        let delta_dist_y = if dir_y == 0.0 {
            f32::INFINITY
        } else {
            (1.0 + (dir_x / dir_y).powi(2)).sqrt()
        };

        // Step and initial sideDist
        let (step_x, mut side_dist_x) = if dir_x < 0.0 {
            (-1, (player.x - map_x as f32) * delta_dist_x)
        } else {
            (1, (map_x as f32 + 1.0 - player.x) * delta_dist_x)
        };

        let (step_y, mut side_dist_y) = if dir_y < 0.0 {
            (-1, (player.y - map_y as f32) * delta_dist_y)
        } else {
            (1, (map_y as f32 + 1.0 - player.y) * delta_dist_y)
        };

        // Perform Digital Differential Analyzer
        let mut hit = false;
        let mut wall_type = 0;
        while !hit {
            if side_dist_x < side_dist_y {
                side_dist_x += delta_dist_x;
                map_x += step_x;
                wall_type = 0;
            } else {
                side_dist_y += delta_dist_y;
                map_y += step_y;
                wall_type = 1;
            }

            if self.world.get_tile(map_x as usize, map_y as usize) > 0 {
                hit = true;
            }
        }

        // Hit distance along ray
        let distance = if wall_type == 0 {
            side_dist_x - delta_dist_x
        } else {
            side_dist_y - delta_dist_y
        };

        distance * distance
    }

    fn random_square(&self) -> (usize, usize) {
        let mut rng = rng();

        // Collect all coordinates where the map has a 0
        let open_tiles: Vec<(usize, usize)> = self
            .world
            .map
            .iter()
            .enumerate()
            .flat_map(|(y, row)| {
                row.iter()
                    .enumerate()
                    .filter_map(move |(x, &tile)| if tile == 0 { Some((x, y)) } else { None })
            })
            .collect();

        // If no open tiles exist, return (1,1) or handle appropriately
        if open_tiles.is_empty() {
            return (1, 1);
        }

        // Pick a random open tile
        let index = rng.random_range(0..open_tiles.len());
        open_tiles[index]
    }
}
