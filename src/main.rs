use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 1024; // 1024  1280
const HEIGHT: usize = 768; // 768   960

struct Input {
    forth: bool,
    back: bool,
    left: bool,
    right: bool,
    strafe: bool,
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

        // Slow down movement when strafing and going backward or forward
        let mut slower = 1.0;
        if input.strafe
            && (input.left || input.right)
            && (input.forth || input.back)
        {
            slower = 0.707;
        }

        if input.forth {
            new_x += self.angle.cos() * self.move_speed * slower;
            new_y += self.angle.sin() * self.move_speed * slower;
        }

        if input.back {
            new_x -= self.angle.cos() * self.move_speed * slower;
            new_y -= self.angle.sin() * self.move_speed * slower;
        }

        if input.strafe {
            let strafe_x = -self.angle.sin();
            let strafe_y = self.angle.cos();

            if input.right {
                new_x += strafe_x * self.move_speed * slower;
                new_y += strafe_y * self.move_speed * slower;
            }
            if input.left {
                new_x -= strafe_x * self.move_speed * slower;
                new_y -= strafe_y * self.move_speed * slower;
            }
        }

        self.check_collision_and_move(new_x, new_y, world);

        if !input.strafe {
            if input.left {
                self.angle -= self.rot_speed;
            }
            if input.right {
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

        // Raycasting per column
        let player = &game_state.player;
        for x in 0..WIDTH {
            // Map screen x coordinate to camera space (-1.0 left .. 1.0 right)
            let camera_x = 2.0 * x as f32 / WIDTH as f32 - 1.0;

            // Ray direction for this column: Offset forward vector by camera_x.
            // The 0.66 is basically half the FOV scaling factor.
            let ray_dir_x = player.angle.cos() + 0.66 * camera_x * (-player.angle.sin());
            let ray_dir_y = player.angle.sin() + 0.66 * camera_x * player.angle.cos();

            // Current square of the map the ray starts in
            let mut map_x = player.x as usize;
            let mut map_y = player.y as usize;

            // Length of ray from one x- or y-wall to the next
            let delta_dist_x = (1.0 + (ray_dir_y / ray_dir_x).powi(2)).sqrt();
            let delta_dist_y = (1.0 + (ray_dir_x / ray_dir_y).powi(2)).sqrt();

            // Step direction (+1 or -1), and distance to first wall
            let step_x;
            let step_y;
            let mut wall_dist_x;
            let mut wall_dist_y;

            // Figure out step and initial wall distances
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

            // Perform DDA (Digital Differential Analyzer) until wall is hit
            let mut hit = false;
            let mut wall_type = 0; // 0 = hit nort-south wall , 1 = hit east-west wall
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

            // Distance to wall (perpendicular to avoid fisheye effect)
            let perp_wall_dist;
            if wall_type == 0 {
                perp_wall_dist =
                    (map_x as f32 - player.x + (1.0 - step_x as f32) / 2.0) / ray_dir_x;
            } else {
                perp_wall_dist =
                    (map_y as f32 - player.y + (1.0 - step_y as f32) / 2.0) / ray_dir_y;
            }

            // Wall height and corresponding line
            let line_height = (HEIGHT as f32 / perp_wall_dist) as isize;
            let draw_start = -line_height / 2 + HEIGHT as isize / 2;
            let draw_end = line_height / 2 + HEIGHT as isize / 2;

            // Wall darkness from its orientation
            let wall_color = if wall_type == 1 { 0x008A7755 } else { 0x00695A41 };

            // Wall slice into buffer
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
        forth: window.is_key_down(Key::Up),
        back: window.is_key_down(Key::Down),
        left: window.is_key_down(Key::Left),
        right: window.is_key_down(Key::Right),
        strafe: window.is_key_down(Key::LeftAlt) || window.is_key_down(Key::LeftShift),
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
