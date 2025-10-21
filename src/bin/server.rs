use fps::{ClientMessage, GameState, Player, PlayerUpdate, ServerMessage, Welcome, PORT};
use local_ip_address::local_ip;
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

fn main() -> std::io::Result<()> {
    let my_local_ip = local_ip().unwrap();
    let socket = UdpSocket::bind(format!("{}:{}", my_local_ip, PORT))?;
    println!("Server started at {}:{}", my_local_ip, PORT);

    let mut game_state = GameState::new();
    let mut clients = HashMap::<SocketAddr, u64>::new();
    let mut next_id: u64 = 0;

    let mut buf = [0; 1024];

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
                            next_id += 1;
                        }
                    }
                    ClientMessage::Input(input) => {
                        if let Some(id) = clients.get(&src) {
                            game_state.update(id.to_string(), &input);
                        }
                    }
                }

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
            Err(e) => {
                eprintln!("Couldn't receive a datagram: {}", e);
            }
        }
    }
}
