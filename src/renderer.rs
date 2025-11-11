use std::collections::HashMap;

use crate::textures::{self};
use crate::{
    Direction, GameState,
    consts::{
        CAMERA_HEIGHT_OFFSET, CAMERA_PLANE_SCALE, CEILING_COLOR, CROSSHAIR_SCALE, FLOOR_COLOR,
        GUN_SCALE, GUN_X_OFFSET, HEIGHT, SPRITE_OTHER_PLAYER_HEIGHT, SPRITE_OTHER_PLAYER_WIDTH,
        WALL_COLOR_PRIMARY, WALL_COLOR_SECONDARY, WIDTH,
    },
    spritesheet::SpriteSheet,
    textures::TextureManager,
};

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

pub struct Renderer {
    pub buffer: Vec<u32>,
    pub z_buffer: Vec<f32>,
    pub texture_manager: TextureManager,
    pub sprite_sheets: HashMap<String, SpriteSheet>,
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

impl Renderer {
    pub fn new(
        texture_manager: TextureManager,
        sprite_sheets: HashMap<String, SpriteSheet>,
    ) -> Self {
        Renderer {
            buffer: vec![0; WIDTH * HEIGHT],
            z_buffer: vec![0.0; WIDTH],
            texture_manager,
            sprite_sheets,
        }
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
                let z_offset = ((player.z + CAMERA_HEIGHT_OFFSET) * line_height as f32) as isize;
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
                    if (wall_type == 0 && ray_dir_x > 0.0) || (wall_type == 1 && ray_dir_y < 0.0) {
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
                            let final_color = if wall_type == 1 {
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

            // sprites from world
            let mut sprite_infos: Vec<SpriteInfo> = game_state
                .sprites
                .iter()
                .map(|s| {
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
            for (id, other_player) in &game_state.players {
                if id != &my_id.to_string() {
                    let direction = get_direction(other_player.angle, player.angle);
                    let frame = match other_player.animation_state {
                        crate::AnimationState::Idle => {
                            &self.sprite_sheets.get(&other_player.texture).unwrap().idle
                                [direction as usize]
                        }
                        crate::AnimationState::Walking => {
                            &self.sprite_sheets.get(&other_player.texture).unwrap().walk
                                [direction as usize][other_player.frame]
                        }
                        crate::AnimationState::Shooting => {
                            &self.sprite_sheets.get(&other_player.texture).unwrap().shoot
                                [direction as usize]
                        }
                    };

                    let sprite_x = other_player.x - player.x;
                    let sprite_y = other_player.y - player.y;
                    sprite_infos.push(SpriteInfo {
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

            // Sort sprites by distance
            sprite_infos.sort_by(|a, b| {
                b.dist_sq
                    .partial_cmp(&a.dist_sq)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

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
                    let sprite_vertical_offset = (player.z + CAMERA_HEIGHT_OFFSET - sprite_info.z)
                        * HEIGHT as f32
                        / transform_y
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
        }

        // Render minimap overlay
        self.render_minimap(game_state, my_id);

        // Render gun
        if let Some(player) = game_state.players.get(&my_id.to_string()) {
            let gun_texture_name = if player.shooting { "gunshot" } else { "gun" };
            if let Some(gun_texture) = self.texture_manager.get_texture(gun_texture_name).cloned() {
                let gun_x = WIDTH - (gun_texture.width as f32 * GUN_SCALE) as usize - GUN_X_OFFSET;
                let gun_y = HEIGHT - (gun_texture.height as f32 * GUN_SCALE) as usize;
                self.draw_sprite_2d(&gun_texture, gun_x, gun_y, GUN_SCALE);
            }
        }

        // Render crosshair
        if let Some(ch_texture) = self.texture_manager.get_texture("crosshair").cloned() {
            let ch_x = WIDTH / 2 - ((ch_texture.width as f32 * CROSSHAIR_SCALE) / 2.0) as usize;
            let ch_y = HEIGHT / 2 - ((ch_texture.height as f32 * CROSSHAIR_SCALE) / 2.0) as usize;
            self.draw_sprite_2d(&ch_texture, ch_x, ch_y, CROSSHAIR_SCALE);
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
