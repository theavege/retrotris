use crate::*;


/// Fixed timestep for the physics simulation (seconds per tick).
pub const TICK: f32 = 1.0 / 60.0;

/// Playfield width/height in logical units; positions wrap around both edges,
/// which is what makes this an "asteroids" field rather than a bounded arena.
pub const FIELD_W: f32 = 1000.0;
pub const FIELD_H: f32 = 1000.0;

/// Circle-ish body with position, velocity and radius, used for the ship,
/// asteroids and bullets alike.
#[derive(Clone, Copy, Default)]
pub struct Body {
    pub pos: (f32, f32),
    pub vel: (f32, f32),
    pub rad: f32,
}

impl Body {
    /// Shortest wrapped displacement between two coordinates on a torus.
    fn wrap_delta(a: f32, b: f32, size: f32) -> f32 {
        let mut d = a - b;
        if d > size / 2.0 {
            d -= size;
        } else if d < -size / 2.0 {
            d += size;
        }
        d
    }
    /// Toroidal distance check: bodies near opposite edges still collide.
    pub fn touches(&self, other: &Body) -> bool {
        let dx = Self::wrap_delta(self.pos.0, other.pos.0, FIELD_W);
        let dy = Self::wrap_delta(self.pos.1, other.pos.1, FIELD_H);
        let r = self.rad + other.rad;
        dx * dx + dy * dy <= r * r
    }
    /// Keep a coordinate inside [0, size) after wrapping.
    pub fn wrap(pos: &mut (f32, f32)) {
        pos.0 = pos.0.rem_euclid(FIELD_W);
        pos.1 = pos.1.rem_euclid(FIELD_H);
    }
    pub fn fly(&mut self, dt: f32) {
        self.pos.0 += self.vel.0 * dt;
        self.pos.1 += self.vel.1 * dt;
        Self::wrap(&mut self.pos);
    }
}

/// The player ship: rotates in place, thrusts along its nose with drag, and
/// carries a short-lived respawn invulnerability window.
pub struct Ship {
    pub body: Body,
    pub angle: f32,
    pub shield: f32,
}

impl Default for Ship {
    fn default() -> Self {
        Self::new()
    }
}

impl Ship {
    const THRUST: f32 = 300.0;
    const DRAG: f32 = 0.995;
    const TURN: f32 = 3.6; // radians per second
    pub const SHIELD_TIME: f32 = 3.0;

    pub fn new() -> Self {
        Self {
            body: Body {
                pos: (FIELD_W / 2.0, FIELD_H / 2.0),
                vel: (0.0, 0.0),
                rad: 14.0,
            },
            angle: -std::f32::consts::FRAC_PI_2, // facing up
            shield: Self::SHIELD_TIME,
        }
    }
    pub fn protect(&mut self) {
        self.shield = Self::SHIELD_TIME;
    }
    pub fn vulnerable(&self) -> bool {
        self.shield <= 0.0
    }
    pub fn turn(&mut self, dir: f32, dt: f32) {
        self.angle += dir * Self::TURN * dt;
    }
    /// Apply engine thrust along the nose direction.
    pub fn thrust(&mut self, dt: f32) {
        self.body.vel.0 += self.angle.cos() * Self::THRUST * dt;
        self.body.vel.1 += self.angle.sin() * Self::THRUST * dt;
    }
    /// Muzzle position: a point just ahead of the nose.
    pub fn muzzle(&self) -> (f32, f32) {
        (
            self.body.pos.0 + self.angle.cos() * self.body.rad,
            self.body.pos.1 + self.angle.sin() * self.body.rad,
        )
    }
    pub fn fly(&mut self, dt: f32) {
        self.body.vel.0 *= Self::DRAG;
        self.body.vel.1 *= Self::DRAG;
        self.body.fly(dt);
        self.shield -= dt;
    }
}

/// A drifting space rock. `scale` is 1 for big rocks, 2 for medium, 3 for
/// small — splitting reduces the scale until it shatters completely.
#[derive(Clone, Copy)]
pub struct Rock {
    pub body: Body,
    pub scale: usize,
    pub spin: f32,
    pub heading: f32,
}

impl Rock {
    const RADII: [f32; 3] = [42.0, 24.0, 12.0];
    const SPEEDS: [f32; 3] = [70.0, 95.0, 120.0];

    /// Spawn one rock at a random position far from the ship center.
    pub fn drift(rng: &mut impl RngExt, scale: usize) -> Self {
        let speed = Self::SPEEDS[scale - 1];
        let heading = rng.random_range(0.0..std::f32::consts::TAU);
        // Sample the border band so rocks never materialize on top of the ship.
        let pos = if rng.random_bool(0.5) {
            (rng.random_range(0.0..FIELD_W), if rng.random_bool(0.5) { 0.0 } else { FIELD_H - 1.0 })
        } else {
            (if rng.random_bool(0.5) { 0.0 } else { FIELD_W - 1.0 }, rng.random_range(0.0..FIELD_H))
        };
        Self {
            body: Body {
                pos,
                vel: (heading.cos() * speed, heading.sin() * speed),
                rad: Self::RADII[scale - 1],
            },
            scale,
            spin: rng.random_range(-1.5..1.5),
            heading,
        }
    }
    pub fn fly(&mut self, dt: f32) {
        self.body.fly(dt);
        self.heading += self.spin * dt;
    }
}

/// Bullets: short-lived fast movers fired from the ship's nose.
#[derive(Default)]
pub struct Shots {
    lives: Vec<f32>,
    bodies: Vec<Body>,
}

impl Shots {
    pub const SPEED: f32 = 520.0;
    pub const LIFETIME: f32 = 1.1;

    pub fn fire(&mut self, origin: (f32, f32), angle: f32) {
        self.bodies.push(Body {
            pos: origin,
            vel: (angle.cos() * Self::SPEED, angle.sin() * Self::SPEED),
            rad: 3.0,
        });
        self.lives.push(Self::LIFETIME);
    }
    /// Advance bullets; expired ones are culled.
    pub fn fly(&mut self, dt: f32) {
        for (b, life) in self.bodies.iter_mut().zip(self.lives.iter_mut()) {
            b.fly(dt);
            *life -= dt;
        }
        let mut keep = 0usize;
        for i in 0..self.lives.len() {
            if self.lives[i] > 0.0 {
                self.lives.swap(keep, i);
                self.bodies.swap(keep, i);
                keep += 1;
            }
        }
        self.lives.truncate(keep);
        self.bodies.truncate(keep);
    }
    pub fn bodies(&self) -> &[Body] {
        &self.bodies
    }
    pub fn remove(&mut self, index: usize) {
        self.bodies.remove(index);
        self.lives.remove(index);
    }
    pub fn clear(&mut self) {
        self.bodies.clear();
        self.lives.clear();
    }
}

/// The asteroid belt: keeps every live rock and knows how to split them.
#[derive(Default)]
pub struct Belt {
    rocks: Vec<Rock>,
}

impl Belt {
    pub fn seed(&mut self, count: usize, rng: &mut impl RngExt) {
        self.rocks.clear();
        for _ in 0..count {
            self.rocks.push(Rock::drift(rng, 1));
        }
    }
    pub fn rocks(&self) -> &[Rock] {
        &self.rocks
    }
    pub fn is_empty(&self) -> bool {
        self.rocks.is_empty()
    }
    pub fn fly(&mut self, dt: f32) {
        for rock in &mut self.rocks {
            rock.fly(dt);
        }
    }
    /// Destroy the rock nearest to `target` if it actually overlaps it, and
    /// spawn its two smaller splinters in place. Returns the destroyed scale.
    pub fn blast(&mut self, target: &Body, rng: &mut impl RngExt) -> Option<usize> {
        let idx = self
            .rocks
            .iter()
            .enumerate()
            .filter(|(_, r)| r.body.touches(target))
            .map(|(i, _)| i)
            .next()?;
        let rock = self.rocks.remove(idx);
        if rock.scale < 3 {
            for _ in 0..2 {
                let mut child = Rock::drift(rng, rock.scale + 1);
                child.body.pos = rock.body.pos;
                self.rocks.push(child);
            }
        }
        Some(rock.scale)
    }
    /// True when any rock overlaps the given body (used for ship collisions).
    pub fn hits(&self, ship: &Body) -> bool {
        self.rocks.iter().any(|r| r.body.touches(ship))
    }
    /// Shoot every bullet at the rocks; each shattered rock spawns splinters.
    /// Returns the scales of all destroyed rocks (for scoring).
    pub fn shot(&mut self, shots: &mut Shots, rng: &mut impl RngExt) -> Vec<usize> {
        let mut hit = Vec::new();
        loop {
            let target = shots
                .bodies()
                .iter()
                .enumerate()
                .find_map(|(i, b)| self.rocks.iter().find(|r| b.touches(&r.body)).map(|r| (i, r.body)));
            match target {
                Some((shot, rock)) => match self.blast(&rock, rng) {
                    Some(scale) => {
                        shots.remove(shot);
                        hit.push(scale);
                    }
                    None => break,
                },
                None => break,
            }
        }
        hit
    }
}

/// Game score: points per shattered rock (smaller = better), lives and wave
/// counter; difficulty tier grows with the wave number.
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
    /// Award points by rock scale: 20 / 50 / 100 for big / medium / small.
    pub fn rock(&mut self, scale: usize) {
        self.points += match scale {
            1 => 20,
            2 => 50,
            _ => 100,
        };
        if self.points > self.best {
            self.best = self.points;
        }
    }
    pub fn lose_life(&mut self) {
        self.lives -= 1;
    }
    pub fn wave_cleared(&mut self) {
        self.points += 150;
        self.wave += 1;
        if self.points > self.best {
            self.best = self.points;
        }
    }
    /// Rocks to seed for the given wave (wave 0 starts with four).
    pub fn seeds(&self) -> usize {
        4 + self.wave as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bullet_shatters_big_rock_into_splinters() {
        let mut rng = rand::rng();
        let mut belt = Belt::default();
        // Place a big rock manually so the test does not depend on randomness.
        belt.rocks.push(Rock {
            body: Body { pos: (500.0, 500.0), vel: (0.0, 0.0), rad: Rock::RADII[0] },
            scale: 1,
            spin: 0.0,
            heading: 0.0,
        });
        let mut shots = Shots::default();
        shots.fire((500.0, 500.0), 0.0);
        let hit = belt.shot(&mut shots, &mut rng);
        assert_eq!(hit, vec![1]);
        assert_eq!(belt.rocks().len(), 2); // two splinters in place of the big rock
        assert!(shots.bodies().is_empty());
    }

    #[test]
    fn positions_wrap_toroidally() {
        let mut body = Body {
            pos: (FIELD_W - 0.5, 1.0),
            vel: (2.0, 0.0),
            rad: 1.0,
        };
        body.fly(1.0);
        assert!(body.pos.0 < 1.0);
    }
}
