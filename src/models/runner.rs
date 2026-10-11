use rand::RngExt;

/// Chrome padding of the play field (also used by the view).
pub const PADDING: f32 = 34.0;
/// Distance from the bottom window edge up to the ground line.
pub const GROUND_MARGIN: f32 = 64.0;
/// Fixed gravity applied to the dino's vertical velocity (px/s^2).
pub const GRAVITY: f32 = 2400.0;
/// Initial upward velocity when a jump starts (px/s).
pub const JUMP_VELOCITY: f32 = -900.0;
/// Base horizontal scroll speed before difficulty scaling (px/s).
pub const BASE_RUN_SPEED: f32 = 360.0;
/// Per-second acceleration factor of the run speed.
pub const SPEED_ACCEL: f32 = 0.06;
/// Upper bound for the per-frame speed-up multiplier.
pub const SPEED_ACCEL_MAX: f32 = 1.02;
/// Score points gained per second while playing.
pub const SCORE_PER_SECOND: f32 = 120.0;
/// Period of one ground texture tile (px).
pub const GROUND_TILE: f32 = 32.0;
/// Minimal score before pterodactyls may start spawning.
pub const PTERO_MIN_SCORE: u32 = 300;

#[derive(Default)]
pub enum Scene {
    #[default]
    Welcome = 0,
    Playing,
    Paused,
    GameOver,
}

#[derive(Default)]
pub enum Difficulty {
    #[default]
    Normal = 0,
    Hard,
    Easy,
}

impl Difficulty {
    pub fn limit(&self) -> (f32, f32) {
        const CACTUS_MIN_GAP: f32 = 320.0;
        const CACTUS_MAX_GAP: f32 = 520.0;
        (
            CACTUS_MIN_GAP * self.gap_scale(),
            CACTUS_MAX_GAP * self.gap_scale(),
        )
    }
    pub fn scaled_gap(&self, first: bool) -> f32 {
        let (min, max) = self.limit();
        match first {
            true => (min + max) * 0.5,
            false => rand::rng().random_range(min..max),
        }
    }
    pub fn switch(&self) -> Self {
        match self {
            Self::Easy => Self::Normal,
            Self::Normal => Self::Hard,
            Self::Hard => Self::Easy,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Normal => "Normal",
            Self::Hard => "Hard",
        }
    }
    pub fn speed_mul(&self) -> f32 {
        match self {
            Self::Easy => 0.9,
            Self::Normal => 1.0,
            Self::Hard => 1.12,
        }
    }
    pub fn gap_scale(&self) -> f32 {
        match self {
            Self::Easy => 1.15,
            Self::Normal => 1.0,
            Self::Hard => 0.85,
        }
    }
    pub fn ptero_enabled(&self) -> bool {
        matches!(self, Self::Normal | Self::Hard)
    }
}

pub enum ObstacleKind {
    Cactus(bool),      // small (true) / large variant
    Pterodactyl(bool), // flies at high (true) / low altitude
}

pub struct Obstacle {
    pub rect: (f32, f32, f32, f32),
    pub kind: ObstacleKind,
}

impl Obstacle {
    /// Builds an obstacle of the given kind entering the field at its right
    /// edge, standing on (or hovering above) the ground line.
    pub fn spawn(kind: ObstacleKind, ground_y: f32, field_w: f32) -> Self {
        let (y, w, h) = match kind {
            ObstacleKind::Cactus(tall) => match tall {
                true => (ground_y - 46.0, 24.0, 46.0),
                false => (ground_y - 60.0, 36.0, 60.0),
            },
            ObstacleKind::Pterodactyl(high) => match high {
                true => (ground_y - 48.0 - 22.0 - 24.0, 46.0, 24.0),
                false => (ground_y - 30.0 - 10.0 - 24.0, 46.0, 24.0),
            },
        };
        Self {
            rect: (field_w - PADDING - w, y, w, h),
            kind,
        }
    }
    /// Random gap distance after this obstacle, difficulty-scaled.
    pub fn next_gap(&self, difficulty: &Difficulty) -> f32 {
        const PTERO_MIN_GAP: f32 = 520.0;
        const PTERO_MAX_GAP: f32 = 760.0;
        let (min, max) = match self.kind {
            ObstacleKind::Pterodactyl(..) => (
                PTERO_MIN_GAP * difficulty.gap_scale(),
                PTERO_MAX_GAP * difficulty.gap_scale(),
            ),
            ObstacleKind::Cactus(..) => difficulty.limit(),
        };
        let range = min..max;
        rand::rng().random_range(range)
    }
}

#[derive(Default)]
pub struct Dino {
    pub ducking: bool,
    pub jumping: bool,
    pub leg_state: bool,
    pub leg_timer: f32,
    pub y: f32,
    pub vy: f32,
    /// One-slot buffer: a jump pressed slightly too early is remembered and
    /// consumed on landing instead of being dropped.
    buffered: bool,
}

impl Dino {
    pub const FOOT_SIZE: (f32, f32) = (10.0, 4.0);
    pub const LEG_STEP: f32 = 6.0;
    pub const LEG_PERIOD: f32 = 0.12;
    pub fn x(&self) -> f32 {
        120.0
    }
    pub fn w(&self) -> f32 {
        44.0
    }
    pub fn height(&self) -> f32 {
        if self.ducking { 30.0 } else { 48.0 }
    }
    pub fn reset(&mut self, ground_top: f32) {
        *self = Self {
            y: ground_top,
            ..Default::default()
        };
    }
    pub fn jump(&mut self, ground_top: f32) {
        if self.jumping {
            self.buffered = true;
        } else if self.y <= ground_top {
            self.start_jump();
        } else {
            self.jumping = true;
            self.vy = JUMP_VELOCITY;
        }
    }
    fn start_jump(&mut self) {
        self.buffered = false;
        self.jumping = true;
        self.ducking = false;
        self.vy = JUMP_VELOCITY;
    }
    /// Advance the physics by `dt` seconds. Returns true on the frame the
    /// dino lands back on the ground.
    pub fn update(&mut self, dt: f32, ground_top: f32) -> bool {
        if self.jumping {
            self.vy += GRAVITY * dt;
            self.y += self.vy * dt;
            if self.y >= ground_top {
                self.y = ground_top;
                self.vy = 0.0;
                self.jumping = false;
                if self.buffered {
                    self.start_jump();
                }
                return true;
            }
        } else {
            self.y = ground_top;
            self.vy = 0.0;
            self.leg_timer += dt;
            if self.leg_timer >= Self::LEG_PERIOD {
                self.leg_timer = 0.0;
                self.leg_state = !self.leg_state;
            }
        }
        false
    }
    /// Collision box inset from the sprite so near-misses feel fair.
    pub fn hitbox(&self) -> (f32, f32, f32, f32) {
        let (w, h) = (self.w(), self.height());
        (self.x() + 3.0, self.y + 3.0, w - 9.0, h - 6.0)
    }
}

/// Current/best score with difficulty-scaled accumulation.
#[derive(Default)]
pub struct Score {
    pub current: u32,
    pub best: u32,
    mul: f32,
}

impl Score {
    pub fn set_difficulty(&mut self, difficulty: &Difficulty) {
        self.mul = difficulty.speed_mul();
    }
    pub fn inc(&mut self, dt: f32) {
        self.current += (SCORE_PER_SECOND * self.mul * dt) as u32;
        if self.current > self.best {
            self.best = self.current;
        }
    }
    pub fn reset(&mut self) {
        self.current = 0;
    }
}

/// Axis-aligned rectangle intersection test (model-side, unit-testable).
pub fn rects_intersect(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 < b.0 + b.2 && a.0 + a.2 > b.0 && a.1 < b.1 + b.3 && a.1 + a.3 > b.1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffered_jump_fires_on_landing() {
        let mut dino = Dino::default();
        dino.reset(100.0);
        assert_eq!(dino.y, 100.0);
        dino.jump(100.0);
        assert!(dino.jumping);
        // Press again mid-air: gets buffered, not executed.
        dino.jump(100.0);
        assert!(dino.jumping);
        // Simulate until landing.
        let mut landed = false;
        for _ in 0..1000 {
            landed = dino.update(1.0 / 60.0, 100.0);
            if landed {
                break;
            }
        }
        assert!(landed, "dino should land within 1000 ticks");
        // The buffered press immediately starts a new jump.
        assert!(dino.jumping);
    }

    #[test]
    fn score_accumulates_and_tracks_best() {
        let mut score = Score::default();
        score.set_difficulty(&Difficulty::Normal);
        for _ in 0..600 {
            score.inc(1.0 / 60.0);
        }
        assert!(score.current >= 1000);
        assert_eq!(score.best, score.current);
        score.reset();
        assert_eq!(score.current, 0);
        assert_eq!(score.best, 1200);
    }

    #[test]
    fn hitbox_is_inset_from_sprite() {
        let mut dino = Dino::default();
        dino.reset(0.0);
        let (x, y, w, h) = dino.hitbox();
        assert!(x > dino.x());
        assert!(y > dino.y);
        assert!(w < dino.w());
        assert!(h < dino.height());
    }

    #[test]
    fn rects_intersect_basics() {
        let a = (0.0, 0.0, 10.0, 10.0);
        assert!(rects_intersect(a, (9.0, 9.0, 10.0, 10.0)));
        assert!(!rects_intersect(a, (10.0, 0.0, 10.0, 10.0)));
    }
}
