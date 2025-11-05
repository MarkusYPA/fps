use anyhow::Result;
use std::io::{self, Write};
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

use fps::{
    ClientMessage, GameState, HEIGHT, Input, PORT, ServerMessage, WIDTH, renderer::Renderer,
    textures::TextureManager,
};

const MOUSE_SPEED: f32 = 0.06;

fn main() -> Result<()> {
    println!("Enter server IP address:");
    let mut server_ip = String::new();
    io::stdin().read_line(&mut server_ip)?;
    let ip_only = server_ip.trim().rsplitn(2, ':').last().unwrap().trim();
    let server_address = format!("{}:{}", ip_only, PORT);

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect(&server_address)?;
    socket.set_nonblocking(true)?;

    let mut buf = [0; 1024];
    let mut my_id: Option<u64> = None;

    // Outer loop: continue until successfully connected
    loop {
        print!("Enter a username: ");
        io::stdout().flush()?; // ensure prompt is printed
        let mut username = String::new();
        io::stdin().read_line(&mut username)?;
        let username = username.trim().to_string();

        // Send connect message
        let connect_message = ClientMessage::Connect(username.clone());
        let encoded = bincode::serialize(&connect_message)?;
        socket.send(&encoded)?;

        // Wait for a response with timeout
        let start = Instant::now();
        let timeout = Duration::from_secs(2);
        let mut got_response = false;

        while start.elapsed() < timeout {
            match socket.recv_from(&mut buf) {
                Ok((amt, _)) => {
                    if let Ok(server_message) = bincode::deserialize::<ServerMessage>(&buf[..amt]) {
                        match server_message {
                            ServerMessage::Welcome(welcome) => {
                                println!("Connected to server with id: {}", welcome.id);
                                my_id = Some(welcome.id);
                                got_response = true;
                                break;
                            }
                            ServerMessage::UsernameRejected(reason) => {
                                eprintln!("Connection rejected: {}", reason);
                                // prompt for a new username
                                got_response = true;
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    eprintln!("Error receiving message: {}", e);
                    return Err(e.into());
                }
            }
        }

        if got_response {
            // If we connected successfully, exit loop
            if let Ok(ServerMessage::Welcome(_)) = bincode::deserialize::<ServerMessage>(&buf[..]) {
                break;
            } else {
                continue; // username rejected, ask again
            }
        } else {
            eprintln!("No response from server, retrying...");
        }
    }

    let my_id = my_id.ok_or_else(|| anyhow::anyhow!("Failed to receive welcome message"))?;

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

    let mut texture_manager = TextureManager::new();
    fps::textures::load_game_textures(&mut texture_manager)?;

    let mut renderer = Renderer::new(texture_manager);
    let mut game_state: Option<GameState> = None;

    let mut frame_count = 0;
    let mut fps_timer = Instant::now();
    let window_clone = window.clone();
    let mut mouse_dx = 0.0;
    let mut mouse_dy = 0.0;
    let mut prev_input: Option<Input> = None;

    Ok(event_loop.run(move |event, elwt| {
        match &event {
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                mouse_dx = delta.0 as f32;
                mouse_dy = delta.1 as f32;
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    elwt.exit();
                    return;
                }
                WindowEvent::RedrawRequested => {
                    if let Some(ref gs) = game_state {
                        renderer.render(gs, my_id);
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
                pitch: -mouse_dy * MOUSE_SPEED, // Invert mouse_dy for natural pitch control
                jump: input.key_pressed(KeyCode::Space),
            };
            mouse_dx = 0.0;
            mouse_dy = 0.0;

            if Some(client_input.clone()) != prev_input {
                let encoded_input =
                    bincode::serialize(&ClientMessage::Input(client_input.clone())).unwrap();
                if let Err(e) = socket.send(&encoded_input) {
                    eprintln!("Error sending data: {}", e);
                }
                prev_input = Some(client_input.clone());
            }
        }

        let mut buf = [0; 1024];

        loop {
            match socket.recv(&mut buf) {
                Ok(amt) => {
                    if let Ok(server_message) = bincode::deserialize::<ServerMessage>(&buf[..amt]) {
                        match server_message {
                            ServerMessage::Welcome(_) => {
                                // This should not happen after initial connection
                                eprintln!("Received unexpected Welcome message");
                            }
                            ServerMessage::InitialState(initial_state) => {
                                game_state = Some(initial_state);
                            }
                            ServerMessage::GameUpdate(player_updates) => {
                                if let Some(ref mut gs) = game_state {
                                    for (id, update) in player_updates {
                                        if let Some(player) = gs.players.get_mut(&id) {
                                            player.x = update.x;
                                            player.y = update.y;
                                            player.z = update.z;
                                            player.angle = update.angle;
                                            player.pitch = update.pitch;
                                        } else {
                                            // New player joined — insert into local game state
                                            let mut p = fps::Player::new();
                                            p.x = update.x;
                                            p.y = update.y;
                                            p.z = update.z;
                                            p.angle = update.angle;
                                            p.pitch = update.pitch;
                                            gs.players.insert(id, p);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
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

        window_clone.request_redraw();
    })?)
}
