use crate::consts::{
    DEFAULT_PLAYER_MOVE_SPEED, DEFAULT_PLAYER_ROT_SPEED, PLAYER_JUMP_VELOCITY, PLAYER_PITCH_LIMIT,
    PLAYER_RADIUS, PLAYER_SPRINT_SPEED_MULTIPLIER,
};

use crate::AnimationState;
use crate::Direction;
use crate::Input;
use crate::World;

use serde::{Deserialize, Serialize};

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
    pub fn new(texturename: String) -> Self {
        Player {
            x: 1.5,
            y: 1.5,
            z: 0.0,
            angle: std::f32::consts::PI / 2.0,
            pitch: 0.0,
            velocity_z: 0.0,
            move_speed: DEFAULT_PLAYER_MOVE_SPEED,
            rot_speed: DEFAULT_PLAYER_ROT_SPEED,
            texture: texturename,
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
        self.pitch = (self.pitch + input.pitch * self.rot_speed * 2.0)
            .clamp(-PLAYER_PITCH_LIMIT, PLAYER_PITCH_LIMIT);
    }

    // Verbose but fast function that avoids heap allocation, vector creation and branching
    fn check_collision_and_move(&mut self, new_x: f32, new_y: f32, world: &World) {
        let dx = new_x - self.x;
        let dy = new_y - self.y;

        let mut clear_x = true;
        let mut clear_y = true;

        // --- Horizontal movement ---
        if dx < 0.0 {
            // Moving left: check left-side corners
            let cx = new_x - PLAYER_RADIUS;
            let top_y = self.y + PLAYER_RADIUS;
            let bottom_y = self.y - PLAYER_RADIUS;

            if world.get_tile(cx.floor() as usize, top_y.floor() as usize) != 0
                || world.get_tile(cx.floor() as usize, bottom_y.floor() as usize) != 0
            {
                clear_x = false;
            }
        } else if dx > 0.0 {
            // Moving right: check right-side corners
            let cx = new_x + PLAYER_RADIUS;
            let top_y = self.y + PLAYER_RADIUS;
            let bottom_y = self.y - PLAYER_RADIUS;

            if world.get_tile(cx.floor() as usize, top_y.floor() as usize) != 0
                || world.get_tile(cx.floor() as usize, bottom_y.floor() as usize) != 0
            {
                clear_x = false;
            }
        }

        // --- Vertical movement ---
        if dy < 0.0 {
            // Moving down: check bottom corners
            let cy = new_y - PLAYER_RADIUS;
            let left_x = self.x - PLAYER_RADIUS;
            let right_x = self.x + PLAYER_RADIUS;

            if world.get_tile(left_x.floor() as usize, cy.floor() as usize) != 0
                || world.get_tile(right_x.floor() as usize, cy.floor() as usize) != 0
            {
                clear_y = false;
            }
        } else if dy > 0.0 {
            // Moving up: check top corners
            let cy = new_y + PLAYER_RADIUS;
            let left_x = self.x - PLAYER_RADIUS;
            let right_x = self.x + PLAYER_RADIUS;

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
