use crate::*;

/// Fixed timestep for the physics simulation (seconds per tick).
pub const TICK: f32 = 1.0 / 60.0;

/// Marching speed of the alien horde, in cells per second, per level tier.
const MARCH_SPEEDS: [f32; 4] = [1.2, 1.8, 2.6, 3.6];
/// Alien fire rate (shots per second) and player bullet cooldown, per tier.
const ALIEN_RATE: [f32; 4] = [0.35, 0.55, 0.8, 1.1];
const SHOT_COOLDOWN: [f32; 4] = [0.5, 0.4, 0.3, 0.25];

/// Player ship: slides horizontally along the bottom row and shoots upward,
/// with a short cooldown between shots (scales with difficulty).
#[derive(Default)]
pub struct Ship {
    pub col: usize,
    cooldown: f32,
}

impl Ship {
    pub fn new() -> Self {
        Self {
            col: COLS as usize / 2,
            cooldown: 0.0,
        }
    }
    pub fn slide(&mut self, step: i32, cols: usize) {
        if step < 0 {
            self.col = self.col.saturating_sub(1);
        } else if self.col + 1 < cols {
            self.col += 1;
        }
    }
    pub fn center(&self) -> f32 {
        self.col as f32 + 0.5
    }
    /// Try to fire; returns true when the cooldown allows it.
    pub fn shoot(&mut self, level: usize) -> bool {
        if self.cooldown > 0.0 {
            return false;
        }
        self.cooldown = SHOT_COOLDOWN[level];
        true
    }
    pub fn reload(&mut self, dt: f32) {
        self.cooldown -= dt;
    }
}

/// The alien horde marches sideways one cell at a time; every wall hit drops
/// it by one row and reverses direction. Rows are addressed from the top.
#[derive(Default)]
pub struct Horde {
    rows: Vec<Vec<bool>>,
    dir: i32,
    offset: f32,
    drop: bool,
}

impl Horde {
    pub const ROWS: usize = 4;

    pub fn new() -> Self {
        Self {
            rows: vec![vec![true; COLS as usize - 2]; Self::ROWS],
            dir: 1,
            offset: 0.0,
            drop: false,
        }
    }
    /// Advance the march by `step` cells; returns true when a full column
    /// step was taken (a good moment for the view to play an animation beat).
    pub fn march(&mut self, step: f32) -> bool {
        self.offset += step;
        if self.dir > 0 && self.right_edge() >= COLS - 1 || self.dir < 0 && self.left_edge() <= 1 {
            self.dir = -self.dir;
            self.drop = true;
        }
        if self.offset >= 1.0 {
            self.offset -= 1.0;
            for row in &mut self.rows {
                // Shift every live cell one column in the march direction.
                let mut next = vec![false; row.len()];
                for (i, &alive) in row.iter().enumerate() {
                    if alive {
                        next[(i as i32 + self.dir) as usize] = true;
                    }
                }
                *row = next;
            }
            if self.drop {
                self.drop = false;
                // Drop the whole formation by one row: top row becomes empty,
                // each other row takes over the row above it.
                self.rows.pop();
                self.rows.insert(0, vec![false; COLS as usize - 2]);
            }
            return true;
        }
        false
    }
    fn left_edge(&self) -> i32 {
        self.extent(i32::MAX, true)
    }
    fn right_edge(&self) -> i32 {
        self.extent(i32::MIN, false)
    }
    fn extent(&self, init: i32, min: bool) -> i32 {
        let mut edge = init;
        for row in &self.rows {
            for (i, &alive) in row.iter().enumerate() {
                if alive {
                    let i = i as i32;
                    edge = if min { edge.min(i) } else { edge.max(i) };
                }
            }
        }
        if edge == init {
            if min { COLS } else { -1 }
        } else {
            edge
        }
    }
    pub fn alive(&self, col: usize, row: usize) -> bool {
        self.rows[row][col]
    }
    pub fn kill(&mut self, col: usize, row: usize) {
        self.rows[row][col] = false;
    }
    pub fn any(&self) -> bool {
        self.rows.iter().any(|row| row.iter().any(|&a| a))
    }
    /// Bottom-most living row index, or `None` when the field is clear.
    pub fn deepest_row(&self) -> Option<usize> {
        (0..self.rows.len())
            .rev()
            .find(|&r| self.rows[r].iter().any(|&a| a))
    }
    /// A random living column, used to pick which alien fires this tick.
    pub fn shooter(&self, rng: &mut impl RngExt) -> Option<usize> {
        let cols: Vec<usize> = (0..COLS as usize)
            .filter(|&c| self.rows.iter().any(|row| row[c]))
            .collect();
        if cols.is_empty() {
            return None;
        }
        Some(cols[rng.random_range(0..cols.len())])
    }
    /// The bottom-most living alien in the given column (the one that fires).
    pub fn deepest_in(&self, col: usize) -> Option<usize> {
        (0..self.rows.len()).rev().find(|&r| self.rows[r][col])
    }
}

/// Projectiles travel in half-cell steps: player bullets up, alien bombs down.
#[derive(Default)]
pub struct Shots {
    bullets: Vec<(f32, f32)>,
    bombs: Vec<(f32, f32)>,
}

impl Shots {
    const BULLET_SPEED: f32 = 14.0;
    const BOMB_SPEED: [f32; 4] = [5.0, 6.0, 7.0, 8.0];

    pub fn fire_bullet(&mut self, x: f32, y: f32) {
        self.bullets.push((x, y));
    }
    pub fn drop_bomb(&mut self, x: f32, y: f32) {
        self.bombs.push((x, y));
    }
    /// Advance everything; off-screen projectiles are culled.
    pub fn fly(&mut self, dt: f32, level: usize, rows: usize) {
        let vy_b = Self::BULLET_SPEED * dt;
        let vy_a = Self::BOMB_SPEED[level] * dt;
        self.bullets.retain_mut(|p| {
            p.1 -= vy_b;
            p.1 > -1.0 && p.0 >= 0.0 && p.0 < COLS as f32
        });
        self.bombs.retain_mut(|p| {
            p.1 += vy_a;
            p.1 < rows as f32 + 2.0 && p.0 >= 0.0 && p.0 < COLS as f32
        });
    }
    /// Remove the bullet overlapping the given cell, if any. Returns true on hit.
    pub fn take_bullet_at(&mut self, col: usize, row: usize) -> bool {
        let idx = self.bullets.iter().position(|(x, y)| {
            *x >= col as f32 && *x < col as f32 + 1.0 && *y >= row as f32 && *y < row as f32 + 1.0
        });
        match idx {
            Some(i) => {
                self.bullets.remove(i);
                true
            }
            None => false,
        }
    }
    /// Remove the bomb overlapping the given cell, if any. Returns true on hit.
    pub fn take_bomb_at(&mut self, col: usize, row: usize) -> bool {
        let idx = self.bombs.iter().position(|(x, y)| {
            *x >= col as f32 && *x < col as f32 + 1.0 && *y >= row as f32 && *y < row as f32 + 1.0
        });
        match idx {
            Some(i) => {
                self.bombs.remove(i);
                true
            }
            None => false,
        }
    }
    pub fn bullets(&self) -> &[(f32, f32)] {
        &self.bullets
    }
    pub fn bombs(&self) -> &[(f32, f32)] {
        &self.bombs
    }
    pub fn clear(&mut self) {
        self.bullets.clear();
        self.bombs.clear();
    }
}

/// Game score: points, lives and wave counter, persisted between sessions.
/// Difficulty tier is derived from the current wave (exclusive ranges).
#[derive(Default)]
pub struct Score {
    pub points: i32,
    pub lives: i32,
    pub best: i32,
    pub wave: i32,
}

impl Score {
    pub const START_LIVES: i32 = 3;

    pub fn reset(&mut self) {
        self.points = 0;
        self.lives = Self::START_LIVES;
        self.wave = 0;
    }
    pub fn alien(&mut self) {
        self.points += 10;
        if self.points > self.best {
            self.best = self.points;
        }
    }
    pub fn wave_cleared(&mut self) {
        self.points += 100;
        self.wave += 1;
        if self.points > self.best {
            self.best = self.points;
        }
    }
    pub fn lose_life(&mut self) {
        self.lives -= 1;
    }
    /// Difficulty tier derived from the wave number (exclusive ranges).
    pub fn level(&self) -> usize {
        match self.wave {
            0 => 0,
            1..=2 => 1,
            3..=5 => 2,
            _ => 3,
        }
    }
    pub fn march_speed(&self) -> f32 {
        MARCH_SPEEDS[self.level()]
    }
    pub fn alien_rate(&self) -> f32 {
        ALIEN_RATE[self.level()]
    }
}
