use std::net::UdpSocket;
use fps::{GameState, Input, PORT};
use local_ip_address::local_ip;

fn main() -> std::io::Result<()> {
    let my_local_ip = local_ip().unwrap();
    let socket = UdpSocket::bind(format!("{}:{}", my_local_ip, PORT))?;
    println!("Server started at {}:{}", my_local_ip, PORT);

    let mut game_state = GameState::new();
    let mut clients = std::collections::HashSet::new();

    let mut buf = [0; 1024];

    loop {
        match socket.recv_from(&mut buf) {
            Ok((amt, src)) => {
                if !clients.contains(&src) {
                    clients.insert(src);
                    println!("New client connected: {}", src);
                }

                let input: Input = bincode::deserialize(&buf[..amt]).unwrap();
                game_state.update(&input);

                let encoded_game_state = bincode::serialize(&game_state).unwrap();
                for client in &clients {
                    socket.send_to(&encoded_game_state, client)?;
                }
            }
            Err(e) => {
                eprintln!("Couldn't receive a datagram: {}", e);
            }
        }
    }
}
