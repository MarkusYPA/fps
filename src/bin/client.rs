use anyhow::Result;
use std::io;
use std::net::UdpSocket;
use std::sync::Arc;
use std::time::{Duration, Instant};

use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use fps::{GameState, HEIGHT, Input, PORT, WIDTH};

const MOUSE_SPEED: f32 = 0.06;

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

            let perp_wall_dist = if wall_type == 0 {
                (map_x as f32 - player.x + (1.0 - step_x as f32) / 2.0) / ray_dir_x
            } else {
                (map_y as f32 - player.y + (1.0 - step_y as f32) / 2.0) / ray_dir_y
            };

            let line_height = (HEIGHT as f32 / perp_wall_dist) as isize;
            let draw_start = (-line_height / 2 + HEIGHT as isize / 2).max(0) as usize;
            let draw_end =
                (line_height / 2 + HEIGHT as isize / 2).min(HEIGHT as isize - 1) as usize;

            let wall_color = if wall_type == 1 {
                0x008A7755
            } else {
                0x00695A41
            };

            for y in draw_start..draw_end {
                self.buffer[y * WIDTH + x] = wall_color;
            }
        }
    }

    fn draw_to_buffer(&self, frame: &mut [u8]) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let color = self.buffer[i];
            let rgba = [(color >> 16) as u8, (color >> 8) as u8, color as u8, 0xFF];
            pixel.copy_from_slice(&rgba);
        }
    }
}

fn main() -> Result<()> {
    println!("Enter server IP address:");
    let mut server_ip = String::new();
    io::stdin().read_line(&mut server_ip)?;
    let ip_only = server_ip.trim().rsplitn(2, ':').last().unwrap().trim();
    let server_address = format!("{}:{}", ip_only, PORT);

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect(server_address)?;
    socket.set_nonblocking(true)?;

    let event_loop = EventLoop::new()?;
    let mut input = WinitInputHelper::new();
    let window = Arc::new({
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        WindowBuilder::new()
            .with_title("FPS Game")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)?
    });

    window.set_cursor_visible(false);
    window
        .set_cursor_grab(winit::window::CursorGrabMode::Confined)
        .or_else(|_e| window.set_cursor_grab(winit::window::CursorGrabMode::Locked))
        .unwrap();

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &*window);
        Pixels::new(WIDTH as u32, HEIGHT as u32, surface_texture)?
    };

    let mut renderer = Renderer::new();
    let mut game_state: Option<GameState> = None;

    let mut frame_count = 0;
    let mut fps_timer = Instant::now();
    let window_clone = window.clone();
    let mut mouse_dx = 0.0;

    Ok(event_loop.run(move |event, elwt| {
        match &event {
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                mouse_dx = delta.0 as f32;
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    elwt.exit();
                    return;
                }
                WindowEvent::RedrawRequested => {
                    if let Some(ref gs) = game_state {
                        renderer.render(gs);
                        renderer.draw_to_buffer(pixels.frame_mut());
                        if let Err(err) = pixels.render() {
                            eprintln!("pixels.render() failed: {}", err);
                            elwt.exit();
                            return;
                        }
                    }

                    frame_count += 1;
                    if fps_timer.elapsed() >= Duration::from_secs(1) {
                        let fps = frame_count;
                        frame_count = 0;
                        fps_timer = Instant::now();
                        window_clone.set_title(&format!("FPS Game - {} FPS", fps));
                    }
                }
                _ => (),
            },
            _ => (),
        }

        if input.update(&event) {
            if input.key_pressed(KeyCode::Escape) || input.close_requested() {
                elwt.exit();
                return;
            }

            let mut turn = mouse_dx * MOUSE_SPEED;
            if input.key_held(KeyCode::ArrowLeft) {
                turn -= 1.0;
            }
            if input.key_held(KeyCode::ArrowRight) {
                turn += 1.0;
            }

            let client_input = Input {
                forth: input.key_held(KeyCode::ArrowUp) || input.key_held(KeyCode::KeyW),
                back: input.key_held(KeyCode::ArrowDown) || input.key_held(KeyCode::KeyS),
                left: input.key_held(KeyCode::KeyA),
                right: input.key_held(KeyCode::KeyD),
                turn,
            };
            mouse_dx = 0.0;

            let encoded_input = bincode::serialize(&client_input).unwrap();
            if let Err(e) = socket.send(&encoded_input) {
                eprintln!("Error sending data: {}", e);
            }

            window_clone.request_redraw();
        }

        let mut buf = [0; 1024];
        let mut latest_game_state: Option<GameState> = None;

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
    })?)
}
