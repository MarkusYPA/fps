use crate::textures::{self};
use crate::{
    Direction, GameState, HEIGHT, Sprite, WIDTH, spritesheet::SpriteSheet, textures::TextureManager,
};

const CAMERA_HEIGHT_OFFSET: f32 = 0.1;

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
    pub sprite_sheet: SpriteSheet,
    pub sprite_sheet_test: SpriteSheet,
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
    frame: Option<&'a textures::Texture>,
}

impl Renderer {
    pub fn new(texture_manager: TextureManager, sprite_sheet: SpriteSheet, sprite_sheet_test: SpriteSheet) -> Self {
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
            sprite_sheet_test,
            sprites,
        }
    }

    pub fn render(&mut self, game_state: &GameState, my_id: u64) {
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

            // cast one ray for each pixel in width
            for x in 0..WIDTH {
                // ray direction
                let camera_x = 2.0 * x as f32 / WIDTH as f32 - 1.0;
                let ray_dir_x = player.angle.cos() + 0.66 * camera_x * (-player.angle.sin());
                let ray_dir_y = player.angle.sin() + 0.66 * camera_x * player.angle.cos();

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

                    if game_state.world.get_tile(map_x, map_y) == 1 {
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

                let wall_color = if wall_type == 1 {
                    0x008A7755
                } else {
                    0x00695A41
                };

                // save vertical wall line to buffer
                for y in draw_start..draw_end {
                    self.buffer[y * WIDTH + x] = wall_color;
                }
            }

            // sprites from world
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

            // sprites from other players
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
                        height: 0.7,
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

                let plane_x = -dir_y * 0.66;
                let plane_y = dir_x * 0.66;

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
    }

    pub fn draw_to_buffer(&self, frame: &mut [u8]) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let color = self.buffer[i];
            let rgba = [(color >> 16) as u8, (color >> 8) as u8, color as u8, 0xFF];
            pixel.copy_from_slice(&rgba);
        }
    }
}
