use fps::{
    ClientMessage, GameState, PlayerUpdate, ServerMessage, Welcome,
    consts::PORT,
    consts::{CAMERA_HEIGHT_OFFSET, SPRITE_OTHER_PLAYER_HEIGHT, SPRITE_OTHER_PLAYER_WIDTH},
    flags,
    player::Player,
};
use local_ip_address::local_ip;
use rand::prelude::*;
use rand::rng;
use std::{
    collections::HashMap,
    env,
    net::{SocketAddr, UdpSocket},
    time::{Duration, Instant},
};

fn main() -> std::io::Result<()> {
    let parsed_flags = flags::parse_flags(env::args()).expect("Failed to parse flags");

    let my_local_ip = local_ip().unwrap();
    let socket = UdpSocket::bind(format!("{}:{}", my_local_ip, PORT))?;
    let map_display = match &parsed_flags.map {
        flags::MapIdentifier::Id(id) => id.to_string(),
        flags::MapIdentifier::Name(name) => name.clone(),
    };
    println!(
        "Server started at {}:{} using map {}",
        my_local_ip, PORT, map_display
    );

    let mut game_state = GameState::new(Some(parsed_flags.map));
    let mut clients = HashMap::<SocketAddr, (u64, String, Instant)>::new();
    let mut client_inputs = HashMap::<u64, fps::Input>::new();
    let mut next_id: u64 = 0;

    // Create and shuffle numbers for assigning random sprites to players
    let mut rng = rng();
    let mut sprite_nums: Vec<u8> = (0..10).collect();
    sprite_nums.shuffle(&mut rng);

    let mut buf = [0; 1024];

    let tick_rate = 100; // ticks per second
    let tick_duration = Duration::from_secs(1) / tick_rate;
    let mut last_tick = Instant::now();

    socket.set_nonblocking(true)?;

    loop {
        // Handle incoming messages
        loop {
            match socket.recv_from(&mut buf) {
                Ok((amt, src)) => {
                    let client_message: ClientMessage = bincode::deserialize(&buf[..amt]).unwrap();

                    if let Some((_, _, last_seen)) = clients.get_mut(&src) {
                        *last_seen = Instant::now();
                    }

                    match client_message {
                        ClientMessage::Connect(username) => {
                            if !clients.contains_key(&src) {
                                if clients.values().any(|(_, name, _)| {
                                    name.to_lowercase() == username.to_lowercase()
                                }) {
                                    println!(
                                        "Rejected connection from {} — username '{}' is already in use.",
                                        src, username
                                    );

                                    let rejection = ServerMessage::UsernameRejected(
                                        "Username already in use".to_string(),
                                    );
                                    let encoded_rejection = bincode::serialize(&rejection).unwrap();
                                    socket.send_to(&encoded_rejection, src)?;
                                } else if username.is_empty() {
                                    println!(
                                        "Rejected connection from {} — username '{}' is empty.",
                                        src, username
                                    );

                                    let rejection = ServerMessage::UsernameRejected(
                                        "Empty username".to_string(),
                                    );
                                    let encoded_rejection = bincode::serialize(&rejection).unwrap();
                                    socket.send_to(&encoded_rejection, src)?;
                                } else {
                                    println!(
                                        "New client connected: {} (username: {})",
                                        src, username
                                    );
                                    clients
                                        .insert(src, (next_id, username.clone(), Instant::now()));

                                    let welcome = Welcome { id: next_id };
                                    let encoded_welcome =
                                        bincode::serialize(&ServerMessage::Welcome(welcome))
                                            .unwrap();
                                    socket.send_to(&encoded_welcome, src)?;

                                    game_state.players.insert(
                                        next_id.to_string(),
                                        Player::new(
                                            sprite_nums[(next_id % 10) as usize].to_string(),
                                        ),
                                    );
                                    client_inputs.insert(next_id, fps::Input::default()); // Initialize with default input
                                    next_id += 1;

                                    let initial_state =
                                        ServerMessage::InitialState(game_state.clone());
                                    let encoded_initial_state =
                                        bincode::serialize(&initial_state).unwrap();
                                    socket.send_to(&encoded_initial_state, src)?;
                                }
                            }
                        }
                        ClientMessage::Input(input) => {
                            if let Some((id, _, _)) = clients.get(&src) {
                                client_inputs.insert(*id, input);
                            }
                        }
                        ClientMessage::Ping => {
                            // Ping received, client is alive
                        }
                        ClientMessage::Shot { angle, pitch } => {
                            if let Some((shooter_id, _, _)) = clients.get(&src) {
                                if let Some(shooter) =
                                    game_state.players.get(&shooter_id.to_string())
                                {
                                    let shot_dir_x = angle.cos();
                                    let shot_dir_y = angle.sin();

                                    for (target_id_str, target) in &game_state.players {
                                        if &shooter_id.to_string() != target_id_str {
                                            let dx = target.x - shooter.x;
                                            let dy = target.y - shooter.y;
                                            let dist_sq = dx * dx + dy * dy;

                                            if dist_sq < 100.0 {
                                                // Max shot distance

                                                // Calculate the dot product of the vector from shooter to target and the shot direction.
                                                // A positive dot product means the target is generally in front of the shooter.
                                                let dot = dx * shot_dir_x + dy * shot_dir_y;
                                                if dot > 0.0 {
                                                    // Calculate the squared length of the projection of the shooter-to-target vector onto the shot direction vector.
                                                    // This helps determine how far along the shot's path the target is.
                                                    let proj_len_sq = dot * dot
                                                        / (shot_dir_x * shot_dir_x
                                                            + shot_dir_y * shot_dir_y);

                                                    // Squared perpendicular distance from the target to the shot ray: how far off-axis the target is from the shot's line of fire.
                                                    let perp_dist_sq = dist_sq - proj_len_sq;

                                                    let target_width =
                                                        SPRITE_OTHER_PLAYER_WIDTH * 0.5; // Player hitbox width
                                                    if perp_dist_sq < target_width * target_width {
                                                        // Vertical check
                                                        let dist = dist_sq.sqrt();
                                                        let shot_height_at_target = shooter.z
                                                            + CAMERA_HEIGHT_OFFSET
                                                            + pitch * dist * 0.5; // pitch is a vertical offset, not an angle 

                                                        if shot_height_at_target > target.z - 0.5
                                                            && shot_height_at_target
                                                                < target.z
                                                                    + SPRITE_OTHER_PLAYER_HEIGHT
                                                                    - 0.5
                                                        {
                                                            // Shot hit someone
                                                            let target_id = target_id_str
                                                                .parse::<u64>()
                                                                .unwrap();
                                                            let shooter_name = clients
                                                                .get(&src)
                                                                .unwrap()
                                                                .1
                                                                .clone();
                                                            let target_name = clients
                                                                .values()
                                                                .find(|(id, _, _)| *id == target_id)
                                                                .unwrap()
                                                                .1
                                                                .clone();

                                                            println!(
                                                                "{} shot {}",
                                                                shooter_name, target_name
                                                            );

                                                            let hit = fps::Hit {
                                                                shooter_id: *shooter_id,
                                                                shooter_name,
                                                                target_id,
                                                                target_name,
                                                            };
                                                            let shot_hit_message =
                                                                ServerMessage::ShotHit(hit);
                                                            let encoded_message =
                                                                bincode::serialize(
                                                                    &shot_hit_message,
                                                                )
                                                                .unwrap();
                                                            for client_addr in clients.keys() {
                                                                socket
                                                                    .send_to(
                                                                        &encoded_message,
                                                                        client_addr,
                                                                    )
                                                                    .unwrap();
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break; // No more messages to read
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::ConnectionReset => {
                    // On Windows, we get "connection reset" errors on UDP sockets
                    // when a client sends an ICMP port unreachable message.
                    // We can safely ignore these and have a clean terminal.
                    // Later client will be safely timed out.
                    continue;
                }
                Err(e) => {
                    eprintln!("Couldn't receive a datagram: {}", e);
                    // Consider what to do with this error, e.g., continue or break
                    break;
                }
            }
        }

        // Remove timed out clients
        let now = Instant::now();
        let timeout = Duration::from_secs(5);
        let mut timed_out_clients = Vec::new();
        clients.retain(|_, (id, username, last_seen)| {
            if now.duration_since(*last_seen) > timeout {
                println!("Client {} ({}) timed out.", id, username);
                timed_out_clients.push(*id);
                false
            } else {
                true
            }
        });

        for id in timed_out_clients {
            game_state.players.remove(&id.to_string());
            client_inputs.remove(&id);
            let player_left_message = ServerMessage::PlayerLeft(id);
            let encoded_message = bincode::serialize(&player_left_message).unwrap();
            for client_addr in clients.keys() {
                socket.send_to(&encoded_message, client_addr).unwrap();
            }
        }

        // Game logic update and broadcast
        let now = Instant::now();
        if now - last_tick >= tick_duration {
            last_tick = now;

            // Apply inputs and update game state
            for (id, input) in &client_inputs {
                game_state.update(id.to_string(), input, tick_duration);
            }

            // Adjust players' z if jumped
            for player in game_state.players.values_mut() {
                player.z += player.velocity_z;
                if player.z > 0.0 {
                    player.velocity_z -= 0.0012;
                } else {
                    player.velocity_z = 0.0;
                    player.z = 0.0;
                }
            }

            // Prepare and send game update to all clients
            let mut player_updates = HashMap::<String, PlayerUpdate>::new();
            for (id, player) in &game_state.players {
                player_updates.insert(
                    id.clone(),
                    PlayerUpdate {
                        x: player.x,
                        y: player.y,
                        z: player.z,
                        angle: player.angle,
                        pitch: player.pitch,
                        texture: player.texture.clone(),
                        animation_state: player.animation_state.clone(),
                        shooting: player.shooting,
                    },
                );
            }

            let encoded_game_update =
                bincode::serialize(&ServerMessage::GameUpdate(player_updates)).unwrap();
            for client_addr in clients.keys() {
                socket.send_to(&encoded_game_update, client_addr)?;
            }
        }

        // Sleep for a short duration to prevent busy-waiting, but allow for immediate processing if a message arrives
        let time_to_next_tick = tick_duration
            .checked_sub(now - last_tick)
            .unwrap_or_default();
        if time_to_next_tick > Duration::ZERO {
            std::thread::sleep(time_to_next_tick);
        }
    }
}
