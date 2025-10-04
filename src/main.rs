use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 640;
const HEIGHT: usize = 480;

struct Player {
    x: f32,
    y: f32,
    angle: f32,
}

struct World {
    map: Vec<Vec<u8>>,
}

impl World {
    fn new() -> Self {
        World {
            map: vec![
                vec![1, 1, 1, 1, 1, 1, 1, 1],
                vec![1, 0, 0, 0, 0, 0, 0, 1],
                vec![1, 0, 1, 0, 0, 1, 0, 1],
                vec![1, 0, 0, 0, 0, 0, 0, 1],
                vec![1, 0, 0, 0, 0, 0, 0, 1],
                vec![1, 0, 1, 0, 0, 1, 0, 1],
                vec![1, 0, 0, 0, 0, 0, 0, 1],
                vec![1, 1, 1, 1, 1, 1, 1, 1],
            ],
        }
    }

    fn get_tile(&self, x: usize, y: usize) -> u8 {
        if x < self.map.len() && y < self.map[0].len() {
            self.map[y][x]
        } else {
            1 // Treat out of bounds as a wall
        }
    }
}

struct GameState {
    player: Player,
    world: World,
}

impl GameState {
    fn new() -> Self {
        GameState {
            player: Player {
                x: 1.5,
                y: 1.5,
                angle: std::f32::consts::PI / 2.0,
            },
            world: World::new(),
        }
    }

    fn update(&mut self, window: &Window) {
        let move_speed = 0.075;
        let strafe_speed = 0.05;
        let rot_speed = 0.05;

        let mut new_x = self.player.x;
        let mut new_y = self.player.y;

        // Move back and forth
        if window.is_key_down(Key::Up) {
            new_x += self.player.angle.cos() * move_speed;
            new_y += self.player.angle.sin() * move_speed;
        }
        if window.is_key_down(Key::Down) {
            new_x -= self.player.angle.cos() * move_speed;
            new_y -= self.player.angle.sin() * move_speed;
        }

        // Strafe left and right
        if window.is_key_down(Key::LeftAlt) {
            if window.is_key_down(Key::Left) {
                new_x += self.player.angle.sin() * strafe_speed;
                new_y -= self.player.angle.cos() * strafe_speed;
            }
            if window.is_key_down(Key::Right) {
                new_x -= self.player.angle.sin() * strafe_speed;
                new_y += self.player.angle.cos() * strafe_speed;
            }
        }

        // Collision detection
        if self.world.get_tile(new_x as usize, self.player.y as usize) == 0 {
            self.player.x = new_x;
        }
        if self.world.get_tile(self.player.x as usize, new_y as usize) == 0 {
            self.player.y = new_y;
        }

        // turn left and right
        if !window.is_key_down(Key::LeftAlt) {
            if window.is_key_down(Key::Left) {
                self.player.angle -= rot_speed;
            }
            if window.is_key_down(Key::Right) {
                self.player.angle += rot_speed;
            }
        }
    }
}

struct Renderer {
    buffer: Vec<u32>,
}

impl Renderer {
    fn new() -> Self {
        Renderer {
            buffer: vec![0; WIDTH * HEIGHT],
        }
    }

    fn render(&mut self, game_state: &GameState) {
        // Clear the buffer (floor and ceiling)
        for y in 0..HEIGHT / 2 {
            for x in 0..WIDTH {
                self.buffer[y * WIDTH + x] = 0x00AACCFF; // Ceiling (light blue)
            }
        }
        for y in HEIGHT / 2..HEIGHT {
            for x in 0..WIDTH {
                self.buffer[y * WIDTH + x] = 0x00555555; // Floor (dark gray)
            }
        }

        // Raycasting
        for x in 0..WIDTH {
            let camera_x = 2.0 * x as f32 / WIDTH as f32 - 1.0; // x-coordinate in camera space
            let ray_dir_x =
                game_state.player.angle.cos() + 0.66 * camera_x * (-game_state.player.angle.sin()); // 0.66 is the camera field of view?
            let ray_dir_y =
                game_state.player.angle.sin() + 0.66 * camera_x * game_state.player.angle.cos();

            let mut map_x = game_state.player.x as usize;
            let mut map_y = game_state.player.y as usize;

            let delta_dist_x = (1.0 + (ray_dir_y / ray_dir_x).powi(2)).sqrt();
            let delta_dist_y = (1.0 + (ray_dir_x / ray_dir_y).powi(2)).sqrt();

            let step_x;
            let step_y;

            let mut side_dist_x;
            let mut side_dist_y;

            if ray_dir_x < 0.0 {
                step_x = -1;
                side_dist_x = (game_state.player.x - map_x as f32) * delta_dist_x;
            } else {
                step_x = 1;
                side_dist_x = (map_x as f32 + 1.0 - game_state.player.x) * delta_dist_x;
            }
            if ray_dir_y < 0.0 {
                step_y = -1;
                side_dist_y = (game_state.player.y - map_y as f32) * delta_dist_y;
            } else {
                step_y = 1;
                side_dist_y = (map_y as f32 + 1.0 - game_state.player.y) * delta_dist_y;
            }

            let mut hit = false;
            let mut side = 0; // 0 for x-side, 1 for y-side
            while !hit {
                if side_dist_x < side_dist_y {
                    side_dist_x += delta_dist_x;
                    map_x = (map_x as isize + step_x) as usize;
                    side = 0;
                } else {
                    side_dist_y += delta_dist_y;
                    map_y = (map_y as isize + step_y) as usize;
                    side = 1;
                }

                if game_state.world.get_tile(map_x, map_y) == 1 {
                    hit = true;
                }
            }

            let perp_wall_dist;
            if side == 0 {
                perp_wall_dist =
                    (map_x as f32 - game_state.player.x + (1.0 - step_x as f32) / 2.0) / ray_dir_x;
            } else {
                perp_wall_dist =
                    (map_y as f32 - game_state.player.y + (1.0 - step_y as f32) / 2.0) / ray_dir_y;
            }

            let line_height = (HEIGHT as f32 / perp_wall_dist) as isize;

            let draw_start = -line_height / 2 + HEIGHT as isize / 2;
            let draw_end = line_height / 2 + HEIGHT as isize / 2;

            let wall_color = if side == 1 { 0x008A7755 } else { 0x00695A41 }; // lighter or darker

            for y in 0..HEIGHT {
                if y as isize >= draw_start && y as isize <= draw_end {
                    self.buffer[y * WIDTH + x] = wall_color;
                }
            }
        }
    }
}

fn main() {
    let mut window = Window::new(
        "FPS Game - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    // Limit to 60 fps
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    let mut game_state = GameState::new();
    let mut renderer = Renderer::new();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        game_state.update(&window);
        renderer.render(&game_state);

        window
            .update_with_buffer(&renderer.buffer, WIDTH, HEIGHT)
            .unwrap();
    }
}
