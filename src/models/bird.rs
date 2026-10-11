use rand::RngExt;

/// All Floppy Bird tuning constants live here, in normalized (0..1) world units.
/// `Config::scale` converts them to pixel space for the current field size,
/// so gameplay feels identical at any window resolution.
pub struct Config {
    pub bird_radius: f32,   // of height
    pub bird_x_ratio: f32,  // of width
    pub gravity: f32,       // heights / s^2
    pub flap_velocity: f32, // heights / s
    pub max_fall: f32,      // heights / s
    pub pipe_width: f32,    // of width
    pub pipe_gap: f32,      // of height
    pub pipe_speed: f32,    // widths / s
    pub pipe_interval: f32, // seconds between pipes
    pub gap_shrink: f32,    // gap shrink per point scored (fraction of base gap)
    pub speed_growth: f32,  // speed increase per point scored (fraction of base)
    pub min_gap: f32,       // minimum gap as fraction of height
    pub max_speed_mul: f32, // cap on the difficulty speed multiplier
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bird_radius: 0.06,
            bird_x_ratio: 0.18,
            gravity: 2.4,
            flap_velocity: -0.75,
            max_fall: 1.2,
            pipe_width: 0.12,
            pipe_gap: 0.32,
            pipe_speed: 0.35,
            pipe_interval: 1.5,
            gap_shrink: 0.015,
            speed_growth: 0.012,
            min_gap: 0.22,
            max_speed_mul: 1.8,
        }
    }
}

impl Config {
    /// Converts normalized units into pixel-space sizes for a given field.
    pub fn scale(&self, field: (i32, i32)) -> Scaled {
        let w = field.0.max(1) as f32;
        let h = field.1.max(1) as f32;
        Scaled {
            bird_radius: self.bird_radius * h,
            bird_x: self.bird_x_ratio * w,
            gravity: self.gravity * h,
            flap_velocity: self.flap_velocity * h,
            max_fall: self.max_fall * h,
            pipe_width: self.pipe_width * w,
            pipe_gap: self.pipe_gap * h,
            pipe_speed: self.pipe_speed * w,
            pipe_interval: self.pipe_interval,
        }
    }
}

/// Pixel-space geometry derived from `Config` and the current field size.
#[derive(Clone, Copy)]
pub struct Scaled {
    pub bird_radius: f32,
    pub bird_x: f32,
    pub gravity: f32,
    pub flap_velocity: f32,
    pub max_fall: f32,
    pub pipe_width: f32,
    pub pipe_gap: f32,
    pub pipe_speed: f32,
    pub pipe_interval: f32,
}

#[derive(Default)]
pub struct Bird {
    pub y: f32,
    pub velocity: f32,
}

impl Bird {
    pub fn new(field_h: i32) -> Self {
        Self {
            y: field_h as f32 / 2.0,
            velocity: 0.0,
        }
    }
    /// Frame-rate independent integration: physics is scaled by delta time.
    pub fn update(&mut self, dt: f32, s: &Scaled) {
        self.velocity = (self.velocity + s.gravity * dt).min(s.max_fall);
        self.y += self.velocity * dt;
    }
    pub fn flap(&mut self, s: &Scaled) {
        self.velocity = s.flap_velocity;
    }
    /// Rotation angle (radians) derived from vertical velocity, for drawing.
    pub fn tilt(&self, s: &Scaled) -> f32 {
        (self.velocity / s.max_fall).clamp(-1.0, 1.0) * 0.6
    }
}

pub struct Pipe {
    pub x: f32,
    pub width: f32,
    pub gap_center_y: f32,
    pub gap: f32,
    pub scored: bool,
}

impl Pipe {
    /// Collision-fairness inset: hitboxes are shrunk slightly relative to visuals
    /// so near-misses feel like passes, not deaths.
    const HITBOX_INSET: f32 = 4.0;

    pub fn top_rect(&self) -> (f32, f32, f32, f32) {
        (self.x, 0.0, self.width, self.gap_center_y - self.gap / 2.0)
    }
    pub fn bottom_rect(&self, field_h: f32) -> (f32, f32, f32, f32) {
        let y = self.gap_center_y + self.gap / 2.0;
        (self.x, y, self.width, field_h - y)
    }
    /// Circle (bird center + radius) vs rect collision using the closest-point method.
    pub fn collides(&self, bird_x: f32, bird_y: f32, radius: f32, field_h: f32) -> bool {
        let r = (radius - Self::HITBOX_INSET).max(1.0);
        [self.top_rect(), self.bottom_rect(field_h)]
            .into_iter()
            .any(|(x, y, w, h)| {
                let (x, w) = (x + Self::HITBOX_INSET, w - 2.0 * Self::HITBOX_INSET);
                if w <= 0.0 || h <= 0.0 {
                    return false;
                }
                let cx = bird_x.clamp(x, x + w);
                let cy = bird_y.clamp(y, y + h);
                let dx = bird_x - cx;
                let dy = bird_y - cy;
                dx * dx + dy * dy < r * r
            })
    }
}

/// Difficulty progression: the gap narrows linearly with score,
/// but never below `config.min_gap` (as a fraction of field height).
pub fn gap_for_score(s: &Scaled, config: &Config, field_h: f32, score: i32) -> f32 {
    let shrink = 1.0 - config.gap_shrink * score as f32;
    (s.pipe_gap * shrink).max(config.min_gap * field_h)
}

pub fn speed_multiplier(config: &Config, score: i32) -> f32 {
    (1.0 + config.speed_growth * score as f32).min(config.max_speed_mul)
}

/// Picks a random gap center fully inside the field, biased away from the
/// very edges so every pipe stays passable.
pub fn random_gap_center(field_h: f32, gap: f32) -> f32 {
    let margin = gap / 2.0 + field_h * 0.06;
    if field_h <= margin * 2.0 {
        field_h / 2.0
    } else {
        rand::rng().random_range(margin..(field_h - margin))
    }
}
