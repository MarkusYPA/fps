use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 1024; // 1024  1280
const HEIGHT: usize = 768; // 768   960

struct Input {
    up_pressed: bool,
    down_pressed: bool,
    left_pressed: bool,
    right_pressed: bool,
    left_alt_pressed: bool,
}

struct Player {
    x: f32,
    y: f32,
    angle: f32,
    move_speed: f32,
    rot_speed: f32,
}

impl Player {
    fn new() -> Self {
        Player {
            x: 1.5,
            y: 1.5,
            angle: std::f32::consts::PI / 2.0,
            move_speed: 0.05,
            rot_speed: 0.03,
        }
    }

    fn take_input(&mut self, input: &Input, world: &World) {
        let mut new_x = self.x;
        let mut new_y = self.y;

        // TODO: slow down movement when strafing and going backward or forward

        if input.up_pressed {
            new_x += self.angle.cos() * self.move_speed;
            new_y += self.angle.sin() * self.move_speed;
        }
        if input.down_pressed {
            new_x -= self.angle.cos() * self.move_speed;
            new_y -= self.angle.sin() * self.move_speed;
        }

        if input.left_alt_pressed {
            let strafe_x = -self.angle.sin();
            let strafe_y = self.angle.cos();

            if input.right_pressed {
                new_x += strafe_x * self.move_speed;
                new_y += strafe_y * self.move_speed;
            }
            if input.left_pressed {
                new_x -= strafe_x * self.move_speed;
                new_y -= strafe_y * self.move_speed;
            }
        }

        self.check_collision_and_move(new_x, new_y, world);

        if !input.left_alt_pressed {
            if input.left_pressed {
                self.angle -= self.rot_speed;
            }
            if input.right_pressed {
                self.angle += self.rot_speed;
            }
        }
    }

    fn check_collision_and_move(&mut self, new_x: f32, new_y: f32, world: &World) {
        if world.get_tile(new_x as usize, self.y as usize) == 0 {
            self.x = new_x;
        }
        if world.get_tile(self.x as usize, new_y as usize) == 0 {
            self.y = new_y;
        }
    }
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
            player: Player::new(),
            world: World::new(),
        }
    }

    fn update(&mut self, input: &Input) {
        self.player.take_input(input, &self.world);
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
        let player = &game_state.player;
        for x in 0..WIDTH {
            let camera_x = 2.0 * x as f32 / WIDTH as f32 - 1.0; // x-coordinate in camera space
            let ray_dir_x = player.angle.cos() + 0.66 * camera_x * (-player.angle.sin()); // 0.66 is the camera field of view?
            let ray_dir_y = player.angle.sin() + 0.66 * camera_x * player.angle.cos();

            let mut map_x = player.x as usize;
            let mut map_y = player.y as usize;

            let delta_dist_x = (1.0 + (ray_dir_y / ray_dir_x).powi(2)).sqrt();
            let delta_dist_y = (1.0 + (ray_dir_x / ray_dir_y).powi(2)).sqrt();

            let step_x;
            let step_y;

            let mut side_dist_x;
            let mut side_dist_y;

            if ray_dir_x < 0.0 {
                step_x = -1;
                side_dist_x = (player.x - map_x as f32) * delta_dist_x;
            } else {
                step_x = 1;
                side_dist_x = (map_x as f32 + 1.0 - player.x) * delta_dist_x;
            }
            if ray_dir_y < 0.0 {
                step_y = -1;
                side_dist_y = (player.y - map_y as f32) * delta_dist_y;
            } else {
                step_y = 1;
                side_dist_y = (map_y as f32 + 1.0 - player.y) * delta_dist_y;
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
                    (map_x as f32 - player.x + (1.0 - step_x as f32) / 2.0) / ray_dir_x;
            } else {
                perp_wall_dist =
                    (map_y as f32 - player.y + (1.0 - step_y as f32) / 2.0) / ray_dir_y;
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

fn handle_input(window: &Window) -> Input {
    Input {
        up_pressed: window.is_key_down(Key::Up),
        down_pressed: window.is_key_down(Key::Down),
        left_pressed: window.is_key_down(Key::Left),
        right_pressed: window.is_key_down(Key::Right),
        left_alt_pressed: window.is_key_down(Key::LeftShift),
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

    window.set_target_fps(60);

    let mut game_state = GameState::new();
    let mut renderer = Renderer::new();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let input = handle_input(&window);
        game_state.update(&input);
        renderer.render(&game_state);

        window
            .update_with_buffer(&renderer.buffer, WIDTH, HEIGHT)
            .unwrap();
    }
}
