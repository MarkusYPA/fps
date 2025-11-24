use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::consts::FONT_PATH;
use crate::text::draw_text;
use crate::textures::{self};
use crate::{
    AnimationState::{Dead, Dying, Idle, Shooting, Walking},
    Direction, GameState,
    consts::{
        CAMERA_HEIGHT_OFFSET, CAMERA_HEIGHT_OFFSET_DEAD, CAMERA_PLANE_SCALE, CEILING_COLOR,
        CROSSHAIR_SCALE, FLOOR_COLOR, GUN_SCALE, GUN_X_OFFSET, HEIGHT, MINIMAP_HEIGHT,
        MINIMAP_MARGIN, SPRITE_OTHER_PLAYER_HEIGHT, SPRITE_OTHER_PLAYER_WIDTH, WALL_COLOR_PRIMARY,
        WALL_COLOR_SECONDARY, WIDTH,
    },
    spritesheet::SpriteSheet,
    textures::TextureManager,
};
use rusttype::{Font, Scale, point};

fn get_direction(player_angle: f32, camera_angle: f32) -> Direction {
    let angle_diff = ((player_angle - camera_angle).to_degrees() + 180.0).rem_euclid(360.0);
    let direction_index = (angle_diff / 45.0).round() as usize % 8;

    match direction_index {
        0 => Direction::Front,
        1 => Direction::FrontRight,
        2 => Direction::Right,
        3 => Direction::BackRight,
        4 => Direction::Back,
        5 => Direction::BackLeft,
        6 => Direction::Left,
        7 => Direction::FrontLeft,
        _ => unreachable!(),
    }
}

pub struct Renderer<'a> {
    pub buffer: Vec<u32>,
    pub z_buffer: Vec<f32>,
    pub texture_manager: TextureManager,
    pub sprite_sheets: HashMap<String, SpriteSheet>,
    // Transient hit marker state: when set, renderer will flash a marker at screen center
    hit_marker_start: Option<Instant>,
    hit_marker_color: u32,
    hit_marker_duration: Duration,
    font: Font<'a>,
}

struct SpriteInfo<'a> {
    x: f32,
    y: f32,
    z: f32,
    texture: &'a String,
    width: f32,
    height: f32,
    dist_sq: f32,
    frame: Option<&'a textures::Texture>,
}

impl<'a> Renderer<'a> {
    pub fn new(
        texture_manager: TextureManager,
        sprite_sheets: HashMap<String, SpriteSheet>,
    ) -> Self {
        let font_data = std::fs::read(FONT_PATH).unwrap();
        let font = Font::try_from_vec(font_data).unwrap();

        Renderer {
            buffer: vec![0; WIDTH * HEIGHT],
            z_buffer: vec![0.0; WIDTH],
            texture_manager,
            sprite_sheets,
            hit_marker_start: None,
            hit_marker_color: 0x00FFFFFF,
            hit_marker_duration: Duration::from_millis(400),
            font,
        }
    }

    // Trigger a transient hit marker flash (caller decides color).
    pub fn show_hit_marker(&mut self, color: u32) {
        self.hit_marker_start = Some(Instant::now());
        self.hit_marker_color = color;
    }

    fn draw_sprite_2d(
        &mut self,
        texture: &textures::Texture,
        pos_x: usize,
        pos_y: usize,
        scale: f32,
    ) {
        let scaled_width = (texture.width as f32 * scale) as usize;
        let scaled_height = (texture.height as f32 * scale) as usize;

        for y in 0..scaled_height {
            for x in 0..scaled_width {
                let screen_x = pos_x + x;
                let screen_y = pos_y + y;

                if screen_x < WIDTH && screen_y < HEIGHT {
                    let tex_x = (x as f32 / scale) as u32;
                    let tex_y = (y as f32 / scale) as u32;

                    if tex_x < texture.width && tex_y < texture.height {
                        let color = texture.pixels[(tex_y * texture.width + tex_x) as usize];
                        let alpha = (color >> 24) & 0xFF;

                        if alpha > 0 {
                            self.buffer[screen_y * WIDTH + screen_x] = color;
                        }
                    }
                }
            }
        }
    }

    pub fn render(&mut self, game_state: &GameState, my_id: u64) {
        if let Some(player) = game_state.players.get(&my_id.to_string()) {
            let pitch_offset = (player.pitch * HEIGHT as f32 / 2.0) as isize;
            let horizon = (HEIGHT as isize / 2 + pitch_offset).clamp(0, HEIGHT as isize) as usize;

            // Clear the buffer with ceiling and floor colors
            for y in 0..horizon {
                for x in 0..WIDTH {
                    self.buffer[y * WIDTH + x] = CEILING_COLOR;
                }
            }
            for y in horizon..HEIGHT {
                for x in 0..WIDTH {
                    self.buffer[y * WIDTH + x] = FLOOR_COLOR;
                }
            }

            let camera_offset = if player.health > 0 {
                CAMERA_HEIGHT_OFFSET
            } else {
                CAMERA_HEIGHT_OFFSET_DEAD
            };

            // cast one ray for each pixel in width
            for x in 0..WIDTH {
                // ray direction
                let camera_x = 2.0 * x as f32 / WIDTH as f32 - 1.0;
                let ray_dir_x =
                    player.angle.cos() + CAMERA_PLANE_SCALE * camera_x * (-player.angle.sin());
                let ray_dir_y =
                    player.angle.sin() + CAMERA_PLANE_SCALE * camera_x * player.angle.cos();

                // direction and steps to measure if wall was hit
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

                // find wall hits
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

                    if game_state.world.get_tile(map_x, map_y) > 0 {
                        hit = true;
                    }
                }

                // how far wall hit was
                let perp_wall_dist = if wall_type == 0 {
                    (map_x as f32 - player.x + (1.0 - step_x as f32) / 2.0) / ray_dir_x
                } else {
                    (map_y as f32 - player.y + (1.0 - step_y as f32) / 2.0) / ray_dir_y
                };

                self.z_buffer[x] = perp_wall_dist;

                // line hight from distance, start and end points account for jump, pitch and camera offset
                let line_height = (HEIGHT as f32 / perp_wall_dist) as isize;
                let z_offset = ((player.z + camera_offset) * line_height as f32) as isize;
                let draw_start = (-line_height / 2 + HEIGHT as isize / 2 + pitch_offset + z_offset)
                    .clamp(0, HEIGHT as isize - 1) as usize;
                let draw_end = (line_height / 2 + HEIGHT as isize / 2 + pitch_offset + z_offset)
                    .clamp(0, HEIGHT as isize) as usize;

                let wall_tile = game_state.world.get_tile(map_x, map_y);
                let wall_texture_name = format!("wall{}", wall_tile);

                if let Some(texture) = self.texture_manager.get_texture(&wall_texture_name) {
                    // calculate where the wall was hit
                    let wall_x = if wall_type == 0 {
                        player.y + perp_wall_dist * ray_dir_y
                    } else {
                        player.x + perp_wall_dist * ray_dir_x
                    };
                    let wall_x = wall_x - wall_x.floor();

                    // x coordinate on the texture
                    let mut tex_x = (wall_x * texture.width as f32) as u32;
                    if (wall_type == 0 && ray_dir_x > 0.0) || (wall_type > 0 && ray_dir_y < 0.0) {
                        tex_x = texture.width - tex_x - 1;
                    }

                    // save vertical wall line to buffer
                    for y in draw_start..draw_end {
                        let tex_y_num =
                            (y as isize - HEIGHT as isize / 2 - pitch_offset - z_offset
                                + line_height / 2)
                                * texture.height as isize;
                        if line_height == 0 {
                            continue;
                        }
                        let tex_y = (tex_y_num / line_height)
                            .max(0)
                            .min(texture.height as isize - 1)
                            as u32;

                        let color_index = (tex_y * texture.width + tex_x) as usize;
                        if color_index < texture.pixels.len() {
                            let color = texture.pixels[color_index];

                            // Make one side of wall darker
                            let final_color = if wall_type > 0 {
                                color
                            } else {
                                let r = (color >> 16) & 0xFF;
                                let g = (color >> 8) & 0xFF;
                                let b = color & 0xFF;
                                let a = (color >> 24) & 0xFF;
                                (a << 24) | ((r / 2) << 16) | ((g / 2) << 8) | (b / 2)
                            };
                            self.buffer[y * WIDTH + x] = final_color;
                        }
                    }
                } else {
                    // Fallback to solid color if texture not found
                    let wall_color = if wall_type == 1 {
                        WALL_COLOR_PRIMARY
                    } else {
                        WALL_COLOR_SECONDARY
                    };
                    for y in draw_start..draw_end {
                        self.buffer[y * WIDTH + x] = wall_color;
                    }
                }
            }

            // floor sprites (puddles) from world
            let mut sprite_infos: Vec<SpriteInfo> = game_state
                .floor_sprites
                .iter()
                .map(|(_, s)| {
                    let sprite_x = s.x - player.x;
                    let sprite_y = s.y - player.y;
                    SpriteInfo {
                        x: s.x,
                        y: s.y,
                        z: s.z,
                        texture: &s.texture,
                        width: s.width,
                        height: s.height,
                        dist_sq: sprite_x * sprite_x + sprite_y * sprite_y,
                        frame: None,
                    }
                })
                .collect();

            // sprites from other players
            let mut player_sprites = Vec::new();
            for (id, other_player) in &game_state.players {
                if id != &my_id.to_string() {
                    let direction = get_direction(other_player.angle, player.angle);
                    let frame = match other_player.animation_state {
                        Idle => {
                            &self.sprite_sheets.get(&other_player.texture).unwrap().idle
                                [direction as usize]
                        }
                        Walking => {
                            &self.sprite_sheets.get(&other_player.texture).unwrap().walk
                                [direction as usize][other_player.frame]
                        }
                        Shooting => {
                            &self.sprite_sheets.get(&other_player.texture).unwrap().shoot
                                [direction as usize]
                        }
                        Dying => {
                            &self.sprite_sheets.get(&other_player.texture).unwrap().die
                                [other_player.frame]
                        }
                        Dead => &self.sprite_sheets.get(&other_player.texture).unwrap().dead[0],
                    };

                    let sprite_x = other_player.x - player.x;
                    let sprite_y = other_player.y - player.y;
                    player_sprites.push(SpriteInfo {
                        x: other_player.x,
                        y: other_player.y,
                        z: other_player.z,
                        texture: &other_player.texture,
                        width: SPRITE_OTHER_PLAYER_WIDTH,
                        height: SPRITE_OTHER_PLAYER_HEIGHT,
                        dist_sq: sprite_x * sprite_x + sprite_y * sprite_y,
                        frame: Some(frame),
                    });
                }
            }

            // Sort floor sprites (puddles) by distance
            sprite_infos.sort_by(|a, b| {
                b.dist_sq
                    .partial_cmp(&a.dist_sq)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Sort player sprites by distance
            player_sprites.sort_by(|a, b| {
                b.dist_sq
                    .partial_cmp(&a.dist_sq)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Combine sprite vectors so puddles are always behind players
            sprite_infos.append(&mut player_sprites);

            // sprites to buffer
            for sprite_info in sprite_infos {
                let sprite_x = sprite_info.x - player.x;
                let sprite_y = sprite_info.y - player.y;

                let dir_x = player.angle.cos();
                let dir_y = player.angle.sin();

                let plane_x = -dir_y * CAMERA_PLANE_SCALE;
                let plane_y = dir_x * CAMERA_PLANE_SCALE;

                let inv_det = 1.0 / (plane_x * dir_y - dir_x * plane_y);
                let transform_x = inv_det * (dir_y * sprite_x - dir_x * sprite_y);
                let transform_y = inv_det * (-plane_y * sprite_x + plane_x * sprite_y);

                // only draw sprites in front of the player
                if transform_y > 0.0 {
                    let sprite_screen_x = (WIDTH as f32 / 2.0) * (1.0 + transform_x / transform_y);

                    // put sprite on the floor if its z is 0
                    let sprite_height = (HEIGHT as f32 / transform_y).abs() * sprite_info.height;
                    let world_half = (HEIGHT as f32 / transform_y).abs() * 0.5;
                    let sprite_vertical_offset =
                        (player.z + camera_offset - sprite_info.z) * HEIGHT as f32 / transform_y
                            - sprite_height * 0.5
                            + world_half;

                    // start and end points with both z:s, pitch and camera offset accounted for
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

                    let sprite_width = (WIDTH as f32 / transform_y).abs() * sprite_info.width;
                    let draw_start_x = (sprite_screen_x - sprite_width / 2.0).max(0.0) as usize;
                    let draw_end_x =
                        (sprite_screen_x + sprite_width / 2.0).min(WIDTH as f32) as usize;

                    // animation frames or static sprites
                    if let Some(raster) = sprite_info
                        .frame
                        .or_else(|| self.texture_manager.get_texture(sprite_info.texture))
                    {
                        // process vertical lines
                        for stripe in draw_start_x..draw_end_x {
                            // proceed if line is closer than any wall there
                            if transform_y < self.z_buffer[stripe] {
                                let tex_x =
                                    ((stripe as f32 - (sprite_screen_x - sprite_width / 2.0))
                                        * raster.width as f32
                                        / sprite_width) as u32;

                                // get pixels on the vertical line
                                for y in draw_start_y..draw_end_y {
                                    let tex_y = ((y as f32
                                        - (HEIGHT as f32 / 2.0 - sprite_height / 2.0
                                            + pitch_offset as f32
                                            + sprite_vertical_offset as f32))
                                        * raster.height as f32
                                        / sprite_height)
                                        as u32;

                                    if tex_x < raster.width && tex_y < raster.height {
                                        let color =
                                            raster.pixels[(tex_y * raster.width + tex_x) as usize];
                                        let alpha = (color >> 24) & 0xFF;

                                        // save to renderer buffer
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

            if player.health > 0 {
                // Render gun
                if let Some(player) = game_state.players.get(&my_id.to_string()) {
                    let gun_texture_name = if player.shooting { "gunshot" } else { "gun" };
                    if let Some(gun_texture) =
                        self.texture_manager.get_texture(gun_texture_name).cloned()
                    {
                        let gun_x =
                            WIDTH - (gun_texture.width as f32 * GUN_SCALE) as usize - GUN_X_OFFSET;
                        let gun_y = HEIGHT - (gun_texture.height as f32 * GUN_SCALE) as usize;
                        self.draw_sprite_2d(&gun_texture, gun_x, gun_y, GUN_SCALE);
                    }
                }

                // Render crosshair
                if let Some(ch_texture) = self.texture_manager.get_texture("crosshair").cloned() {
                    let ch_x =
                        WIDTH / 2 - ((ch_texture.width as f32 * CROSSHAIR_SCALE) / 2.0) as usize;
                    let ch_y =
                        HEIGHT / 2 - ((ch_texture.height as f32 * CROSSHAIR_SCALE) / 2.0) as usize;
                    self.draw_sprite_2d(&ch_texture, ch_x, ch_y, CROSSHAIR_SCALE);
                }
            }

            // Render transient hit marker (overlays crosshair)
            if let Some(start) = self.hit_marker_start {
                if start.elapsed() < self.hit_marker_duration {
                    let cx = (WIDTH / 2) as i32;
                    let cy = (HEIGHT / 2) as i32;
                    let inner = 6;
                    let outer = 14;
                    let color = self.hit_marker_color;

                    // Draw the four lines of the hit marker
                    self.draw_line(cx - inner, cy - inner, cx - outer, cy - outer, color);
                    self.draw_line(cx + inner, cy - inner, cx + outer, cy - outer, color);
                    self.draw_line(cx - inner, cy + inner, cx - outer, cy + outer, color);
                    self.draw_line(cx + inner, cy + inner, cx + outer, cy + outer, color);
                } else {
                    self.hit_marker_start = None;
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

    fn measure_text_bounds(&self, text: &str, size: f32) -> (f32, f32) {
        let scale = Scale::uniform(size);
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for glyph in self.font.layout(text, scale, point(0.0, 0.0)) {
            if let Some(bb) = glyph.pixel_bounding_box() {
                min_x = min_x.min(bb.min.x as f32);
                max_x = max_x.max(bb.max.x as f32);
                min_y = min_y.min(bb.min.y as f32);
                max_y = max_y.max(bb.max.y as f32);
            }
        }

        if !min_x.is_finite() {
            (0.0, 0.0)
        } else {
            (max_x - min_x, max_y - min_y)
        }
    }

    pub fn fill_rect(
        frame: &mut [u8],
        rect_x: usize,
        rect_y: usize,
        rect_w: usize,
        rect_h: usize,
        color: [u8; 4],
    ) {
        for y in rect_y..(rect_y + rect_h) {
            for x in rect_x..(rect_x + rect_w) {
                if x < WIDTH && y < HEIGHT {
                    let idx = (y * WIDTH + x) * 4;
                    if idx + 3 < frame.len() {
                        let bg_r = frame[idx];
                        let bg_g = frame[idx + 1];
                        let bg_b = frame[idx + 2];

                        let alpha = color[3] as u16;
                        let r = (color[0] as u16 * alpha + bg_r as u16 * (255 - alpha)) / 255;
                        let g = (color[1] as u16 * alpha + bg_g as u16 * (255 - alpha)) / 255;
                        let b = (color[2] as u16 * alpha + bg_b as u16 * (255 - alpha)) / 255;

                        frame[idx] = r as u8;
                        frame[idx + 1] = g as u8;
                        frame[idx + 2] = b as u8;
                        frame[idx + 3] = 255;
                    }
                }
            }
        }
    }

    pub fn display_health(&self, game_state: &GameState, my_id: u64, frame: &mut [u8]) {
        if let Some(player) = game_state.players.get(&my_id.to_string()) {
            // Draw a semi-transparent black rectangle behind the health text
            let rect_x = 100;
            let rect_y = HEIGHT - 55;
            let rect_w = 150;
            let rect_h = 40;
            let color = [0, 0, 0, 128]; // semi-transparent black

            Self::fill_rect(frame, rect_x, rect_y, rect_w, rect_h, color);

            draw_text(
                frame,
                &self.font,
                "Health",
                30.0,
                110,
                HEIGHT - 50,
                [220, 210, 200, 255],
            );

            draw_text(
                frame,
                &self.font,
                &player.health.to_string(),
                30.0,
                200,
                HEIGHT - 50,
                [255, 255, 255, 255],
            );
        }
    }

    pub fn display_leaderboard(&self, game_state: &GameState, frame: &mut [u8]) {
        let mut sorted_entries: Vec<_> = game_state.leaderboard.iter().collect();
        sorted_entries.sort_by(|(name_a, score_a), (name_b, score_b)| {
            score_b.cmp(score_a).then_with(|| name_a.cmp(name_b))
        });

        let formatted_entries: Vec<String> = sorted_entries
            .into_iter()
            .map(|(name, score)| format!("{}: {}", name, score))
            .collect();

        let title_text = "Leaderboard";
        let title_font_size = 28.0;
        let entry_font_size = 24.0;

        let (title_width, title_height) = self.measure_text_bounds(title_text, title_font_size);
        let mut max_entry_width = title_width;
        for entry in &formatted_entries {
            let (entry_width, _) = self.measure_text_bounds(entry, entry_font_size);
            max_entry_width = max_entry_width.max(entry_width);
        }

        let padding_x = 16;
        let padding_y = 12;
        let header_gap = 10;
        let row_gap = 6;
        let rect_margin = 20;

        let header_height = title_height.ceil() as usize + header_gap;
        let row_height = entry_font_size.ceil() as usize + row_gap;
        let rect_width = max_entry_width.ceil() as usize + padding_x * 2;
        let rect_height = padding_y * 2 + header_height + formatted_entries.len() * row_height;

        let rect_x = WIDTH.saturating_sub(rect_width + rect_margin);
        let desired_rect_y = MINIMAP_MARGIN * 2 + MINIMAP_HEIGHT;
        // Anchor below the minimap; extremely long lists may extend past the bottom.
        let rect_y = desired_rect_y.min(HEIGHT.saturating_sub(1));

        Self::fill_rect(
            frame,
            rect_x,
            rect_y,
            rect_width,
            rect_height.max(1),
            [0, 0, 0, 160],
        );

        let text_x = rect_x + padding_x;
        let mut text_y = rect_y + padding_y;

        draw_text(
            frame,
            &self.font,
            title_text,
            title_font_size,
            text_x,
            text_y,
            [220, 210, 200, 255],
        );

        text_y += header_height;
        for entry in &formatted_entries {
            draw_text(
                frame,
                &self.font,
                entry,
                entry_font_size,
                text_x,
                text_y,
                [255, 255, 255, 255],
            );
            text_y += row_height;
        }
    }

    pub fn display_winner(&self, winner_name: &str, frame: &mut [u8]) {
        let font_size = 150.0;
        let text = format!("{} Won!", winner_name);

        // Measure text to determine box size
        let (text_width, text_height) = self.measure_text_bounds(&text, font_size);

        // Add padding around the text (40 pixels on each side)
        let padding = 40;
        let rect_w = (text_width as usize) + padding * 2;
        let rect_h = (text_height as usize) + padding * 2;

        // Center the box on screen
        let rect_x = (WIDTH - rect_w) / 2;
        let rect_y = (HEIGHT - rect_h) / 2;
        let color = [0, 0, 0, 200]; // semi-transparent black

        Self::fill_rect(frame, rect_x, rect_y, rect_w, rect_h, color);

        // Center text horizontally within the box
        let text_x = rect_x + (rect_w as f32 / 2.0 - text_width / 2.0) as usize;

        // Center text vertically
        // draw_text uses (y + v_metrics.ascent) as the baseline
        // To center the text, we position it so the visual center aligns with the box center
        let box_center_y = rect_y as f32 + rect_h as f32 / 2.0;
        let text_y = (box_center_y - text_height) as usize;

        draw_text(
            frame,
            &self.font,
            &text,
            font_size,
            text_x,
            text_y,
            [255, 215, 0, 255], // Gold color for winner text
        );
    }
}
