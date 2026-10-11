use crate::*;

/// Fixed timestep for the physics simulation (seconds per tick).
pub const TICK: f32 = 1.0 / 60.0;

#[derive(Clone, Copy, PartialEq)]
pub enum Side {
    Left,
    Right,
}

/// A vertical paddle that can be moved up and down within the field.
#[derive(Default)]
pub struct Paddle {
    x: i32,
    y: i32,
    size: (i32, i32),
}

impl Paddle {
    pub fn new(side: Side, field: (i32, i32)) -> Self {
        let size = (WIDTH * 2 / 3, HEIGHT * 4);
        let x = match side {
            Side::Left => PAD,
            Side::Right => field.0 - PAD - size.0,
        };
        Self {
            x,
            y: (field.1 - size.1) / 2,
            size,
        }
    }
    pub fn set_y(&mut self, value: i32, limit: i32) {
        self.y = value.clamp(0, limit - self.size.1);
    }
    /// Move the paddle by `step` pixels, clamped inside the field.
    pub fn slide(&mut self, step: i32, limit: i32) {
        self.set_y(self.y + step, limit);
    }
    /// Center the paddle at the given vertical coordinate (mouse follow).
    pub fn center_on(&mut self, y: i32, limit: i32) {
        self.set_y(y - self.size.1 / 2, limit);
    }
    pub fn draw(&self) -> (i32, i32, i32, i32) {
        (self.x, self.y, self.size.0, self.size.1)
    }
}

/// The ball moves with a constant-magnitude velocity vector; bouncing off a
/// paddle deflects it depending on where it was hit.
#[derive(Default)]
pub struct Ball {
    pos: (f32, f32),
    vel: (f32, f32),
}

impl Ball {
    pub const SIZE: i32 = HEIGHT;
    pub fn new(field: (i32, i32), speed: f32) -> Self {
        let angle = 0.5_f32; // slight initial diagonal
        Self {
            pos: (field.0 as f32 / 2.0, field.1 as f32 / 2.0),
            vel: (speed * angle.cos(), speed * angle.sin()),
        }
    }
    /// Advance the ball by one fixed tick. Returns the side of the field the
    /// ball escaped through, if any.
    pub fn step(&mut self, field: (i32, i32)) -> Option<Side> {
        self.pos.0 += self.vel.0;
        self.pos.1 += self.vel.1;
        // Top / bottom walls.
        if self.pos.1 <= 0.0 {
            self.pos.1 = 0.0;
            self.vel.1 = self.vel.1.abs();
        } else if self.pos.1 + Self::SIZE as f32 >= field.1 as f32 {
            self.pos.1 = field.1 as f32 - Self::SIZE as f32;
            self.vel.1 = -self.vel.1.abs();
        }
        // Escape through the left / right goals.
        if self.pos.0 + Self::SIZE as f32 <= 0.0 {
            return Some(Side::Left);
        }
        if self.pos.0 >= field.0 as f32 {
            return Some(Side::Right);
        }
        None
    }
    /// Bounce the ball off a paddle. Returns true if a bounce happened.
    /// The reflection angle depends on where the ball hits the paddle:
    /// hitting the center keeps it flat, hitting the edges makes it steep.
    pub fn check_paddle(&mut self, paddle: &Paddle, side: Side, speed: f32) -> bool {
        let half = Self::SIZE as f32 / 2.0;
        let cy = self.pos.1 + half;
        let top = paddle.y as f32;
        let bottom = (paddle.y + paddle.size.1) as f32;
        let overlap = match side {
            Side::Left => {
                let px = (paddle.x + paddle.size.0) as f32;
                self.pos.0 <= px && self.pos.0 + Self::SIZE as f32 > px && self.vel.0 < 0.0
            }
            Side::Right => {
                let px = paddle.x as f32;
                self.pos.0 + Self::SIZE as f32 >= px && self.pos.0 < px && self.vel.0 > 0.0
            }
        };
        if !overlap || !(top..=bottom).contains(&cy) {
            return false;
        }
        // Relative hit position in [-1, 1], mapped to a max 60-degree deflection.
        let rel = (cy - (top + bottom) / 2.0) / ((bottom - top) / 2.0);
        let angle = rel * std::f32::consts::PI / 3.0;
        let dir = match side {
            Side::Left => 1.0,
            Side::Right => -1.0,
        };
        self.vel = (dir * speed * angle.cos(), speed * angle.sin());
        // Push the ball out of the paddle to avoid double bounces.
        self.pos.0 = match side {
            Side::Left => (paddle.x + paddle.size.0) as f32,
            Side::Right => paddle.x as f32 - Self::SIZE as f32,
        };
        true
    }
    pub fn draw(&self) -> (i32, i32, i32) {
        (self.pos.0 as i32, self.pos.1 as i32, Self::SIZE)
    }
}

/// Game score: points won by each player plus the best streak, persisted
/// between sessions. Levels shorten the interval between ball speed-ups.
#[derive(Default)]
pub struct Score {
    pub left: i32,
    pub right: i32,
    pub best: i32,
}

impl Score {
    pub fn reset(&mut self) {
        self.left = 0;
        self.right = 0;
    }
    pub fn inc(&mut self, side: Side) {
        match side {
            Side::Left => self.left += 1,
            Side::Right => self.right += 1,
        }
        if self.right > self.best {
            self.best = self.right;
        }
    }
    /// Difficulty level derived from the player's score (exclusive ranges).
    pub fn level(&self) -> usize {
        match self.right {
            0..=2 => 0,
            3..=5 => 1,
            6..=9 => 2,
            _ => 3,
        }
    }
}
