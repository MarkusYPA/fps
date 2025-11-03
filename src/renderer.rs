use crate::{GameState, HEIGHT, WIDTH, textures::TextureManager};

pub struct Renderer {
    buffer: Vec<u32>,
    z_buffer: Vec<f32>,
    texture_manager: TextureManager,
}

impl Renderer {
    pub fn new(texture_manager: TextureManager) -> Self {
        Renderer {
            buffer: vec![0; WIDTH * HEIGHT],
            z_buffer: vec![0.0; WIDTH],
            texture_manager,
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

                self.z_buffer[x] = perp_wall_dist;

                let line_height = (HEIGHT as f32 / perp_wall_dist) as isize;
                let z_offset = (player.z * line_height as f32) as isize;
                let draw_start = (-line_height / 2 + HEIGHT as isize / 2 + pitch_offset + z_offset)
                    .clamp(0, HEIGHT as isize - 1) as usize;
                let draw_end = (line_height / 2 + HEIGHT as isize / 2 + pitch_offset + z_offset)
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

            // Sprite rendering

            // Sort sprites by distance
            let mut sprites_with_dist: Vec<_> = game_state
                .sprites
                .iter()
                .map(|sprite| {
                    let sprite_x = sprite.x - player.x;
                    let sprite_y = sprite.y - player.y;
                    let dist_sq = sprite_x * sprite_x + sprite_y * sprite_y;
                    (sprite, dist_sq)
                })
                .collect();
            sprites_with_dist
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            for (sprite, _) in sprites_with_dist {
                let sprite_x = sprite.x - player.x;
                let sprite_y = sprite.y - player.y;

                let dir_x = player.angle.cos();
                let dir_y = player.angle.sin();

                let plane_x = -dir_y * 0.66;
                let plane_y = dir_x * 0.66;

                let inv_det = 1.0 / (plane_x * dir_y - dir_x * plane_y);

                let transform_x = inv_det * (dir_y * sprite_x - dir_x * sprite_y);
                let transform_y = inv_det * (-plane_y * sprite_x + plane_x * sprite_y);

                if transform_y > 0.0 {
                    // only draw sprites in front of the player
                    let sprite_screen_x = (WIDTH as f32 / 2.0) * (1.0 + transform_x / transform_y);

                    let sprite_height = (HEIGHT as f32 / transform_y).abs() * sprite.height;
                    let world_half = (HEIGHT as f32 / transform_y).abs() * 0.5;
                    let sprite_vertical_offset =
                        (player.z - sprite.z) * HEIGHT as f32 / transform_y - sprite_height * 0.5
                            + world_half;

                    let draw_start_y = (-sprite_height / 2.0
                        + HEIGHT as f32 / 2.0
                        + pitch_offset as f32
                        + sprite_vertical_offset)
                        .max(0.0) as usize;
                    let draw_end_y = (sprite_height / 2.0
                        + HEIGHT as f32 / 2.0
                        + pitch_offset as f32
                        + sprite_vertical_offset)
                        .min(HEIGHT as f32) as usize;

                    let sprite_width = (WIDTH as f32 / transform_y).abs() * sprite.width;
                    let draw_start_x = (sprite_screen_x - sprite_width / 2.0).max(0.0) as usize;
                    let draw_end_x =
                        (sprite_screen_x + sprite_width / 2.0).min(WIDTH as f32) as usize;

                    if let Some(texture) = self.texture_manager.get_texture(&sprite.texture) {
                        for stripe in draw_start_x..draw_end_x {
                            if transform_y < self.z_buffer[stripe] {
                                let tex_x =
                                    ((stripe as f32 - (sprite_screen_x - sprite_width / 2.0))
                                        * texture.width as f32
                                        / sprite_width) as u32;

                                for y in draw_start_y..draw_end_y {
                                    let tex_y = ((y as f32
                                        - (HEIGHT as f32 / 2.0 - sprite_height / 2.0
                                            + pitch_offset as f32
                                            + sprite_vertical_offset as f32))
                                        * texture.height as f32
                                        / sprite_height)
                                        as u32;

                                    let color =
                                        texture.pixels[(tex_y * texture.width + tex_x) as usize];
                                    let alpha = (color >> 24) & 0xFF;

                                    if alpha > 0 {
                                        self.buffer[y * WIDTH + stripe] = color;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Render minimap overlay
        self.render_minimap(game_state, my_id);
    }

    pub fn draw_to_buffer(&self, frame: &mut [u8]) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let color = self.buffer[i];
            let rgba = [(color >> 16) as u8, (color >> 8) as u8, color as u8, 0xFF];
            pixel.copy_from_slice(&rgba);
        }
    }

    // ===== Minimap Helper Functions =====

    /// Fill a rectangle with a color
    fn fill_rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: u32) {
        for row in 0..height {
            let py = y + row;
            if py >= HEIGHT {
                break;
            }
            for col in 0..width {
                let px = x + col;
                if px >= WIDTH {
                    break;
                }
                self.buffer[py * WIDTH + px] = color;
            }
        }
    }

    /// Draw a filled circle
    fn draw_circle(&mut self, cx: usize, cy: usize, radius: usize, color: u32) {
        let r2 = (radius * radius) as i32;
        for y in 0..=(2 * radius) {
            for x in 0..=(2 * radius) {
                let dx = x as i32 - radius as i32;
                let dy = y as i32 - radius as i32;
                if dx * dx + dy * dy <= r2 {
                    let px = (cx as i32 + dx) as usize;
                    let py = (cy as i32 + dy) as usize;
                    if px < WIDTH && py < HEIGHT {
                        self.buffer[py * WIDTH + px] = color;
                    }
                }
            }
        }
    }

    /// Draw a line between two points (simple Bresenham-ish approach)
    fn draw_line(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, color: u32) {
        let dx = (x1 as i32 - x0 as i32).abs();
        let dy = (y1 as i32 - y0 as i32).abs();
        let sx = if x1 as i32 > x0 as i32 { 1 } else { -1 };
        let sy = if y1 as i32 > y0 as i32 { 1 } else { -1 };
        let mut err = (dx - dy) / 2;
        let mut x = x0 as i32;
        let mut y = y0 as i32;
        let max_steps = (dx + dy + 1) as usize + 10; // Safety limit to prevent infinite loops
        let mut step_count = 0;

        loop {
            if step_count > max_steps {
                break; // Safety check to prevent infinite loops
            }
            step_count += 1;

            if x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 {
                self.buffer[y as usize * WIDTH + x as usize] = color;
            }
            if x == x1 as i32 && y == y1 as i32 {
                break;
            }
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Render the minimap in the top-right corner
    pub fn render_minimap(&mut self, game_state: &GameState, my_id: u64) {
        let minimap_width = 150;
        let minimap_height = 150;
        let start_x = WIDTH - minimap_width - 10;
        let start_y = 10;

        // Get actual map dimensions
        let map_width = game_state.world.map.len();
        let map_height = if map_width > 0 {
            game_state.world.map[0].len()
        } else {
            1
        };

        // Calculate tile size based on actual map dimensions
        let tile_size = minimap_width / map_width.max(1);

        // Draw background (border and background fill)
        self.fill_rect(start_x, start_y, minimap_width, minimap_height, 0x00111111); // Dark background

        // Draw world tiles
        for y in 0..map_height {
            for x in 0..map_width {
                let px = start_x + x * tile_size;
                let py = start_y + y * tile_size;
                let tile = game_state.world.get_tile(x, y);
                let tile_color = if tile == 1 {
                    0x00444444 // Wall: dark gray
                } else {
                    0x00AAAAAA // Open space: light gray
                };
                self.fill_rect(px, py, tile_size, tile_size, tile_color);

                // Draw grid lines
                self.draw_line(px, py, px + tile_size, py, 0x00222222);
                self.draw_line(px, py, px, py + tile_size, 0x00222222);
            }
        }

        // Draw all other players as dots
        for (id, player) in &game_state.players {
            if id != &my_id.to_string() {
                let px = start_x + (player.x * tile_size as f32) as usize;
                let py = start_y + (player.y * tile_size as f32) as usize;
                self.draw_circle(px, py, 3, 0x0000FF00); // Green: other players
            }
        }

        // Draw own player's direction indicator (solid triangle pointing in view direction)
        if let Some(player) = game_state.players.get(&my_id.to_string()) {
            let px = start_x as i32 + (player.x * tile_size as f32) as i32;
            let py = start_y as i32 + (player.y * tile_size as f32) as i32;

            // Triangle dimensions: tip longer, base shorter
            let tip_len = 12.0;
            let base_len = 6.0;

            // Calculate triangle points
            let tip_x = (px as f32 + player.angle.cos() * tip_len) as i32;
            let tip_y = (py as f32 + player.angle.sin() * tip_len) as i32;

            // Left point (90 degrees left from forward direction)
            let left_angle = player.angle + std::f32::consts::PI / 2.0;
            let left_x = (px as f32 + left_angle.cos() * base_len) as i32;
            let left_y = (py as f32 + left_angle.sin() * base_len) as i32;

            // Right point (90 degrees right from forward direction)
            let right_angle = player.angle - std::f32::consts::PI / 2.0;
            let right_x = (px as f32 + right_angle.cos() * base_len) as i32;
            let right_y = (py as f32 + right_angle.sin() * base_len) as i32;

            // Draw filled triangle (3 lines forming outline)
            if tip_x >= 0 && tip_x < WIDTH as i32 && tip_y >= 0 && tip_y < HEIGHT as i32 {
                self.draw_line(
                    tip_x as usize,
                    tip_y as usize,
                    left_x as usize,
                    left_y as usize,
                    0x00FF0000,
                );
                self.draw_line(
                    left_x as usize,
                    left_y as usize,
                    right_x as usize,
                    right_y as usize,
                    0x00FF0000,
                );
                self.draw_line(
                    right_x as usize,
                    right_y as usize,
                    tip_x as usize,
                    tip_y as usize,
                    0x00FF0000,
                );
            }
        }

        // Draw minimap border
        self.draw_line(
            start_x,
            start_y,
            start_x + minimap_width,
            start_y,
            0x00FFFFFF,
        );
        self.draw_line(
            start_x + minimap_width,
            start_y,
            start_x + minimap_width,
            start_y + minimap_height,
            0x00FFFFFF,
        );
        self.draw_line(
            start_x + minimap_width,
            start_y + minimap_height,
            start_x,
            start_y + minimap_height,
            0x00FFFFFF,
        );
        self.draw_line(
            start_x,
            start_y + minimap_height,
            start_x,
            start_y,
            0x00FFFFFF,
        );
    }
}
