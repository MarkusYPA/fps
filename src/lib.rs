use serde::{Deserialize, Serialize};

pub mod renderer;
pub mod minimap;
pub mod textures;

pub const WIDTH: usize = 1024;
pub const HEIGHT: usize = 768;
pub const PORT: u16 = 8080;

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessage {
    Connect(String),
    Input(Input),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage {
    Welcome(Welcome),
    GameUpdate(std::collections::HashMap<String, PlayerUpdate>),
    InitialState(GameState),
    UsernameRejected(String),
}


#[derive(Serialize, Deserialize, Debug)]
pub struct Welcome {
    pub id: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerUpdate {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub angle: f32,
    pub pitch: f32,
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Input {
    pub forth: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub turn: f32,
    pub pitch: f32,
    pub jump: bool,
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
            move_speed: 0.05,
            rot_speed: 0.03,
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

        self.check_collision_and_move(new_x, new_y, world);

        if input.jump && self.z == 0.0 {
            self.velocity_z = 0.028;
        }

        self.angle += input.turn * self.rot_speed;
        self.pitch = (self.pitch + input.pitch * self.rot_speed * 2.0).clamp(
            -std::f32::consts::PI / 2.5,  // restrict pitch angle
            std::f32::consts::PI / 2.5,
        );
    }

    fn check_collision_and_move(&mut self, new_x: f32, new_y: f32, world: &World) {
        if world.get_tile(new_x as usize, self.y as usize) == 0 {
            self.x = new_x;
        }
        if world.get_tile(self.x as usize, new_y as usize) == 0 {
            self.y = new_y;
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct World {
    pub map: Vec<Vec<u8>>,
}

impl World {
    pub fn new() -> Self {
        World {
            map: vec![
    vec![1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],
    vec![1,0,0,0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,0,0,1],
    vec![1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1],
    vec![1,0,0,1,0,1,0,1,0,1,1,1,0,1,1,1,0,1,0,1,0,1,0,0,1],
    vec![1,1,0,0,0,1,0,0,0,1,0,1,0,0,0,1,0,0,0,1,0,0,0,1,1],
    vec![1,0,0,1,0,1,0,1,0,1,0,1,0,1,1,1,0,1,0,1,0,1,0,0,1],
    vec![1,1,0,0,0,1,0,0,0,1,0,1,0,1,0,0,0,0,0,1,0,0,0,1,1],
    vec![1,0,0,1,0,1,1,1,0,1,0,1,0,1,1,1,0,1,1,1,1,1,0,0,1],
    vec![1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1],
    vec![1,0,0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,0,1],
    vec![1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1],
    vec![1,0,0,1,1,1,1,1,0,1,0,1,0,1,0,1,0,1,0,1,1,1,0,0,1],
    vec![1,1,0,1,0,1,0,1,0,1,0,0,0,1,0,1,0,0,0,0,0,1,0,1,1],
    vec![1,0,0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,1,1,1,0,0,1],
    vec![1,1,0,1,0,1,0,1,0,1,0,0,0,1,0,1,0,0,0,1,0,0,0,1,1],
    vec![1,0,0,1,0,1,0,1,0,1,1,1,1,1,0,1,1,1,0,1,1,1,0,0,1],
    vec![1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1],
    vec![1,0,0,1,0,1,0,1,0,1,0,1,0,1,0,1,1,1,0,1,1,1,0,0,1],
    vec![1,1,0,1,0,1,0,1,0,1,0,0,0,1,0,1,0,0,0,1,0,0,0,1,1],
    vec![1,0,0,1,0,1,0,1,0,1,0,1,0,1,0,1,1,1,0,1,1,1,0,0,1],
    vec![1,1,0,1,0,1,0,1,0,1,0,0,0,1,0,0,0,1,0,1,0,0,0,1,1],
    vec![1,0,0,1,1,1,1,1,0,1,1,1,1,1,0,1,1,1,0,1,1,1,0,0,1],
    vec![1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1],
    vec![1,1,1,0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,1,1,1],
    vec![1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],
],

        }
    }

    pub fn get_tile(&self, x: usize, y: usize) -> u8 {
        if x < self.map.len() && y < self.map[0].len() {
            self.map[y][x]
        } else {
            1
        }
    }
}

use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameState {
    pub players: HashMap<String, Player>,
    pub world: World,
    pub sprites: Vec<Sprite>,
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            players: HashMap::new(),
            world: World::new(),
            sprites: vec![
                Sprite { x: 3.2, y: 4.3, z: 0.0, texture: "character2".to_string(), width: 0.2, height: 0.65 },
                Sprite { x: 4.2, y: 4.3, z: 0.0, texture: "character3".to_string(), width: 0.2, height: 0.65 },
            ]
        }
    }

    pub fn update(&mut self, id: String, input: &Input) {
        if let Some(player) = self.players.get_mut(&id) {
            player.take_input(input, &self.world);
        }
    }
}
