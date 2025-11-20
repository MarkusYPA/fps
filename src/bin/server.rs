use fps::{
    ClientMessage, PlayerUpdate, ServerMessage, Welcome, consts::PORT, flags, gamestate::GameState,
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

                                    let new_player = Player::new(
                                        sprite_nums[(next_id % 10) as usize].to_string(),
                                        &game_state.world,
                                    );
                                    game_state.players.insert(next_id.to_string(), new_player);
                                    game_state.leaderboard.insert(username.clone(), 0);
                                    client_inputs.insert(next_id, fps::Input::default()); // Initialize with default input
                                    next_id += 1;

                                    let initial_state =
                                        ServerMessage::InitialState(game_state.clone());
                                    let encoded_initial_state =
                                        bincode::serialize(&initial_state).unwrap();
                                    socket.send_to(&encoded_initial_state, src)?;

                                    let leaderboard_update = ServerMessage::LeaderboardUpdate(game_state.leaderboard.clone().into_iter().map(|(name, score)| (name, score as usize)).collect());
                                    let encoded_leaderboard_update = bincode::serialize(&leaderboard_update).unwrap();
                                    for client_addr in clients.keys() {
                                        socket.send_to(&encoded_leaderboard_update, client_addr).unwrap();
                                    }
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
                        ClientMessage::Shot => {
                            if let Some((shooter_id, shooter_name, _)) = clients.get(&src) {
                                if let Some(target_id) = game_state.measure_shot(shooter_id) {
                                    // reduce target hp
                                    if let Some(target) =
                                        game_state.players.get_mut(&target_id.to_string())
                                    {
                                        if target.health == 20 {
                                        let new_score = game_state
                                            .leaderboard
                                            .get(&shooter_name.clone())
                                            .unwrap()
                                            + 1;
                                            if new_score >= 20 {
                                                game_state.winner = Some(shooter_name.clone());
                                                let winner_message = ServerMessage::Winner(shooter_name.clone());
                                                let encoded_winner_message = bincode::serialize(&winner_message).unwrap();
                                                for client_addr in clients.keys() {
                                                    socket.send_to(&encoded_winner_message, client_addr).unwrap();
                                                }
                                                println!("Game over! Winner is {}", game_state.winner.unwrap());
                                                return Ok(());
                                            }
                                        game_state
                                            .leaderboard
                                            .insert(shooter_name.clone(), new_score);
                                        let leaderboard_update = ServerMessage::LeaderboardUpdate(game_state.leaderboard.clone().into_iter().map(|(name, score)| (name, score as usize)).collect());
                                        let encoded_leaderboard_update = bincode::serialize(&leaderboard_update).unwrap();
                                        for client_addr in clients.keys() {
                                            socket.send_to(&encoded_leaderboard_update, client_addr).unwrap();
                                        }
                                    }
                                        target.take_damage(20);
                                    }

                                    // Send message about hit to clients
                                    let target_name = clients
                                        .values()
                                        .find(|(id, _, _)| *id == target_id)
                                        .unwrap()
                                        .1
                                        .clone();

                                    let hit = fps::Hit {
                                        shooter_id: *shooter_id,
                                        shooter_name: shooter_name.to_string(),
                                        target_id,
                                        target_name,
                                    };
                                    let shot_hit_message = ServerMessage::ShotHit(hit);
                                    let encoded_message =
                                        bincode::serialize(&shot_hit_message).unwrap();
                                    for client_addr in clients.keys() {
                                        socket.send_to(&encoded_message, client_addr).unwrap();
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
        let clients_clone = clients.clone();
        clients.retain(|_, (id, username, last_seen)| {
            if now.duration_since(*last_seen) > timeout {
                println!("Client {} ({}) timed out.", id, username);
                timed_out_clients.push(*id);

                // Remove player from leaderboard
                game_state.leaderboard.remove(username);
                let leaderboard_update = ServerMessage::LeaderboardUpdate(game_state.leaderboard.clone().into_iter().map(|(name, score)| (name, score as usize)).collect());
                let encoded_leaderboard_update = bincode::serialize(&leaderboard_update).unwrap();
                for client_addr in clients_clone.keys() {
                    socket.send_to(&encoded_leaderboard_update, client_addr).unwrap();
                }

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

            let mut sprites_changed = false;

            // Apply inputs and update game state
            for (id, input) in &client_inputs {
                if game_state.update(id.to_string(), input, tick_duration) {
                    sprites_changed = true
                }
            }

            // remove puddles if they hit timeout
            if game_state.limit_sprites() {
                sprites_changed = true;
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
                        health: player.health,
                        score: player.score,
                    },
                );
            }

            let encoded_game_update =
                bincode::serialize(&ServerMessage::GameUpdate(player_updates)).unwrap();

            for client_addr in clients.keys() {
                socket.send_to(&encoded_game_update, client_addr)?;
            }

            if sprites_changed {
                let encoded_sprite_update = bincode::serialize(&ServerMessage::SpriteUpdate(
                    game_state.floor_sprites.clone(),
                ))
                .unwrap();
                for client_addr in clients.keys() {
                    socket.send_to(&encoded_sprite_update, client_addr)?;
                }
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
