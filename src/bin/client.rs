use minifb::{Key, Window, WindowOptions};
use std::io;
use std::net::UdpSocket;
use std::time::{Duration, Instant};

use fps::{GameState, HEIGHT, Input, WIDTH};

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

        let player = &game_state.player;
        for x in 0..WIDTH {
            let camera_x = 2.0 * x as f32 / WIDTH as f32 - 1.0;
            let ray_dir_x = player.angle.cos() + 0.66 * camera_x * (-player.angle.sin());
            let ray_dir_y = player.angle.sin() + 0.66 * camera_x * player.angle.cos();

            let mut map_x = player.x as usize;
            let mut map_y = player.y as usize;

            let delta_dist_x = (1.0 + (ray_dir_y / ray_dir_x).powi(2)).sqrt();
            let delta_dist_y = (1.0 + (ray_dir_x / ray_dir_y).powi(2)).sqrt();

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

            let perp_wall_dist;
            if wall_type == 0 {
                perp_wall_dist =
                    (map_x as f32 - player.x + (1.0 - step_x as f32) / 2.0) / ray_dir_x;
            } else {
                perp_wall_dist =
                    (map_y as f32 - player.y + (1.0 - step_y as f32) / 2.0) / ray_dir_y;
            }

            let line_height = (HEIGHT as f32 / perp_wall_dist) as isize;
            let draw_start = -line_height / 2 + HEIGHT as isize / 2;
            let draw_end = line_height / 2 + HEIGHT as isize / 2;

            let wall_color = if wall_type == 1 {
                0x008A7755
            } else {
                0x00695A41
            };

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

fn main() -> io::Result<()> {
    println!("Enter server IP address:");
    let mut server_ip = String::new();
    io::stdin().read_line(&mut server_ip)?;
    let server_address = format!("{}:8080", server_ip.trim());

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect(server_address)?;
    socket.set_nonblocking(true)?;

    let mut window = Window::new(
        "FPS Game - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap();

    window.set_target_fps(60);

    let mut renderer = Renderer::new();
    let mut game_state: Option<GameState> = None;

    let mut frame_count = 0;
    let mut fps_timer = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let input = handle_input(&window);
        let encoded_input = bincode::serialize(&input).unwrap();
        socket.send(&encoded_input)?;

        let mut buf = [0; 1024];
        let mut latest_game_state: Option<GameState> = None;

        // call recv until WouldBlock error -> ensure all data is read
        loop {
            match socket.recv(&mut buf) {
                Ok(amt) => {
                    if let Ok(decoded_state) = bincode::deserialize(&buf[..amt]) {
                        latest_game_state = Some(decoded_state);
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => {
                    eprintln!("Error receiving data: {}", e);
                    break;
                }
            }
        }

        if latest_game_state.is_some() {
            game_state = latest_game_state;
        }

        if let Some(ref gs) = game_state {
            renderer.render(gs);
            window
                .update_with_buffer(&renderer.buffer, WIDTH, HEIGHT)
                .unwrap();
        }

        frame_count += 1;
        if fps_timer.elapsed() >= Duration::from_secs(1) {
            let fps = frame_count;
            frame_count = 0;
            fps_timer = Instant::now();
            window.set_title(&format!("FPS Game - {} FPS", fps));
        }
    }

    Ok(())
}
