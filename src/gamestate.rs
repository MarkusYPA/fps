use crate::AnimationState;
use crate::Input;
use crate::Sprite;
use crate::consts::{
    CAMERA_HEIGHT_OFFSET, PUDDLE_TIMEOUT, SHOT_MAX_DISTANCE, SPRITE_OTHER_PLAYER_HEIGHT,
    SPRITE_OTHER_PLAYER_WIDTH,
};
use crate::player::Player;
use crate::{consts::RESPAWN_DELAY, map::World};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, f32::MAX, time::Duration};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameState {
    pub players: HashMap<String, Player>,
    pub world: World,
    //pub sprites: Vec<Sprite>,
    sprite_id: u32,
    pub sprites: HashMap<u32, Sprite>,
    pub sprite_timeouts: HashMap<u32, Duration>,
}

impl GameState {
    pub fn new(map_identifier: Option<crate::flags::MapIdentifier>) -> Self {
        let world = match map_identifier {
            Some(crate::flags::MapIdentifier::Id(id)) => World::new(Some(id), None),
            Some(crate::flags::MapIdentifier::Name(name)) => World::new(Some(0), Some(&name)),
            None => World::new(Some(1), None),
        };

        GameState {
            players: HashMap::new(),
            world,
            //sprites: Vec::new(),
            sprite_id: 0,
            sprites: HashMap::new(),
            sprite_timeouts: HashMap::new(),
        }
    }

    pub fn add_puddle(&mut self, x: f32, y: f32) {
        let puddle = Sprite {
            x,
            y,
            z: -0.0325,
            texture: "puddle".to_string(),
            width: 0.3,
            height: 0.075,
        };
        //self.sprites.push(puddle);
        self.sprites.insert(self.sprite_id, puddle);
        self.sprite_timeouts.insert(self.sprite_id, PUDDLE_TIMEOUT);
        self.sprite_id += 1;

        println!("sprite inserted");
    }

    pub fn check_sprites(&mut self, dt: Duration) -> bool {
        let mut to_remove = Vec::new();
        let mut changed = false;

        // Iterate mutably but don't remove yet
        for (id, dur) in self.sprite_timeouts.iter_mut() {
            *dur = dur.saturating_sub(dt);

            if dur.is_zero() {
                to_remove.push(*id);
                changed = true;
            }
        }

        // Now remove outside the borrow
        for id in to_remove {
            self.sprites.remove(&id);
            self.sprite_timeouts.remove(&id);
            println!("sprite removed");
        }

        changed
    }

    pub fn update(&mut self, id: String, input: &Input, dt: Duration) -> bool {
        // generate respawn position before mutable borrow
        let respawn_pos = if self
            .players
            .get(&id)
            .map(|p| p.health == 0 && p.death_timer.is_zero())
            .unwrap_or(false)
        {
            Some(Player::get_random_spawn_point(&self.world))
        } else {
            None
        };

        let mut puddle_coordiantes = (0.0, 0.0);

        if let Some(player) = self.players.get_mut(&id) {
            player.take_input(input, &self.world);

            if player.dying {
                player.animation_state = AnimationState::Dying;
                player.death_timer = player.death_timer.saturating_sub(dt);
                if player.death_timer < RESPAWN_DELAY {
                    player.dying = false;
                    puddle_coordiantes = (player.x, player.y);
                }
            } else if player.health == 0 {
                player.animation_state = AnimationState::Dead;
                player.death_timer = player.death_timer.saturating_sub(dt);
                if player.death_timer.is_zero() {
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

        if puddle_coordiantes.0 != 0.0 {
            self.add_puddle(puddle_coordiantes.0, puddle_coordiantes.1);
            return true;
        }

        false
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
}
