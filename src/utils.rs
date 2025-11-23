// Utility functions / functions I'm not sure where to put

use crate::ServerMessage;
use crate::gamestate::GameState;
use crate::map::World;
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use rand::seq::SliceRandom;
use rand::Rng;
use crate::consts::{DEFAULT_RANDOM_MAP_PATH_DEVIATION_CHANCE, DEFAULT_RANDOM_MAP_HOLE_CHANCE};

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

/// Returns true if all adjacent tiles are walls, also checks corners if include_corners is true
pub fn check_adjacent_tiles(world: &World, tile: (usize, usize), ignore_tile: (usize, usize), include_corners: bool) -> bool {
    for dx in -1..=1 {
        for dy in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            // Skip corners if not including them
            if !include_corners && dx != 0 && dy != 0 {
                continue;
            }
            let nx = tile.0 as i32 + dx;
            let ny = tile.1 as i32 + dy;
            // For ignoring previously cut out tiles
            if nx == ignore_tile.0 as i32 && ny == ignore_tile.1 as i32 {
                continue;
            }
            if nx >= 0 && ny >= 0 {
                let nx = nx as usize;
                let ny = ny as usize;
                if ny < world.map.len() && nx < world.map[ny].len() {
                    if world.get_tile(ny, nx) == 0 {
                        return false;
                    }
                }
            }
        }
    }
    true
}

pub fn carve_path(world: &mut World, tile: (usize, usize), include_corners: bool, prev_direction: Option<(i32, i32)>) {
    world.map[tile.1][tile.0] = 0;
    let mut directions = vec![(0, 1), (0, -1), (1, 0), (-1, 0)];
    let mut rng = rand::rng();
    
    // Prioritize previous direction if available, with a small chance to deviate
    if let Some(prev_dir) = prev_direction {
        // chance to deviate from previous direction
        if rng.random_range(0..100) < DEFAULT_RANDOM_MAP_PATH_DEVIATION_CHANCE {
            directions.shuffle(&mut rng);
        } else {
            directions.retain(|&d| d != prev_dir);
            directions.insert(0, prev_dir);
            // Shuffle remaining directions
            if directions.len() > 1 {
                let first = directions.remove(0);
                directions.shuffle(&mut rng);
                directions.insert(0, first);
            }
        }
    } else {
        directions.shuffle(&mut rng);
    }

    for (dx, dy) in directions {
        let nx = tile.0 as i32 + dx;
        let ny = tile.1 as i32 + dy;
        // 1 instead of 0 to not carve out the edges of the map
        if nx < 1 || ny < 1 {
            continue;
        }
        let nx = nx as usize;
        let ny = ny as usize;
        // -1 instead of len() to not carve out the edges of the map
        if ny < world.map.len()-1 && nx < world.map[ny].len()-1 {
            if world.get_tile(ny, nx) == 0 {
                continue;
            }
            if check_adjacent_tiles(world, (nx, ny), tile, include_corners) {
                carve_path(world, (nx, ny), include_corners, Some((dx, dy)));
            } else if rng.random_range(0..100) < DEFAULT_RANDOM_MAP_HOLE_CHANCE {
                carve_path(world, (nx, ny), include_corners, Some((dx, dy)));
            }
        }
    }
}
