use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::cmp;
use std::collections::HashMap;
use std::io::{self, Write};
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::{Duration, Instant};

use pixels::{Pixels, SurfaceTexture};
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{DeviceEvent, Event, MouseButton, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::window::{CursorGrabMode, Window, WindowBuilder};
use winit_input_helper::WinitInputHelper;

use fps::{
    AnimationState::{Dying, Walking},
    ClientMessage, Input, ServerMessage,
    consts::{DIE_FRAME_TIME, HEIGHT, MOUSE_SPEED, MOUSE_SENSITIVITY_MAX, MOUSE_SENSITIVITY_MIN, PORT, SHOOT_COOLDOWN, WALK_FRAME_TIME, WIDTH},
    gamestate::GameState,
    player::Player,
    renderer::{MenuHover, Renderer},
    spritesheet::hue_variations,
    textures::TextureManager,
};

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    last_name: Option<String>,
    recent_servers: Vec<String>,
    mouse_sensitivity: Option<f32>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            last_name: None,
            recent_servers: Vec::new(),
            mouse_sensitivity: None,
        }
    }
}

fn save_config(config: &Config) -> Result<()> {
    let config_path = "client_config.toml";
    let config_str = toml::to_string_pretty(config)?;
    std::fs::write(config_path, config_str)?;
    Ok(())
}

fn load_config() -> Config {
    let config_path = "client_config.toml";
    std::fs::read_to_string(config_path)
        .ok()
        .and_then(|content| toml::from_str(&content).ok())
        .unwrap_or_default()
}

fn connect_to_server() -> Result<Option<(UdpSocket, u64, String)>> {
    let mut config = load_config();

    loop {
        // Get server IP
        println!("Select a server or enter a new IP:");
        for (i, server) in config.recent_servers.iter().enumerate() {
            println!("{}: {}", i + 1, server);
        }
        print!(
            "Enter selection (1-{}, default: 1), or new IP: ",
            config.recent_servers.len()
        );
        io::stdout().flush()?;

        let mut selection = String::new();
        io::stdin().read_line(&mut selection)?;
        let selection = selection.trim();

        let server_address_str = if selection.is_empty() {
            if let Some(first) = config.recent_servers.get(0) {
                first.clone()
            } else {
                println!("No recent servers, please enter an IP:");
                let mut server_ip = String::new();
                io::stdin().read_line(&mut server_ip)?;
                server_ip.trim().to_string()
            }
        } else if let Ok(num) = selection.parse::<usize>() {
            if num > 0 && num <= config.recent_servers.len() {
                config.recent_servers.get(num - 1).cloned().unwrap()
            } else {
                println!("Invalid selection. Please enter a new IP:");
                let mut server_ip = String::new();
                io::stdin().read_line(&mut server_ip)?;
                server_ip.trim().to_string()
            }
        } else {
            selection.to_string()
        };

        let server_address: SocketAddr = if server_address_str.contains(':') {
            server_address_str.parse()?
        } else {
            format!("{}:{}", server_address_str, PORT).parse()?
        };

        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.connect(server_address)?;
        socket.set_nonblocking(true)?;

        let mut buf = [0; 2048];

        // Inner loop for username attempts
        loop {
            print!(
                "Enter a username (default: {}): ",
                config.last_name.as_deref().unwrap_or("")
            );
            io::stdout().flush()?;
            let mut username_input = String::new();
            io::stdin().read_line(&mut username_input)?;
            let username_trimmed = username_input.trim();

            let final_username = if username_trimmed.is_empty() {
                config.last_name.clone().unwrap_or_default()
            } else {
                username_trimmed.to_string()
            };

            if final_username.is_empty() {
                println!("Username cannot be empty.");
                continue;
            }

            // Send connect message
            let connect_message = ClientMessage::Connect(final_username.clone());
            let encoded = bincode::serialize(&connect_message)?;
            socket.send(&encoded)?;

            // Wait for a response with timeout
            let start = Instant::now();
            let timeout = Duration::from_secs(2);
            let mut got_response = false;

            while start.elapsed() < timeout {
                match socket.recv_from(&mut buf) {
                    Ok((amt, _)) => {
                        if let Ok(server_message) =
                            bincode::deserialize::<ServerMessage>(&buf[..amt])
                        {
                            match server_message {
                                ServerMessage::Welcome(welcome) => {
                                    println!("Connected to server with id: {}", welcome.id);

                                    // Update and save config
                                    config.last_name = Some(final_username.clone());
                                    let addr_string = server_address.to_string();
                                    config.recent_servers.retain(|s| s != &addr_string);
                                    config.recent_servers.insert(0, addr_string);
                                    config.recent_servers.truncate(5);
                                    save_config(&config)?;

                                    return Ok(Some((socket, welcome.id, final_username)));
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
                    Err(e) => return Err(e.into()),
                }
            }

            if got_response {
                // Username was rejected, loop again for a new username
                continue;
            } else {
                eprintln!("No response from server. Check the IP and server status.");
                break; // Breaks inner loop to re-prompt for IP
            }
        }

        print!("Try again with a different IP? (y/n): ");
        io::stdout().flush()?;
        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        if choice.trim().to_lowercase() != "y" {
            return Ok(None); // Exit if user doesn't want to retry
        }
    }
}

fn main() -> Result<()> {
    let (socket, my_id, _username) = match connect_to_server()? {
        Some(conn) => conn,
        None => return Ok(()), // User chose to exit
    };

    let socket_clone = socket.try_clone()?;
    std::thread::spawn(move || {
        loop {
            let ping_message = ClientMessage::Ping;
            let encoded = bincode::serialize(&ping_message).unwrap();
            if let Err(e) = socket_clone.send(&encoded) {
                eprintln!("Error sending ping: {}", e);
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    });

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

    // move cursor to center of window to prevent clicking elsewhere and don't allow it to move or show
    center_and_grab_cursor(window.clone());
    let mut cursor_grabbed = true;
    let mut first_mouse_move = true; // auto-moving mouse to center is not input

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &*window);
        Pixels::new(WIDTH as u32, HEIGHT as u32, surface_texture)?
    };

    // generate hue variations of the spritesheet, if they don't already exist
    hue_variations("assets/blob0.png");

    // define spritesheets
    let mut texture_manager = TextureManager::new();
    fps::textures::load_game_textures(&mut texture_manager)?;
    let mut spritesheets = HashMap::new();
    for i in 0..10 {
        spritesheets.insert(
            format!("{i}"), // key matches a player's texture property
            fps::spritesheet::SpriteSheet::new(&format!("assets/blob{i}.png"))?,
        );
    }
    let mut renderer = Renderer::new(texture_manager, spritesheets);
    let mut game_state: Option<GameState> = None;

    let mut frame_count = 0;
    let mut fps_timer = Instant::now();
    let window_clone = window.clone();
    let mut mouse_dx = 0.0;
    let mut mouse_dy = 0.0;
    let mut prev_input: Option<Input> = None;
    let mut focused = false;
    let mut last_frame_time = Instant::now();
    let mut last_shot_timestamp = Instant::now().checked_sub(SHOOT_COOLDOWN).unwrap_or(Instant::now());
    let mut show_menu = false;
    let mut config = load_config();
    let mut mouse_sensitivity = config
        .mouse_sensitivity
        .unwrap_or(MOUSE_SPEED)
        .clamp(MOUSE_SENSITIVITY_MIN, MOUSE_SENSITIVITY_MAX);
    let mut cursor_pos = (0.0, 0.0);
    let mut menu_hovered_item: Option<MenuHover> = None;

    Ok(event_loop.run(move |event, elwt| {
        let delta_time = last_frame_time.elapsed().as_secs_f32();
        last_frame_time = Instant::now();

        match &event {
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                if !first_mouse_move && cursor_grabbed && focused && !show_menu {
                    mouse_dx = delta.0 as f32;
                    mouse_dy = delta.1 as f32;
                } else {
                    first_mouse_move = false
                }
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    elwt.exit();
                    return;
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if show_menu {
                        cursor_pos = (position.x as f32, position.y as f32);
                    }
                }
                WindowEvent::Focused(is_focused) => {
                    focused = *is_focused;
                    center_and_grab_cursor(window_clone.clone());
                    first_mouse_move = true;
                }
                WindowEvent::RedrawRequested => {
                    if let Some(ref gs) = game_state {
                        renderer.render(gs, my_id);
                        renderer.draw_to_buffer(pixels.frame_mut());
                        renderer.display_health(gs, my_id, pixels.frame_mut());
                        renderer.display_leaderboard(gs, pixels.frame_mut());

                        if let Some(winner) = &gs.winner {
                            renderer.display_winner(&winner, pixels.frame_mut());
                        }

                        // Display menu if it's open
                        if show_menu {
                            renderer.display_menu(mouse_sensitivity, pixels.frame_mut(), menu_hovered_item);
                        }

                        frame_count += 1;
                        if fps_timer.elapsed() >= Duration::from_secs(1) {
                            let fps = frame_count;
                            frame_count = 0;
                            fps_timer = Instant::now();
                            window_clone.set_title(&format!("Blob Hunter 3-D - {} FPS", fps));
                        }

                        if let Err(err) = pixels.render() {
                            eprintln!("pixels.render() failed: {}", err);
                            elwt.exit();
                            return;
                        }
                    }
                }
                _ => (),
            },
            _ => (),
        }

        if input.update(&event) {
            if input.close_requested() {
                elwt.exit();
                return;
            }
            if input.key_pressed(KeyCode::Escape) {
                show_menu = !show_menu;
                if show_menu {
                    cursor_grabbed = false;
                    window_clone.set_cursor_visible(true);
                    window_clone.set_cursor_grab(CursorGrabMode::None).unwrap();

                    // Clear inputs and send zero input to server to make sure character stops when menu is opened
                    mouse_dx = 0.0;
                    mouse_dy = 0.0;

                    if let Some(ref gs) = game_state {
                        if gs.winner.is_none() {
                            let zero_input = Input {
                                forth: false,
                                back: false,
                                left: false,
                                right: false,
                                turn: 0.0,
                                pitch: 0.0,
                                jump: false,
                                sprint: false,
                                shoot: false,
                            };
                            let encoded_input = bincode::serialize(&ClientMessage::Input(zero_input)).unwrap();
                            if let Err(e) = socket.send(&encoded_input) {
                                eprintln!("Error sending zero input: {}", e);
                            }
                            prev_input = None;
                        }
                    }
                } else {
                    // Re-grab cursor when menu closes
                    center_and_grab_cursor(window_clone.clone());
                    cursor_grabbed = true;
                    first_mouse_move = true;
                }
            }

            if show_menu {
                // Update hover state and handle menu clicks
                let (quit_bounds, sens_bounds) = renderer.get_menu_item_bounds(mouse_sensitivity);
                menu_hovered_item = if quit_bounds.contains(cursor_pos.0, cursor_pos.1) {
                    Some(MenuHover::Quit)
                } else if sens_bounds.contains(cursor_pos.0, cursor_pos.1) {
                    Some(MenuHover::MouseSensitivity)
                } else {
                    None
                };

                let mut sensitivity_changed = false;
                if input.mouse_pressed(MouseButton::Left) {
                    if quit_bounds.contains(cursor_pos.0, cursor_pos.1) {
                        elwt.exit();
                        return;
                    } else if sens_bounds.contains(cursor_pos.0, cursor_pos.1) {
                        mouse_sensitivity += 0.01;
                        if mouse_sensitivity > MOUSE_SENSITIVITY_MAX {
                            mouse_sensitivity = MOUSE_SENSITIVITY_MIN;
                        }
                        sensitivity_changed = true;
                    }
                } else if input.mouse_pressed(MouseButton::Right) {
                    if sens_bounds.contains(cursor_pos.0, cursor_pos.1) {
                        mouse_sensitivity -= 0.01;
                        if mouse_sensitivity < MOUSE_SENSITIVITY_MIN {
                            mouse_sensitivity = MOUSE_SENSITIVITY_MAX;
                        }
                        sensitivity_changed = true;
                    }
                }

                if sensitivity_changed {
                    config.mouse_sensitivity = Some(mouse_sensitivity);
                    if let Err(e) = save_config(&config) {
                        eprintln!("Error saving config: {}", e);
                    }
                }
            } else {
                menu_hovered_item = None;
            }
            if !show_menu && game_state.as_ref().map(|gs| gs.winner.is_none()).unwrap_or(false) {
                if input.key_pressed(KeyCode::Tab) {
                    cursor_grabbed = !cursor_grabbed;
                    window_clone.set_cursor_visible(!cursor_grabbed);
                    let grab_mode = if cursor_grabbed {
                        CursorGrabMode::Confined
                    } else {
                        CursorGrabMode::None
                    };
                    window_clone
                        .set_cursor_grab(grab_mode)
                        .or_else(|_e| {
                            if cursor_grabbed {
                                window_clone.set_cursor_grab(CursorGrabMode::Locked)
                            } else {
                                window_clone.set_cursor_grab(CursorGrabMode::None)
                            }
                        })
                        .unwrap();
                }

                let mut turn = mouse_dx * mouse_sensitivity;
                if input.key_held(KeyCode::ArrowLeft) {
                    turn -= 1.0;
                }
                if input.key_held(KeyCode::ArrowRight) {
                    turn += 1.0;
                }

                let can_shoot = last_shot_timestamp.elapsed() >= SHOOT_COOLDOWN;
                let mouse_pressed = input.mouse_pressed(MouseButton::Left);
                
                if mouse_pressed && can_shoot {
                    let shot_message = ClientMessage::Shot;
                    let encoded_shot = bincode::serialize(&shot_message).unwrap();
                    if let Err(e) = socket.send(&encoded_shot) {
                        eprintln!("Error sending shot data: {}", e);
                    } else {
                        last_shot_timestamp = Instant::now();
                    }
                }

                let client_input = Input {
                    forth: input.key_held(KeyCode::ArrowUp) || input.key_held(KeyCode::KeyW),
                    back: input.key_held(KeyCode::ArrowDown) || input.key_held(KeyCode::KeyS),
                    left: input.key_held(KeyCode::KeyA),
                    right: input.key_held(KeyCode::KeyD),
                    turn,
                    pitch: -mouse_dy * mouse_sensitivity, // Invert mouse_dy for natural pitch control
                    jump: input.key_pressed(KeyCode::Space),
                    sprint: input.key_held(KeyCode::ShiftLeft),
                    shoot: mouse_pressed && can_shoot,
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
        }

        let mut buf = [0; 2048];

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
                                            player.texture = update.texture;
                                            player.animation_state = update.animation_state;
                                            player.shooting = update.shooting;
                                            player.health = update.health;
                                            player.score = update.score;
                                        } else {
                                            // New player joined — insert into local game state
                                            let mut p = Player::new("0".to_string(), &gs.world);
                                            p.x = update.x;
                                            p.y = update.y;
                                            p.z = update.z;
                                            p.angle = update.angle;
                                            p.pitch = update.pitch;
                                            p.texture = update.texture;
                                            p.animation_state = update.animation_state;
                                            p.shooting = update.shooting;
                                            p.direction = fps::Direction::Front;
                                            gs.players.insert(id.clone(), p);
                                        }
                                    }
                                }
                            }
                            ServerMessage::SpriteUpdate(new_sprites) => {
                                if let Some(ref mut gs) = game_state {
                                    gs.floor_sprites = new_sprites;
                                }
                            }
                            ServerMessage::PlayerLeft(id) => {
                                if let Some(ref mut gs) = game_state {
                                    gs.players.remove(&id.to_string());
                                }
                            }
                            ServerMessage::ShotHit(hit) => {
                                if hit.shooter_id == my_id {
                                    println!("I shot {}", hit.target_name);
                                    // Flash a hit marker for successful hit
                                    renderer.show_hit_marker(0x00FFFFFF);
                                } else if hit.target_id == my_id {
                                    println!("{} shot me", hit.shooter_name);
                                }
                            }
                            ServerMessage::LeaderboardUpdate(leaderboard) => {
                                if let Some(ref mut gs) = game_state {
                                    gs.leaderboard = leaderboard;
                                }
                            }
                            ServerMessage::Winner(winner) => {
                                if let Some(ref mut gs) = game_state {
                                    gs.winner = Some(winner);
                                }
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(ref e) if e.kind() == io::ErrorKind::ConnectionRefused => {
                    eprintln!("Connection to the server was lost.");
                    elwt.exit();
                    return;
                }
                Err(e) => {
                    eprintln!("Error receiving data: {}", e);
                    break;
                }
            }
        }

        if let Some(gs) = &mut game_state {
            for player in gs.players.values_mut() {
                if player.animation_state == Walking {
                    player.frame_timer += delta_time;
                    if player.frame_timer > WALK_FRAME_TIME {
                        player.frame_timer = 0.0;
                        player.frame = (player.frame + 1) % 4;
                    }
                } else if player.animation_state == Dying {
                    player.frame_timer += delta_time;
                    if player.frame_timer > DIE_FRAME_TIME {
                        player.frame_timer = 0.0;
                        player.frame = cmp::min(player.frame + 1, 2);
                    }
                } else {
                    player.frame = 0;
                }
            }
        }

        window_clone.request_redraw();
    })?)
}

fn center_and_grab_cursor(window: Arc<Window>) {
    let size = window.inner_size();
    let center_x = size.width / 2;
    let center_y = size.height / 2;

    window
        .set_cursor_position(PhysicalPosition::new(center_x, center_y))
        .unwrap();

    window.set_cursor_visible(false);
    window
        .set_cursor_grab(CursorGrabMode::Confined)
        .or_else(|_e| window.set_cursor_grab(CursorGrabMode::Locked))
        .unwrap();
}
