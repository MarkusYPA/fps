use crate::{map::World};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap};
use crate::gamestate::GameState;

pub mod consts;
pub mod flags;
pub mod gamestate;
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
    Shot,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage {
    Welcome(Welcome),
    GameUpdate(HashMap<String, PlayerUpdate>),
    SpriteUpdate(HashMap<u32, Sprite>),
    InitialState(GameState),
    UsernameRejected(String),
    PlayerLeft(u64),
    ShotHit(Hit),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hit {
    pub shooter_id: u64,
    pub shooter_name: String,
    pub target_id: u64,
    pub target_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Welcome {
    pub id: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AnimationState {
    Idle,
    Walking,
    Shooting,
    Dying,
    Dead,
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
    pub shooting: bool,
    pub health: u16,
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
    pub shoot: bool,
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
