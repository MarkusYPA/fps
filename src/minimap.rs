use crate::renderer::Renderer;
use crate::{
    GameState, consts::HEIGHT, consts::MINIMAP_BACKGROUND_COLOR, consts::MINIMAP_BORDER_COLOR,
    consts::MINIMAP_GRID_COLOR, consts::MINIMAP_HEIGHT, consts::MINIMAP_MARGIN,
    consts::MINIMAP_OPEN_SPACE_COLOR, consts::MINIMAP_OTHER_PLAYER_COLOR,
    consts::MINIMAP_PLAYER_DOT_RADIUS, consts::MINIMAP_PLAYER_ICON_SIZE,
    consts::MINIMAP_WALL_COLOR, consts::MINIMAP_WIDTH, consts::WIDTH,
};

impl<'a> Renderer<'a> {
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

    // Draw a line between two points (simple Bresenham-ish approach)
    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
        // Bresenham's line algorithm with i32 coords and clipping checks.
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x1 > x0 { 1 } else { -1 };
        let sy = if y1 > y0 { 1 } else { -1 };
        let mut err = dx - dy;
        let mut x = x0;
        let mut y = y0;

        // Safety limit to prevent infinite loops in degenerate cases
        let max_steps = (dx as i64 + dy as i64 + 1) as usize + 100;
        let mut step_count = 0usize;

        loop {
            if step_count > max_steps {
                break;
            }
            step_count += 1;
            // Plot the pixel if within bounds
            if x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 {
                self.buffer[y as usize * WIDTH + x as usize] = color;
            }

            if x == x1 && y == y1 {
                break;
            }
            // Update error term and coordinates
            let e2 = err * 2;
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
        let minimap_width = MINIMAP_WIDTH;
        let minimap_height = MINIMAP_HEIGHT;
        let start_x = WIDTH - minimap_width - MINIMAP_MARGIN;
        let start_y = MINIMAP_MARGIN;

        // Get actual map dimensions (fix swapped width/height)
        let map_height = game_state.world.map.len();
        let map_width = if map_height > 0 {
            game_state.world.map[0].len()
        } else {
            1
        };

        // Use dynamic fractional tile mapping for perfect fit on any map size
        let map_w = map_width.max(1) as f32;
        let map_h = map_height.max(1) as f32;
        let mmw = minimap_width as f32;
        let mmh = minimap_height as f32;

        // Calculate tile size that fits both dimensions and preserves aspect ratio
        let tile_size_f = (mmw / map_w).min(mmh / map_h);
        let total_w = tile_size_f * map_w;
        let total_h = tile_size_f * map_h;

        // Center the map in the minimap box
        let offset_x = (mmw - total_w) * 0.5;
        let offset_y = (mmh - total_h) * 0.5;
        let base_x = start_x as f32 + offset_x;
        let base_y = start_y as f32 + offset_y;

        // Draw background first
        self.fill_rect(
            start_x,
            start_y,
            minimap_width,
            minimap_height,
            MINIMAP_BACKGROUND_COLOR,
        );

        // Draw tiles with fractional positioning for perfect coverage
        for tile_y in 0..map_height {
            let y0_f = base_y + tile_y as f32 * tile_size_f;
            let y1_f = base_y + (tile_y + 1) as f32 * tile_size_f;
            let py0 = y0_f.floor() as usize;
            let py1 = y1_f.ceil() as usize;

            if py1 <= py0 {
                continue;
            }

            for tile_x in 0..map_width {
                let x0_f = base_x + tile_x as f32 * tile_size_f;
                let x1_f = base_x + (tile_x + 1) as f32 * tile_size_f;
                let px0 = x0_f.floor() as usize;
                let px1 = x1_f.ceil() as usize;

                if px1 <= px0 {
                    continue;
                }

                let tile = game_state.world.get_tile(tile_x, tile_y);
                let tile_color = if tile > 0 {
                    MINIMAP_WALL_COLOR
                } else {
                    MINIMAP_OPEN_SPACE_COLOR
                };

                self.fill_rect(px0, py0, px1 - px0, py1 - py0, tile_color);

                // Draw grid lines at tile boundaries
                self.draw_line(
                    px0 as i32,
                    py0 as i32,
                    px1 as i32,
                    py0 as i32,
                    MINIMAP_GRID_COLOR,
                );
                self.draw_line(
                    px0 as i32,
                    py0 as i32,
                    px0 as i32,
                    py1 as i32,
                    MINIMAP_GRID_COLOR,
                );
            }
        }

        // Draw all other players using the dynamic coordinate system
        for (id, player) in &game_state.players {
            if id != &my_id.to_string() {
                let px = (base_x + player.x * tile_size_f).round() as usize;
                let py = (base_y + player.y * tile_size_f).round() as usize;
                self.draw_circle(
                    px,
                    py,
                    MINIMAP_PLAYER_DOT_RADIUS,
                    MINIMAP_OTHER_PLAYER_COLOR,
                );
            }
        }

        // Draw own player's indicator using a navigator PNG
        if let Some(player) = game_state.players.get(&my_id.to_string()) {
            if let Some(tex) = self.texture_manager.get_texture("navigator") {
                let icon_size = MINIMAP_PLAYER_ICON_SIZE;
                let (icon_w, icon_h) = (icon_size as i32, icon_size as i32);
                let (half_w, half_h) = (icon_w / 2, icon_h / 2);

                let center_px = base_x + player.x * tile_size_f;
                let center_py = base_y + player.y * tile_size_f;

                let tex_cx = tex.width as f32 * 0.5;
                let tex_cy = tex.height as f32 * 0.5;
                let scale_x = tex.width as f32 / icon_size;
                let scale_y = tex.height as f32 / icon_size;

                // simplified rotation formula (equivalent to +PI/2)
                let angle = player.angle + std::f32::consts::FRAC_PI_2;
                let (sin_a, cos_a) = angle.sin_cos();

                for dy in -half_h..half_h {
                    let dst_y = center_py as i32 + dy;
                    if dst_y < start_y as i32 || dst_y >= (start_y + minimap_height) as i32 {
                        continue;
                    }

                    for dx in -half_w..half_w {
                        let dst_x = center_px as i32 + dx;
                        if dst_x < start_x as i32 || dst_x >= (start_x + minimap_width) as i32 {
                            continue;
                        }

                        // Rotate and scale
                        let src_x = ((dx as f32) * scale_x) * cos_a
                            + ((dy as f32) * scale_y) * sin_a
                            + tex_cx;
                        let src_y = -((dx as f32) * scale_x) * sin_a
                            + ((dy as f32) * scale_y) * cos_a
                            + tex_cy;

                        let sx = src_x as i32;
                        let sy = src_y as i32;

                        if sx >= 0 && sy >= 0 && (sx as u32) < tex.width && (sy as u32) < tex.height
                        {
                            let color = tex.pixels[(sy as u32 * tex.width + sx as u32) as usize];
                            if (color >> 24) & 0xFF > 0 {
                                self.buffer[dst_y as usize * WIDTH + dst_x as usize] = color;
                            }
                        }
                    }
                }
            }
        }

        // Draw minimap border
        self.draw_line(
            start_x as i32,
            start_y as i32,
            (start_x + minimap_width) as i32,
            start_y as i32,
            MINIMAP_BORDER_COLOR,
        );
        self.draw_line(
            (start_x + minimap_width) as i32,
            start_y as i32,
            (start_x + minimap_width) as i32,
            (start_y + minimap_height) as i32,
            MINIMAP_BORDER_COLOR,
        );
        self.draw_line(
            (start_x + minimap_width) as i32,
            (start_y + minimap_height) as i32,
            start_x as i32,
            (start_y + minimap_height) as i32,
            MINIMAP_BORDER_COLOR,
        );
        self.draw_line(
            start_x as i32,
            (start_y + minimap_height) as i32,
            start_x as i32,
            start_y as i32,
            MINIMAP_BORDER_COLOR,
        );
    }
}
