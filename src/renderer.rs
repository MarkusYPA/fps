use crate::{GameState, HEIGHT, WIDTH};

pub struct Renderer {
    buffer: Vec<u32>,
}

impl Renderer {
    pub fn new() -> Self {
        Renderer {
            buffer: vec![0; WIDTH * HEIGHT],
        }
    }

    pub fn render(&mut self, game_state: &GameState, my_id: u64) {
        // Clear the buffer with ceiling and floor colors
        for y in 0..HEIGHT / 2 {
            for x in 0..WIDTH {
                self.buffer[y * WIDTH + x] = 0x00AACCFF; // Ceiling
            }
        }
        for y in HEIGHT / 2..HEIGHT {
            for x in 0..WIDTH {
                self.buffer[y * WIDTH + x] = 0x00555555; // Floor
            }
        }

        if let Some(player) = game_state.players.get(&my_id.to_string()) {
            let pitch_offset = (player.pitch * HEIGHT as f32 / 2.0) as isize;
            let horizon = (HEIGHT as isize / 2 + pitch_offset).clamp(0, HEIGHT as isize) as usize;

            // Clear the buffer with ceiling and floor colors
            for y in 0..horizon {
                for x in 0..WIDTH {
                    self.buffer[y * WIDTH + x] = 0x00AACCFF; // Ceiling
                }
            }
            for y in horizon..HEIGHT {
                for x in 0..WIDTH {
                    self.buffer[y * WIDTH + x] = 0x00555555; // Floor
                }
            }
            for x in 0..WIDTH {
                let camera_x = 2.0 * x as f32 / WIDTH as f32 - 1.0;
                let ray_dir_x = player.angle.cos() + 0.66 * camera_x * (-player.angle.sin());
                let ray_dir_y = player.angle.sin() + 0.66 * camera_x * player.angle.cos();

                let mut map_x = player.x as usize;
                let mut map_y = player.y as usize;

                let delta_dist_x = (1.0f32 + (ray_dir_y / ray_dir_x).powi(2)).sqrt();
                let delta_dist_y = (1.0f32 + (ray_dir_x / ray_dir_y).powi(2)).sqrt();

                let step_x;
                let step_y;
                let mut wall_dist_x;
                let mut wall_dist_y;

                if ray_dir_x < 0.0 {
                    step_x = -1;
                    wall_dist_x = (player.x - map_x as f32) * delta_dist_x;
                } else {
                    step_x = 1;
                    wall_dist_x = (map_x as f32 + 1.0 - player.x) * delta_dist_x;
                }
                if ray_dir_y < 0.0 {
                    step_y = -1;
                    wall_dist_y = (player.y - map_y as f32) * delta_dist_y;
                } else {
                    step_y = 1;
                    wall_dist_y = (map_y as f32 + 1.0 - player.y) * delta_dist_y;
                }

                let mut hit = false;
                let mut wall_type = 0;
                while !hit {
                    if wall_dist_x < wall_dist_y {
                        wall_dist_x += delta_dist_x;
                        map_x = (map_x as isize + step_x) as usize;
                        wall_type = 0;
                    } else {
                        wall_dist_y += delta_dist_y;
                        map_y = (map_y as isize + step_y) as usize;
                        wall_type = 1;
                    }

                    if game_state.world.get_tile(map_x, map_y) == 1 {
                        hit = true;
                    }
                }

                let perp_wall_dist = if wall_type == 0 {
                    (map_x as f32 - player.x + (1.0 - step_x as f32) / 2.0) / ray_dir_x
                } else {
                    (map_y as f32 - player.y + (1.0 - step_y as f32) / 2.0) / ray_dir_y
                };

                let line_height = (HEIGHT as f32 / perp_wall_dist) as isize;
                let draw_start = (-line_height / 2 + HEIGHT as isize / 2 + pitch_offset)
                    .clamp(0, HEIGHT as isize - 1) as usize;
                let draw_end = (line_height / 2 + HEIGHT as isize / 2 + pitch_offset)
                    .clamp(0, HEIGHT as isize) as usize;

                let wall_color = if wall_type == 1 {
                    0x008A7755
                } else {
                    0x00695A41
                };

                for y in draw_start..draw_end {
                    self.buffer[y * WIDTH + x] = wall_color;
                }
            }
        }
    }

    pub fn draw_to_buffer(&self, frame: &mut [u8]) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let color = self.buffer[i];
            let rgba = [(color >> 16) as u8, (color >> 8) as u8, color as u8, 0xFF];
            pixel.copy_from_slice(&rgba);
        }
    }
}
