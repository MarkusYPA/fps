use crate::renderer::Renderer;
use crate::{
    GameState, consts::HEIGHT, consts::MINIMAP_BACKGROUND_COLOR, consts::MINIMAP_BORDER_COLOR,
    consts::MINIMAP_GRID_COLOR, consts::MINIMAP_HEIGHT, consts::MINIMAP_MARGIN,
    consts::MINIMAP_OPEN_SPACE_COLOR, consts::MINIMAP_OTHER_PLAYER_COLOR,
    consts::MINIMAP_PLAYER_DOT_RADIUS, consts::MINIMAP_PLAYER_ICON_SIZE,
    consts::MINIMAP_WALL_COLOR, consts::MINIMAP_WIDTH, consts::WIDTH,
};

impl Renderer {
    /* ---------------------------------------------------------
     * Helper: fill a rectangle
     * --------------------------------------------------------- */
    fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        for row in y..y + h {
            if row >= HEIGHT { break; }

            let start = row * WIDTH + x;
            let end   = start + w.min(WIDTH - x);
            self.buffer[start..end].fill(color);
        }
    }

    /* ---------------------------------------------------------
     * Helper: draw filled circle
     * --------------------------------------------------------- */
    fn draw_circle(&mut self, cx: usize, cy: usize, radius: usize, color: u32) {
    let r = radius as i32;
    let r2 = r * r;

    for y in -r..=r {
        for x in -r..=r {
            if x * x + y * y <= r2 {
                let px = cx as i32 + x;
                let py = cy as i32 + y;

                if px >= 0 && py >= 0 && px < WIDTH as i32 && py < HEIGHT as i32 {
                    self.buffer[py as usize * WIDTH + px as usize] = color;
                }
            }
        }
    }
}

    /* ---------------------------------------------------------
     * Helper: Bresenham line
     * --------------------------------------------------------- */
    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x1 > x0 { 1 } else { -1 };
        let sy = if y1 > y0 { 1 } else { -1 };
        let mut err = dx - dy;
        let mut x = x0;
        let mut y = y0;

        let max_steps = (dx as usize + dy as usize + 1) + 100;
        let mut steps = 0;

        loop {
            if steps > max_steps { break; }
            steps += 1;

            if x >= 0 && y >= 0 && x < WIDTH as i32 && y < HEIGHT as i32 {
                self.buffer[y as usize * WIDTH + x as usize] = color;
            }

            if x == x1 && y == y1 { break; }

            let e2 = err * 2;
            if e2 > -dy { err -= dy; x += sx; }
            if e2 <  dx { err += dx; y += sy; }
        }
    }

    /* ---------------------------------------------------------
     * Helper: minimap grid
     * --------------------------------------------------------- */
    fn draw_grid_cell(&mut self, px: usize, py: usize, size: usize, color: u32) {
        self.draw_line(px as i32, py as i32, (px + size) as i32, py as i32, color);
        self.draw_line(px as i32, py as i32, px as i32, (py + size) as i32, color);
    }

    /* ---------------------------------------------------------
     * Main: minimap
     * --------------------------------------------------------- */
    pub fn render_minimap(&mut self, game: &GameState, my_id: u64) {
        let w = MINIMAP_WIDTH;
        let h = MINIMAP_HEIGHT;
        let x0 = WIDTH - w - MINIMAP_MARGIN;
        let y0 = MINIMAP_MARGIN;

        /* Background */
        self.fill_rect(x0, y0, w, h, MINIMAP_BACKGROUND_COLOR);

        /* Map info */
        let map_w = game.world.map.len().max(1);
        let map_h = game.world.map.get(0).map_or(1, |row| row.len());
        let tile  = w / map_w;

        /* Tiles + grid */
        for y in 0..map_h {
            for x in 0..map_w {
                let px = x0 + x * tile;
                let py = y0 + y * tile;

                let tile_color = if game.world.get_tile(x, y) > 0 {
                    MINIMAP_WALL_COLOR
                } else {
                    MINIMAP_OPEN_SPACE_COLOR
                };

                self.fill_rect(px, py, tile, tile, tile_color);
                self.draw_grid_cell(px, py, tile, MINIMAP_GRID_COLOR);
            }
        }

        /* Other players */
        for (id, p) in &game.players {
            if id != &my_id.to_string() {
                let px = x0 + (p.x * tile as f32) as usize;
                let py = y0 + (p.y * tile as f32) as usize;

                self.draw_circle(px, py, MINIMAP_PLAYER_DOT_RADIUS, MINIMAP_OTHER_PLAYER_COLOR);
            }
        }

        /* Own player icon */
        if let Some(p) = game.players.get(&my_id.to_string()) {
            if let Some(tex) = self.texture_manager.get_texture("navigator") {
                let size = MINIMAP_PLAYER_ICON_SIZE as i32;
                let (hw, hh) = (size / 2, size / 2);

                let cx = x0 as f32 + p.x * tile as f32;
                let cy = y0 as f32 + p.y * tile as f32;

                let tex_cx = tex.width as f32 * 0.5;
                let tex_cy = tex.height as f32 * 0.5;

                let sx = tex.width as f32 / size as f32;
                let sy = tex.height as f32 / size as f32;

                let angle = p.angle + std::f32::consts::FRAC_PI_2;
                let (sin_a, cos_a) = angle.sin_cos();

                for dy in -hh..hh {
                    let dst_y = cy as i32 + dy;
                    if dst_y < y0 as i32 || dst_y >= (y0 + h) as i32 { continue; }

                    for dx in -hw..hw {
                        let dst_x = cx as i32 + dx;
                        if dst_x < x0 as i32 || dst_x >= (x0 + w) as i32 { continue; }

                        let src_x =  dx as f32 * sx * cos_a + dy as f32 * sy * sin_a + tex_cx;
                        let src_y = -dx as f32 * sx * sin_a + dy as f32 * sy * cos_a + tex_cy;

                        let sx_i = src_x as i32;
                        let sy_i = src_y as i32;

                        if sx_i >= 0 && sy_i >= 0
                            && (sx_i as u32) < tex.width
                            && (sy_i as u32) < tex.height
                        {
                            let color = tex.pixels[(sy_i as u32 * tex.width + sx_i as u32) as usize];

                            if (color >> 24) & 0xFF > 0 {
                                self.buffer[dst_y as usize * WIDTH + dst_x as usize] = color;
                            }
                        }
                    }
                }
            }
        }

        /* Border */
        let ex = x0 + w;
        let ey = y0 + h;

        self.draw_line(x0 as i32, y0 as i32, ex as i32, y0 as i32, MINIMAP_BORDER_COLOR); // top
        self.draw_line(ex as i32,  y0 as i32, ex as i32, ey as i32, MINIMAP_BORDER_COLOR); // right
        self.draw_line(ex as i32,  ey as i32, x0 as i32, ey as i32, MINIMAP_BORDER_COLOR); // bottom
        self.draw_line(x0 as i32,  ey as i32, x0 as i32, y0 as i32, MINIMAP_BORDER_COLOR); // left
    }
}
