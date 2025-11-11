use crate::map::World;
use crate::player::Player;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::consts::{SPRITE_NPC_HEIGHT, SPRITE_NPC_WIDTH};

pub mod consts;
pub mod flags;
pub mod map;
pub mod minimap;
pub mod player;
pub mod renderer;
pub mod spritesheet;
pub mod textures;

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessage {
    Connect(String),
    Input(Input),
    Ping,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage {
    Welcome(Welcome),
    GameUpdate(HashMap<String, PlayerUpdate>),
    InitialState(GameState),
    UsernameRejected(String),
    PlayerLeft(u64),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Welcome {
    pub id: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AnimationState {
    Idle,
    Walking,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Direction {
    Front,
    FrontRight,
    Right,
    BackRight,
    Back,
    BackLeft,
    Left,
    FrontLeft,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerUpdate {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub angle: f32,
    pub pitch: f32,
    pub texture: String,
    pub animation_state: AnimationState,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Input {
    pub forth: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub turn: f32,
    pub pitch: f32,
    pub jump: bool,
    pub sprint: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sprite {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub texture: String,
    pub width: f32,
    pub height: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameState {
    pub players: HashMap<String, Player>,
    pub world: World,
    pub sprites: Vec<Sprite>,
}

impl GameState {
    pub fn new(map_identifier: Option<crate::flags::MapIdentifier>) -> Self {
        let world = match map_identifier {
            Some(crate::flags::MapIdentifier::Id(id)) => World::new(Some(id), None),
            Some(crate::flags::MapIdentifier::Name(name)) => World::new(Some(0), Some(&name)),
            None => World::new(Some(1), None),
        };
        let sprites = vec![
            Sprite {
                x: 3.2,
                y: 4.3,
                z: 0.0,
                texture: "character2".to_string(),
                width: SPRITE_NPC_WIDTH,
                height: SPRITE_NPC_HEIGHT,
            },
            Sprite {
                x: 4.2,
                y: 4.3,
                z: 0.0,
                texture: "character3".to_string(),
                width: SPRITE_NPC_WIDTH,
                height: SPRITE_NPC_HEIGHT,
            },
        ];
        GameState {
            players: HashMap::new(),
            world,
            sprites,
        }
    }

    pub fn update(&mut self, id: String, input: &Input) {
        if let Some(player) = self.players.get_mut(&id) {
            player.take_input(input, &self.world);
            if input.forth || input.back || input.left || input.right {
                player.animation_state = AnimationState::Walking;
            } else {
                player.animation_state = AnimationState::Idle;
            }
        }
    }
}
