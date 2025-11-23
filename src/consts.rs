use std::time::Duration;

use image::Rgba;

pub const WIDTH: usize = 1024;
pub const HEIGHT: usize = 768;
pub const PORT: u16 = 8080;
pub const FONT_PATH: &str = "assets/VT323-Regular.ttf";
pub const TICK_RATE: u32 = 100;
pub const WIN_SLEEP_TIME: Duration = Duration::from_secs(5);
pub const SCORE_TO_WIN: usize = 1;

pub const MOUSE_SENSITIVITY_MIN: f32 = 0.01;
pub const MOUSE_SENSITIVITY_MAX: f32 = 0.20;

pub const DEFAULT_MAP_ID: usize = 1;
pub const MOUSE_SPEED: f32 = 0.06;
pub const CAMERA_HEIGHT_OFFSET: f32 = 0.1;
pub const CAMERA_HEIGHT_OFFSET_DEAD: f32 = -0.4;
pub const CYAN_TRANSPARENT: Rgba<u8> = Rgba([0, 255, 255, 255]);
pub const WALK_FRAME_TIME: f32 = 0.05;
pub const DIE_FRAME_TIME: f32 = 0.20;
pub const RESPAWN_DELAY: Duration = Duration::from_secs(4);

pub const DEFAULT_PLAYER_MOVE_SPEED: f32 = 0.035;
pub const DEFAULT_PLAYER_ROT_SPEED: f32 = 0.03;
pub const PLAYER_JUMP_VELOCITY: f32 = 0.028;
pub const PLAYER_PITCH_LIMIT: f32 = std::f32::consts::PI / 2.5;
pub const PLAYER_SPRINT_SPEED_MULTIPLIER: f32 = 1.5;
pub const PLAYER_RADIUS: f32 = 0.2;

pub const MINIMAP_WIDTH: usize = 160;
pub const MINIMAP_HEIGHT: usize = 160;
pub const MINIMAP_MARGIN: usize = 10;
pub const MINIMAP_BACKGROUND_COLOR: u32 = 0x0011_1111;
pub const MINIMAP_WALL_COLOR: u32 = 0x0044_4444;
pub const MINIMAP_OPEN_SPACE_COLOR: u32 = 0x00AA_AAAA;
pub const MINIMAP_GRID_COLOR: u32 = 0x0022_2222;
pub const MINIMAP_OTHER_PLAYER_COLOR: u32 = 0x00FF_0000;
pub const MINIMAP_BORDER_COLOR: u32 = 0x00FF_FFFF;
pub const MINIMAP_PLAYER_DOT_RADIUS: usize = 3;
pub const MINIMAP_PLAYER_ICON_SIZE: f32 = 12.0;

pub const CEILING_COLOR: u32 = 0x00AA_CCFF;
pub const FLOOR_COLOR: u32 = 0x0055_5555;
pub const WALL_COLOR_PRIMARY: u32 = 0x008A_7755;
pub const WALL_COLOR_SECONDARY: u32 = 0x0069_5A41;
pub const SPRITE_OTHER_PLAYER_WIDTH: f32 = 0.4;
pub const SPRITE_OTHER_PLAYER_HEIGHT: f32 = 0.7;
pub const SPRITE_NPC_WIDTH: f32 = 0.2;
pub const SPRITE_NPC_HEIGHT: f32 = 0.7;
pub const CAMERA_PLANE_SCALE: f32 = 0.66;

pub const GUN_SCALE: f32 = 1.0;
pub const GUN_X_OFFSET: usize = 190;
pub const CROSSHAIR_SCALE: f32 = 0.5;
pub const SHOT_TIME: Duration = Duration::from_millis(35);
pub const SHOT_MAX_DISTANCE: f32 = 200.0;

pub const MAX_PUDDLES: usize = 100;
