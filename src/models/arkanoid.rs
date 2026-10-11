use crate::*;

/// Fixed timestep for the physics simulation (seconds per tick).
pub const TICK: f32 = 1.0 / 60.0;

/// A horizontal paddle that slides left/right along the bottom of the field.
#[derive(Default)]
pub struct Paddle {
    x: f32,
    y: f32,
    size: (f32, f32),
}

impl Paddle {
    pub fn new(field: (i32, i32)) -> Self {
        let size = (WIDTH as f32 * 2.0, HEIGHT as f32 / 2.0);
        Self {
            x: (field.0 as f32 - size.0) / 2.0,
            y: field.1 as f32 - size.1 - PAD as f32,
            size,
        }
    }
    /// Center the paddle at the given horizontal coordinate (mouse follow),
    /// clamped inside the field.
    pub fn center_on(&mut self, x: f32, limit: i32) {
        let max = limit as f32 - self.size.0;
        self.x = (x - self.size.0 / 2.0).clamp(0.0, max.max(0.0));
    }
    /// Move the paddle by `step` pixels, clamped inside the field.
    pub fn slide(&mut self, step: f32, limit: i32) {
        self.center_on(self.x + self.size.0 / 2.0 + step, limit);
    }
    pub fn draw(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.size.0, self.size.1)
    }
}

/// The brick wall: a grid of cells, each holding an optional health value.
/// Cells are addressed by column/row; geometry is derived from the field so
/// the layout adapts on resize.
#[derive(Default)]
pub struct Wall {
    cols: usize,
    rows: usize,
    cells: Vec<u8>,
}

impl Wall {
    pub const COLS: usize = 10;
    pub const ROWS: usize = 6;
    /// Vertical offset of the wall's top edge (below the score HUD).
    pub const TOP: i32 = PAD * 5;

    pub fn new() -> Self {
        Self {
            cols: Self::COLS,
            rows: Self::ROWS,
            // Top rows are tougher: they need two hits before breaking.
            cells: (0..Wall::COLS * Wall::ROWS)
                .map(|i| if i < Wall::COLS * 2 { 2 } else { 1 })
                .collect(),
        }
    }
    pub fn cell(&self, col: usize, row: usize) -> u8 {
        self.cells[row * self.cols + col]
    }
    pub fn damage(&mut self, col: usize, row: usize) {
        let c = &mut self.cells[row * self.cols + col];
        *c = c.saturating_sub(1);
    }
    pub fn alive(&self) -> bool {
        self.cells.iter().any(|&c| c > 0)
    }
    /// Geometry of one cell in pixel coordinates.
    pub fn rect(&self, col: usize, row: usize, field: (i32, i32)) -> (f32, f32, f32, f32) {
        let gap = PAD as f32 / 2.0;
        let w = (field.0 as f32 - gap * (self.cols as f32 + 1.0)) / self.cols as f32;
        let h = HEIGHT as f32;
        let x = gap + col as f32 * (w + gap);
        let y = Self::TOP as f32 + row as f32 * (h + gap);
        (x, y, w, h)
    }
    /// Find the cell containing the point (px, py), if any still stands.
    pub fn hit_at(&self, px: f32, py: f32, field: (i32, i32)) -> Option<(usize, usize)> {
        for row in 0..self.rows {
            for col in 0..self.cols {
                if self.cell(col, row) == 0 {
                    continue;
                }
                let (x, y, w, h) = self.rect(col, row, field);
                if px >= x && px < x + w && py >= y && py < y + h {
                    return Some((col, row));
                }
            }
        }
        None
    }
}

/// The ball moves with a constant-magnitude velocity vector. Bouncing off the
/// paddle deflects it depending on where it was hit; bouncing off a brick can
/// flip its vertical direction.
#[derive(Default)]
pub struct Ball {
    pos: (f32, f32),
    vel: (f32, f32),
}

impl Ball {
    pub const SIZE: f32 = HEIGHT as f32 * 0.7;

    /// Place the ball resting on top of the paddle, ready to be launched.
    pub fn attach(paddle: &Paddle) -> Self {
        let (x, y, w, _) = paddle.draw();
        Self {
            pos: (x + w / 2.0 - Ball::SIZE / 2.0, y - Ball::SIZE),
            vel: (0.0, 0.0),
        }
    }
    /// Launch the ball straight up-ish with the given speed.
    pub fn launch(&mut self, speed: f32) {
        let angle = 0.4_f32; // slight diagonal so serves vary a little
        self.vel = (speed * angle.sin(), -speed * angle.cos());
    }
    pub fn moving(&self) -> bool {
        self.vel != (0.0, 0.0)
    }
    /// Vertical component of the velocity, used by the view to pick the
    /// ball's leading edge for brick collision checks.
    pub fn vel_y(&self) -> f32 {
        self.vel.1
    }
    /// Advance the ball by one fixed tick, bouncing off the side/top walls.
    /// Returns `Some(true)` when the ball escaped through the bottom.
    pub fn step(&mut self, field: (i32, i32)) -> Option<bool> {
        self.pos.0 += self.vel.0;
        self.pos.1 += self.vel.1;
        let size = Self::SIZE;
        if self.pos.0 <= 0.0 {
            self.pos.0 = 0.0;
            self.vel.0 = self.vel.0.abs();
        } else if self.pos.0 + size >= field.0 as f32 {
            self.pos.0 = field.0 as f32 - size;
            self.vel.0 = -self.vel.0.abs();
        }
        if self.pos.1 <= Wall::TOP as f32 - PAD as f32 {
            self.pos.1 = Wall::TOP as f32 - PAD as f32;
            self.vel.1 = self.vel.1.abs();
        }
        if self.pos.1 >= field.1 as f32 {
            return Some(true);
        }
        None
    }
    /// Bounce the ball off the paddle. Returns true if a bounce happened.
    /// The reflection angle depends on where the ball hits the paddle:
    /// hitting the center keeps it steep, hitting the edges makes it flat.
    pub fn check_paddle(&mut self, paddle: &Paddle, speed: f32) -> bool {
        if self.vel.1 <= 0.0 {
            return false;
        }
        let (px, py, pw, ph) = paddle.draw();
        let half = Self::SIZE / 2.0;
        let cx = self.pos.0 + half;
        let cy = self.pos.1 + half;
        if cx < px || cx > px + pw || cy < py || cy > py + ph {
            return false;
        }
        // Relative hit position in [-1, 1], mapped to a max 60-degree deflection.
        let rel = ((cx - (px + pw / 2.0)) / (pw / 2.0)).clamp(-1.0, 1.0);
        let angle = rel * std::f32::consts::PI / 3.0;
        self.vel = (speed * angle.sin(), -speed * angle.cos());
        // Push the ball out of the paddle to avoid double bounces.
        self.pos.1 = py - Self::SIZE;
        true
    }
    /// Reflect the ball vertically when it hits a brick cell.
    pub fn bounce_brick(&mut self) {
        self.vel.1 = -self.vel.1;
    }
    pub fn draw(&self) -> (f32, f32, f32) {
        (self.pos.0, self.pos.1, Self::SIZE)
    }
}

/// Game score: points, lives and cleared levels, persisted between sessions.
/// Difficulty level is derived from the current score (exclusive ranges).
#[derive(Default)]
pub struct Score {
    pub points: i32,
    pub lives: i32,
    pub best: i32,
}

impl Score {
    pub const START_LIVES: i32 = 3;

    pub fn reset(&mut self) {
        self.points = 0;
        self.lives = Self::START_LIVES;
    }
    /// Break a brick: tougher bricks (health 2) are worth more than fresh ones.
    pub fn brick(&mut self, health: u8) {
        self.points += match health {
            2 => 20,
            _ => 10,
        };
        if self.points > self.best {
            self.best = self.points;
        }
    }
    pub fn lose_life(&mut self) {
        self.lives -= 1;
    }
    /// Difficulty level derived from the player's score (exclusive ranges).
    pub fn level(&self) -> usize {
        match self.points {
            0..=99 => 0,
            100..=299 => 1,
            300..=599 => 2,
            _ => 3,
        }
    }
}
