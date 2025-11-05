use crate::{
    Direction, GameState, HEIGHT, Sprite, WIDTH, spritesheet::SpriteSheet, textures::TextureManager,
};

const CAMERA_HEIGHT_OFFSET: f32 = 0.1;

fn get_direction(player_angle: f32, camera_angle: f32) -> Direction {
    let mut angle_diff = (player_angle - camera_angle).to_degrees() + 180.0;
    if angle_diff < 0.0 {
        angle_diff += 360.0;
    }
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
        _ => Direction::Front,
    }
}

pub struct Renderer {
    pub buffer: Vec<u32>,
    pub z_buffer: Vec<f32>,
    pub texture_manager: TextureManager,
    pub sprite_sheet: SpriteSheet,
    pub sprites: Vec<Sprite>,
}

struct SpriteInfo<'a> {
    x: f32,
    y: f32,
    z: f32,
    texture: &'a String,
    width: f32,
    height: f32,
    dist_sq: f32,
    frame: Option<&'a crate::spritesheet::Frame>,
}

impl Renderer {
    pub fn new(texture_manager: TextureManager, sprite_sheet: SpriteSheet) -> Self {
        let sprites = vec![
            Sprite {
                x: 3.2,
                y: 4.3,
                z: 0.0,
                texture: "character2".to_string(),
                width: 0.2,
                height: 0.7,
            },
            Sprite {
                x: 4.2,
                y: 4.3,
                z: 0.0,
                texture: "character3".to_string(),
                width: 0.2,
                height: 0.7,
            },
        ];
        Renderer {
            buffer: vec![0; WIDTH * HEIGHT],
            z_buffer: vec![0.0; WIDTH],
            texture_manager,
            sprite_sheet,
            sprites,
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
                //let camera_height_offset = 0.1;
                let z_offset = ((player.z + CAMERA_HEIGHT_OFFSET) * line_height as f32) as isize;
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

            let mut sprite_infos: Vec<SpriteInfo> = self
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

            for (id, other_player) in &game_state.players {
                if id != &my_id.to_string() {
                    let direction = get_direction(other_player.angle, player.angle);
                    let frame = match other_player.animation_state {
                        crate::AnimationState::Idle => &self.sprite_sheet.idle[direction as usize],
                        crate::AnimationState::Walking => {
                            &self.sprite_sheet.walk[direction as usize][other_player.frame]
                        }
                    };

                    let sprite_x = other_player.x - player.x;
                    let sprite_y = other_player.y - player.y;
                    sprite_infos.push(SpriteInfo {
                        x: other_player.x,
                        y: other_player.y,
                        z: other_player.z,
                        texture: &other_player.texture,
                        width: 0.5,
                        height: 1.0,
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

            for sprite_info in sprite_infos {
                let sprite_x = sprite_info.x - player.x;
                let sprite_y = sprite_info.y - player.y;

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

                    let sprite_height = (HEIGHT as f32 / transform_y).abs() * sprite_info.height;
                    let world_half = (HEIGHT as f32 / transform_y).abs() * 0.5;
                    let sprite_vertical_offset = (player.z + CAMERA_HEIGHT_OFFSET - sprite_info.z)
                        * HEIGHT as f32
                        / transform_y
                        - sprite_height * 0.5
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

                    let sprite_width = (WIDTH as f32 / transform_y).abs() * sprite_info.width;
                    let draw_start_x = (sprite_screen_x - sprite_width / 2.0).max(0.0) as usize;
                    let draw_end_x =
                        (sprite_screen_x + sprite_width / 2.0).min(WIDTH as f32) as usize;

                    if let Some(frame) = sprite_info.frame {
                        for stripe in draw_start_x..draw_end_x {
                            if transform_y < self.z_buffer[stripe] {
                                let tex_x =
                                    ((stripe as f32 - (sprite_screen_x - sprite_width / 2.0))
                                        * frame.width as f32
                                        / sprite_width) as u32;

                                for y in draw_start_y..draw_end_y {
                                    let tex_y = ((y as f32
                                        - (HEIGHT as f32 / 2.0 - sprite_height / 2.0
                                            + pitch_offset as f32
                                            + sprite_vertical_offset as f32))
                                        * frame.height as f32
                                        / sprite_height)
                                        as u32;

                                    if tex_x < frame.width && tex_y < frame.height {
                                        let color =
                                            frame.pixels[(tex_y * frame.width + tex_x) as usize];
                                        let alpha = (color >> 24) & 0xFF;

                                        if alpha > 0 {
                                            self.buffer[y * WIDTH + stripe] = color;
                                        }
                                    }
                                }
                            }
                        }
                    } else if let Some(texture) =
                        self.texture_manager.get_texture(sprite_info.texture)
                    {
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

                                    if tex_x < texture.width && tex_y < texture.height {
                                        let color = texture.pixels
                                            [(tex_y * texture.width + tex_x) as usize];
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
}
