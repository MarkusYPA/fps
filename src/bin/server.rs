use fps::{ClientMessage, GameState, Player, PlayerUpdate, ServerMessage, Welcome, PORT};
use local_ip_address::local_ip;
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

fn main() -> std::io::Result<()> {
    let my_local_ip = local_ip().unwrap();
    let socket = UdpSocket::bind(format!("{}:{}", my_local_ip, PORT))?;
    println!("Server started at {}:{}", my_local_ip, PORT);

    let mut game_state = GameState::new();
    let mut clients = HashMap::<SocketAddr, u64>::new();
    let mut client_inputs = HashMap::<u64, fps::Input>::new();
    let mut next_id: u64 = 0;

    let mut buf = [0; 1024];

    let tick_rate = 100; // ticks per second
    let tick_duration = Duration::from_secs(1) / tick_rate;
    //let tick_duration = Duration::from_millis(50) / tick_rate;
    let mut last_tick = Instant::now();

    socket.set_nonblocking(true)?;

    loop {
        // Handle incoming messages
        loop {
            match socket.recv_from(&mut buf) {
                Ok((amt, src)) => {
                    let client_message: ClientMessage = bincode::deserialize(&buf[..amt]).unwrap();

                    match client_message {
                        ClientMessage::Connect => {
                            if !clients.contains_key(&src) {
                                println!("New client connected: {}", src);
                                clients.insert(src, next_id);

                                let welcome = Welcome { id: next_id };
                                let encoded_welcome = bincode::serialize(&ServerMessage::Welcome(welcome)).unwrap();
                                socket.send_to(&encoded_welcome, src)?;

                                game_state
                                    .players
                                    .insert(next_id.to_string(), Player::new());
                                client_inputs.insert(next_id, fps::Input::default()); // Initialize with default input
                                next_id += 1;
                            }
                        }
                        ClientMessage::Input(input) => {
                            if let Some(id) = clients.get(&src) {
                                client_inputs.insert(*id, input);
                            }
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break; // No more messages to read
                }
                Err(e) => {
                    eprintln!("Couldn't receive a datagram: {}", e);
                    // Consider what to do with this error, e.g., continue or break
                    break;
                }
            }
        }

        // Game logic update and broadcast
        let now = Instant::now();
        if now - last_tick >= tick_duration {
            last_tick = now;

            // Apply inputs and update game state
            for (id, input) in &client_inputs {
                game_state.update(id.to_string(), input);
            }

            // Prepare and send game update to all clients
            let mut player_updates = HashMap::new();
            for (id, player) in &game_state.players {
                player_updates.insert(id.clone(), PlayerUpdate {
                    x: player.x,
                    y: player.y,
                    angle: player.angle,
                });
            }

            let encoded_game_update = bincode::serialize(&ServerMessage::GameUpdate(player_updates)).unwrap();
            for client_addr in clients.keys() {
                socket.send_to(&encoded_game_update, client_addr)?;
            }
        }

        // Sleep for a short duration to prevent busy-waiting, but allow for immediate processing if a message arrives
        let time_to_next_tick = tick_duration.checked_sub(now - last_tick).unwrap_or_default();
        if time_to_next_tick > Duration::ZERO {
            std::thread::sleep(time_to_next_tick);
        }
    }
}
