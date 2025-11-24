// Utility functions / functions I'm not sure where to put

use crate::ServerMessage;
use crate::gamestate::GameState;
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

pub fn set_winner(
    game_state: &mut GameState,
    winner_name: String,
    socket: &UdpSocket,
    clients: &HashMap<SocketAddr, (u64, String, std::time::Instant)>,
) {
    game_state.winner = Some(winner_name.clone());
    broadcast_message(
        ServerMessage::Winner(winner_name.clone()),
        socket,
        Some(clients),
        None,
    )
    .unwrap();

    println!("Game over! Winner is {winner_name}");
}

/// Updates the leaderboard with a new score and broadcasts the update to all clients. Returns the new score.
pub fn update_leaderboard(
    game_state: &mut GameState,
    shooter_name: String,
    socket: &UdpSocket,
    clients: &HashMap<SocketAddr, (u64, String, std::time::Instant)>,
    set_score: Option<usize>, // Set the score to a specific value
    up_score: Option<usize>,  // Increase the score by a specific value
    reset_score_all: bool,    // Reset the score of all players
) -> usize {
    let new_score = if let Some(score) = set_score {
        // Set the score to a specific value
        score
    } else if let Some(increment) = up_score {
        // Increase the score by a specific value
        game_state
            .leaderboard
            .get(&shooter_name)
            .copied()
            .unwrap_or(0)
            + increment
    } else if reset_score_all {
        // Reset the score of all players
        game_state.leaderboard.iter_mut().for_each(|(_, score)| {
            *score = 0;
        });
        return 0;
    } else {
        // No operation specified, return early
        return game_state
            .leaderboard
            .get(&shooter_name)
            .copied()
            .unwrap_or(0);
    };

    game_state
        .leaderboard
        .insert(shooter_name.clone(), new_score);

    let leaderboard = game_state.leaderboard.clone();

    broadcast_message(
        ServerMessage::LeaderboardUpdate(leaderboard),
        socket,
        Some(clients),
        None,
    )
    .unwrap();

    new_score
}

/// Broadcasts a message to all clients or a specific client.
pub fn broadcast_message(
    message: ServerMessage,
    socket: &UdpSocket,
    clients: Option<&HashMap<SocketAddr, (u64, String, std::time::Instant)>>,
    client: Option<SocketAddr>,
) -> std::io::Result<()> {
    let encoded_message = bincode::serialize(&message).unwrap();
    match (clients, client) {
        (Some(clients), None) => {
            for client_addr in clients.keys() {
                socket.send_to(&encoded_message, client_addr)?;
            }
        }
        (None, Some(client)) => {
            socket.send_to(&encoded_message, client)?;
        }
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Either both clients and client, or neither clients and client provided",
            ));
        }
    }
    Ok(())
}
