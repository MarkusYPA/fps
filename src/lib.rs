use crate::map::World;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::consts::{
    DEFAULT_PLAYER_MOVE_SPEED,
    DEFAULT_PLAYER_ROT_SPEED,
    PLAYER_JUMP_VELOCITY,
    PLAYER_PITCH_LIMIT,
    PLAYER_SPRINT_SPEED_MULTIPLIER,
};

pub mod consts;
pub mod flags;
pub mod map;
pub mod minimap;
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
pub struct Player {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub angle: f32,
    pub pitch: f32,
    pub velocity_z: f32,
    pub move_speed: f32,
    pub rot_speed: f32,
    pub texture: String,
    pub animation_state: AnimationState,
    pub direction: Direction,
    pub frame: usize,
    pub frame_timer: f32,
}

impl Player {
    pub fn new() -> Self {
        Player {
            x: 1.5,
            y: 1.5,
            z: 0.0,
            angle: std::f32::consts::PI / 2.0,
            pitch: 0.0,
            velocity_z: 0.0,
            move_speed: DEFAULT_PLAYER_MOVE_SPEED,
            rot_speed: DEFAULT_PLAYER_ROT_SPEED,
            texture: "character4".to_string(),
            animation_state: AnimationState::Idle,
            direction: Direction::Front,
            frame: 0,
            frame_timer: 0.0,
        }
    }

    pub fn take_input(&mut self, input: &Input, world: &World) {
        let mut new_x = self.x;
        let mut new_y = self.y;

        let mut slower = 1.0;
        if (input.left || input.right) && (input.forth || input.back) {
            slower = 0.707;
        }

        if input.forth {
            new_x += self.angle.cos() * self.move_speed * slower;
            new_y += self.angle.sin() * self.move_speed * slower;
        }

        if input.back {
            new_x -= self.angle.cos() * self.move_speed * slower;
            new_y -= self.angle.sin() * self.move_speed * slower;
        }

        let strafe_x = -self.angle.sin();
        let strafe_y = self.angle.cos();

        if input.right {
            new_x += strafe_x * self.move_speed * slower;
            new_y += strafe_y * self.move_speed * slower;
        }

        if input.left {
            new_x -= strafe_x * self.move_speed * slower;
            new_y -= strafe_y * self.move_speed * slower;
        }

        if input.sprint {
            new_x += self.angle.cos() * self.move_speed * PLAYER_SPRINT_SPEED_MULTIPLIER * slower;
            new_y += self.angle.sin() * self.move_speed * PLAYER_SPRINT_SPEED_MULTIPLIER * slower;
        }

        self.check_collision_and_move(new_x, new_y, world);

        if input.jump && self.z == 0.0 {
            self.velocity_z = PLAYER_JUMP_VELOCITY;
        }

        self.angle += input.turn * self.rot_speed;
        self.pitch = (self.pitch + input.pitch * self.rot_speed * 2.0).clamp(
            -PLAYER_PITCH_LIMIT,
            PLAYER_PITCH_LIMIT,
        );
    }

    // Verbose but fast function that avoids heap allocation, vector creation and branching
    fn check_collision_and_move(&mut self, new_x: f32, new_y: f32, world: &World) {
        let radius = 0.2;
        let dx = new_x - self.x;
        let dy = new_y - self.y;

        let mut clear_x = true;
        let mut clear_y = true;

        // --- Horizontal movement ---
        if dx < 0.0 {
            // Moving left: check left-side corners
            let cx = new_x - radius;
            let top_y = self.y + radius;
            let bottom_y = self.y - radius;

            if world.get_tile(cx.floor() as usize, top_y.floor() as usize) != 0
                || world.get_tile(cx.floor() as usize, bottom_y.floor() as usize) != 0
            {
                clear_x = false;
            }
        } else if dx > 0.0 {
            // Moving right: check right-side corners
            let cx = new_x + radius;
            let top_y = self.y + radius;
            let bottom_y = self.y - radius;

            if world.get_tile(cx.floor() as usize, top_y.floor() as usize) != 0
                || world.get_tile(cx.floor() as usize, bottom_y.floor() as usize) != 0
            {
                clear_x = false;
            }
        }

        // --- Vertical movement ---
        if dy < 0.0 {
            // Moving down: check bottom corners
            let cy = new_y - radius;
            let left_x = self.x - radius;
            let right_x = self.x + radius;

            if world.get_tile(left_x.floor() as usize, cy.floor() as usize) != 0
                || world.get_tile(right_x.floor() as usize, cy.floor() as usize) != 0
            {
                clear_y = false;
            }
        } else if dy > 0.0 {
            // Moving up: check top corners
            let cy = new_y + radius;
            let left_x = self.x - radius;
            let right_x = self.x + radius;

            if world.get_tile(left_x.floor() as usize, cy.floor() as usize) != 0
                || world.get_tile(right_x.floor() as usize, cy.floor() as usize) != 0
            {
                clear_y = false;
            }
        }

        // --- Apply movement ---
        if clear_x {
            self.x += dx;
        }
        if clear_y {
            self.y += dy;
        }
    }
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
}

impl GameState {
    pub fn new(map_identifier: Option<crate::flags::MapIdentifier>) -> Self {
        let world = match map_identifier {
            Some(crate::flags::MapIdentifier::Id(id)) => World::new(Some(id), None),
            Some(crate::flags::MapIdentifier::Name(name)) => World::new(Some(0), Some(&name)),
            None => World::new(Some(1), None),
        };
        GameState {
            players: HashMap::new(),
            world,
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
