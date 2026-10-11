//! Super Plumber - pure game logic (LÖVE-style `update`/model, no FLTK).

/// Tile size in pixels (also used by the view).
pub const TILE: f32 = 24.0;
/// Gravity applied to vertical velocity (px/s^2).
pub const GRAVITY: f32 = 1500.0;
/// Upward velocity when a jump starts (px/s).
pub const JUMP_VELOCITY: f32 = -520.0;
/// Maximum horizontal walk speed (px/s).
pub const WALK_SPEED: f32 = 160.0;
/// Horizontal acceleration (px/s^2) so movement feels weighty.
pub const ACCEL: f32 = 900.0;
/// Ground friction (px/s^2) applied when no direction key is held.
pub const FRICTION: f32 = 1200.0;
/// How far below the tile surface the player's feet may hang and still be
/// considered "on the ground" (snap tolerance).
pub const GROUND_SNAP: f32 = 8.0;
/// Player width in pixels.
pub const PLAYER_W: f32 = 18.0;
/// Player height in pixels.
pub const PLAYER_H: f32 = 22.0;
/// Contact distance for stomping / touching a Goomba.
pub const STOMP_RANGE: f32 = 10.0;
/// Coin pickup radius (center-to-center).
pub const COIN_RANGE: f32 = 16.0;
/// Vertical gap between flagpole base and the goal banner.
pub const FLAG_HEIGHT: f32 = 72.0;
/// Points awarded per coin.
pub const COIN_SCORE: u32 = 100;
/// Points awarded per stomped Goomba.
pub const STOMP_SCORE: u32 = 200;
/// Points awarded for reaching the flag.
pub const GOAL_SCORE: u32 = 1000;
/// Number of lives at the start of a run.
pub const START_LIVES: u32 = 3;
/// Milliseconds between animation frames while walking.
pub const ANIM_PERIOD: u32 = 150;

#[derive(Default)]
pub enum Scene {
    #[default]
    Welcome = 0,
    Playing,
    Paused,
    GameOver,
}

/// Level rows are plain strings so they read like classic ASCII maps:
/// `#` solid block, `B` brick, `?` question block (hides a coin),
/// `C` loose coin, `G` Goomba, `P` player spawn, `F` flagpole base.
pub const LEVELS: [&str; 3] = [
    "\
............................................................................
............................................................................
............................................................................
............................................................................
..........BB?BB.............................................................
............................................................................
....................CC......................................................
..............?...........................................BB?BB.............
............................................................................
...................................GG.........................................
............................................................................
####..###....#####....##....########....######....##....####........FFFF####
####..###....#####....##....########....######....##....####........FFFF####",
    "\
............................................................................
............................................................................
............................................................................
............................................................................
.......................................................?....................
............................................................................
..........................CC................................................
...............BB?BB..................BB?BB.................................
............................................................................
..........................GG................................GG..............
............................................................................
####....####....##....##....########....##....######....####........FFFF####
####....####....##....##....########....##....######....####........FFFF####",
    "\
............................................................................
............................................................................
............................................................................
............................................................................
..............................................BB?BB.........................
............................................................................
......................................................CC....................
..................................?.........................................
............................................................................
......................................GG................................GG..
............................................................................
####..##....######....##....####....######....##....#####........FFFF#######
####..##....######....##....####....######....##....#####........FFFF#######",
];

/// Axis-aligned rectangle in pixels.
#[derive(Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    /// A rect shrunk on all sides, used for forgiving collision boxes.
    pub fn inset(&self, dx: f32, dy: f32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            w: (self.w - 2.0 * dx).max(0.0),
            h: (self.h - 2.0 * dy).max(0.0),
        }
    }
    pub fn overlaps(&self, other: &Rect) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }
}

/// The plumber. `x`/`y` are the top-left corner in pixels; `vy > 0` falls.
#[derive(Default)]
pub struct Player {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub grounded: bool,
    pub facing: f32,
    pub anim_ms: u32,
    pub frame: u8,
    /// Buffered jump press consumed on landing (like the Runner's dino).
    pub want_jump: bool,
}

impl Player {
    pub fn rect(&self) -> Rect {
        Rect::new(self.x, self.y, PLAYER_W, PLAYER_H)
    }
    /// Collision box slightly narrower than the sprite so near-misses feel fair.
    pub fn hitbox(&self) -> Rect {
        self.rect().inset(2.0, 1.0)
    }
    pub fn reset(&mut self, col: usize, row: usize) {
        *self = Self {
            x: col as f32 * TILE,
            y: row as f32 * TILE + (TILE - PLAYER_H),
            facing: 1.0,
            ..Default::default()
        };
    }
    pub fn jump(&mut self) {
        if self.grounded {
            self.vy = JUMP_VELOCITY;
            self.grounded = false;
            self.want_jump = false;
        } else {
            self.want_jump = true;
        }
    }
    pub fn walk(&mut self, dir: f32) {
        self.vx += dir * ACCEL;
        self.vx = self.vx.clamp(-WALK_SPEED, WALK_SPEED);
        if dir != 0.0 {
            self.facing = dir.signum();
        }
    }
    /// Advance physics one tick. Returns true when a new step started
    /// (used by the view to toggle the walk animation frame).
    pub fn update(&mut self, dt: f32, solids: &[Rect]) -> bool {
        // Friction when no key is pushing.
        let push = self.vx.abs() < 0.01;
        if push {
            self.vx = 0.0;
        } else {
            let drop = FRICTION * dt;
            self.vx = if self.vx > 0.0 {
                (self.vx - drop).max(0.0)
            } else {
                (self.vx + drop).min(0.0)
            };
        }
        self.vy += GRAVITY * dt;
        // Horizontal move + resolve.
        self.x += self.vx * dt;
        for s in solids {
            let mut r = self.hitbox();
            if r.overlaps(s) {
                if self.vx > 0.0 {
                    self.x = s.x - PLAYER_W + 2.0;
                } else if self.vx < 0.0 {
                    self.x = s.x + s.w - 2.0;
                }
                self.vx = 0.0;
                r = self.hitbox();
                let _ = r;
            }
        }
        // Vertical move + resolve.
        let was_airborne = !self.grounded;
        self.y += self.vy * dt;
        self.grounded = false;
        for s in solids {
            if self.hitbox().overlaps(s) {
                if self.vy >= 0.0 {
                    self.y = s.y - PLAYER_H;
                    self.vy = 0.0;
                    self.grounded = true;
                    if std::mem::take(&mut self.want_jump) {
                        self.vy = JUMP_VELOCITY;
                        self.grounded = false;
                    }
                } else {
                    self.y = s.y + s.h - 1.0;
                    self.vy = 0.0;
                }
            }
        }
        // Step animation toggle.
        if self.grounded && self.vx.abs() > 20.0 {
            self.anim_ms += (dt * 1000.0) as u32;
            if self.anim_ms >= ANIM_PERIOD {
                self.anim_ms = 0;
                self.frame = (self.frame + 1) % 2;
                return true;
            }
        } else {
            self.anim_ms = 0;
        }
        was_airborne && self.grounded
    }
}

/// Walking enemy that patrols between two columns.
#[derive(Clone)]
pub struct Goomba {
    pub x: f32,
    pub y: f32,
    pub dir: f32,
    pub min_col: usize,
    pub max_col: usize,
    pub dead: bool,
    pub squash_ms: u32,
}

impl Goomba {
    pub fn rect(&self) -> Rect {
        Rect::new(self.x, self.y, TILE - 6.0, TILE - 6.0)
    }
    pub fn update(&mut self, dt: f32, cols: usize) {
        if self.dead {
            self.squash_ms = self.squash_ms.saturating_add((dt * 1000.0) as u32);
            return;
        }
        self.x += self.dir * 50.0 * dt;
        let col = ((self.x + (TILE - 6.0) / 2.0) / TILE) as i32;
        if col <= self.min_col as i32 {
            self.dir = 1.0;
        } else if col >= self.max_col.min(cols - 1) as i32 {
            self.dir = -1.0;
        }
    }
}

/// Solid tiles (ground, blocks) plus decorative bricks rendered by the view.
#[derive(Default)]
pub struct World {
    pub cols: usize,
    pub rows: usize,
    pub solids: Vec<Rect>,
    pub bricks: Vec<(usize, usize)>,
    pub coins: Vec<(f32, f32)>,
    pub goombas: Vec<Goomba>,
    pub flag_x: f32,
    pub flag_y: f32,
    pub spawn: (usize, usize),
    pub pit_y: f32,
}

impl World {
    /// Parses an ASCII level string into rectangles and entities.
    pub fn parse(level: &str) -> Self {
        let lines: Vec<&str> = level.lines().filter(|l| !l.trim().is_empty()).collect();
        let rows = lines.len();
        let cols = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let mut world = Self {
            cols,
            rows,
            spawn: (1, rows.saturating_sub(3)),
            pit_y: (rows as f32) * TILE + 64.0,
            ..Default::default()
        };
        for (r, line) in lines.iter().enumerate() {
            for (c, ch) in line.chars().enumerate() {
                let x = c as f32 * TILE;
                let y = r as f32 * TILE;
                match ch {
                    '#' => world.solids.push(Rect::new(x, y, TILE, TILE)),
                    'B' | '?' => {
                        world.solids.push(Rect::new(x, y, TILE, TILE));
                        world.bricks.push((c, r));
                    }
                    'C' => world.coins.push((x + TILE / 2.0, y + TILE / 2.0)),
                    'G' => {
                        world.goombas.push(Goomba {
                            x,
                            y: y + (TILE - (TILE - 6.0)),
                            dir: -1.0,
                            min_col: c.saturating_sub(3),
                            max_col: (c + 3).min(cols - 1),
                            dead: false,
                            squash_ms: 0,
                        });
                    }
                    'P' => world.spawn = (c, r),
                    'F' => {
                        world.flag_x = x + TILE / 2.0;
                        world.flag_y = y;
                    }
                    _ => {}
                }
            }
        }
        world
    }
}

/// Score/lives bookkeeping with best-score persistence helpers.
#[derive(Default)]
pub struct Score {
    pub current: u32,
    pub best: u32,
    pub lives: u32,
}

impl Score {
    pub fn reset_run(&mut self) {
        self.current = 0;
        self.lives = START_LIVES;
    }
    pub fn add(&mut self, points: u32) {
        self.current += points;
        if self.current > self.best {
            self.best = self.current;
        }
    }
    pub fn lose_life(&mut self) -> bool {
        if self.lives == 0 {
            return false;
        }
        self.lives -= 1;
        true
    }
}

/// Top-level simulation for one room/level.
pub struct Mario {
    pub scene: Scene,
    pub level: usize,
    pub world: World,
    pub player: Player,
    pub score: Score,
    pub won: bool,
    pub time_ms: u32,
    /// Feet position before the last physics step (for reliable stomps).
    pub prev_bottom: f32,
}

impl Default for Mario {
    fn default() -> Self {
        let mut m = Self {
            scene: Scene::Welcome,
            level: 0,
            world: World::parse(LEVELS[0]),
            player: Player::default(),
            score: Score::default(),
            won: false,
            time_ms: 0,
            prev_bottom: 0.0,
        };
        m.player
            .reset(m.world.spawn.0, m.world.spawn.1.max(m.world.rows - 2));
        m
    }
}

impl Mario {
    pub fn start(&mut self) {
        self.score.reset_run();
        self.load_level(0);
        self.scene = Scene::Playing;
    }
    pub fn load_level(&mut self, index: usize) {
        self.level = index % LEVELS.len();
        self.world = World::parse(LEVELS[self.level]);
        let spawn_row = self.world.spawn.1.max(self.world.rows.saturating_sub(3));
        self.player.reset(self.world.spawn.0, spawn_row);
        self.won = false;
    }
    pub fn next_level(&mut self) {
        self.load_level(self.level + 1);
    }
    pub fn respawn(&mut self) {
        let row = self.world.spawn.1.max(self.world.rows.saturating_sub(3));
        self.player.reset(self.world.spawn.0, row);
    }
    /// Advances the whole simulation by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        if !matches!(self.scene, Scene::Playing) {
            return;
        }
        self.time_ms += (dt * 1000.0) as u32;
        let solids = self.world.solids.clone();
        self.prev_bottom = self.player.y + PLAYER_H;
        self.player.update(dt, &solids);
        // Coins.
        let pc = (
            self.player.x + PLAYER_W / 2.0,
            self.player.y + PLAYER_H / 2.0,
        );
        self.world.coins.retain(|(cx, cy)| {
            let keep = (pc.0 - cx).hypot(pc.1 - cy) > COIN_RANGE;
            if !keep {
                self.score.add(COIN_SCORE);
            }
            keep
        });
        // Goombas.
        for g in &mut self.world.goombas {
            g.update(dt, self.world.cols);
        }
        let pr = self.player.rect();
        for g in &mut self.world.goombas {
            if g.dead {
                continue;
            }
            let gr = g.rect();
            if !pr.overlaps(&gr) {
                continue;
            }
            let falling = self.player.vy > 0.0 || self.prev_bottom <= gr.y + 4.0;
            let stomping = falling && pr.y + pr.h - gr.y < STOMP_RANGE;
            if stomping {
                g.dead = true;
                self.score.add(STOMP_SCORE);
                self.player.vy = JUMP_VELOCITY * 0.6;
            } else {
                self.hit();
                break;
            }
        }
        // Fell into a pit.
        if self.player.y > self.world.pit_y {
            self.hit();
        }
        // Reached the flag.
        if !self.won
            && (self.player.x + PLAYER_W / 2.0 - self.world.flag_x).abs() < TILE
            && self.player.y + PLAYER_H >= self.world.flag_y - FLAG_HEIGHT
        {
            self.won = true;
            self.score.add(GOAL_SCORE);
            self.scene = Scene::GameOver;
        }
        // Clean up squashed goombas after a short delay.
        self.world.goombas.retain(|g| !g.dead || g.squash_ms < 400);
    }
    fn hit(&mut self) {
        if self.score.lose_life() {
            self.respawn();
        } else {
            self.scene = Scene::GameOver;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_have_entities() {
        for level in LEVELS {
            let world = World::parse(level);
            assert!(!world.solids.is_empty());
            assert!(!world.goombas.is_empty());
            assert!(!world.coins.is_empty());
            assert!(world.flag_x > 0.0);
        }
    }

    #[test]
    fn player_lands_on_solid_ground() {
        let world = World::parse(LEVELS[0]);
        let mut player = Player::default();
        player.reset(world.spawn.0, world.spawn.1);
        let solids = world.solids.clone();
        let mut landed = false;
        for _ in 0..600 {
            landed = player.update(1.0 / 60.0, &solids);
            if landed || player.grounded {
                break;
            }
        }
        assert!(player.grounded, "player should settle on the ground");
    }

    /// Drops a stationary goomba right in front of the player so the next
    /// tick produces a clean overlap (without the enemy moving away first).
    fn place_goomba_in_front(mario: &mut Mario, dy: f32) {
        let mut g = mario.world.goombas.remove(0);
        g.x = mario.player.x + PLAYER_W / 2.0 - (TILE - 6.0) / 2.0;
        g.y = mario.player.y + PLAYER_H - 4.0 + dy;
        g.dir = 0.0;
        mario.world.goombas.push(g);
    }

    #[test]
    fn stomp_kills_goomba_and_bounces() {
        let mut mario = Mario::default();
        mario.start();
        // Freeze the player in mid-air above a stationary goomba so only the
        // vertical (stomp) collision path runs.
        for g in &mut mario.world.goombas {
            g.dir = 0.0;
        }
        let target_y = mario.player.y + PLAYER_H + 4.0;
        let mut g = mario.world.goombas.remove(0);
        g.x = mario.player.x + PLAYER_W / 2.0 - (TILE - 6.0) / 2.0;
        g.y = target_y;
        mario.world.goombas.push(g);
        mario.player.vy = 100.0;
        // Hold the player at a fixed height: update() re-applies gravity and
        // ground snapping, which would otherwise change vy before the check.
        let solids = std::mem::take(&mut mario.world.solids);
        mario.update(1.0 / 60.0);
        mario.world.solids = solids;
        println!(
            "player y={} vy={} g0 dead={} x={} y={}",
            mario.player.y, mario.player.vy, mario.world.goombas[0].dead, mario.world.goombas[0].x, mario.world.goombas[0].y
        );
        assert!(mario.world.goombas[0].dead);
        assert!(mario.player.vy < 0.0, "player should bounce upward");
        assert_eq!(mario.score.current, STOMP_SCORE);
    }

    #[test]
    fn side_contact_costs_a_life() {
        let mut mario = Mario::default();
        mario.start();
        let lives = mario.score.lives;
        place_goomba_in_front(&mut mario, -PLAYER_H / 2.0);
        mario.update(1.0 / 60.0);
        assert_eq!(mario.score.lives, lives - 1);
    }

    #[test]
    fn coin_pickup_scores() {
        let mut mario = Mario::default();
        mario.start();
        let before = mario.score.current;
        mario.world.coins.push((
            mario.player.x + PLAYER_W / 2.0,
            mario.player.y + PLAYER_H / 2.0,
        ));
        mario.update(1.0 / 60.0);
        assert_eq!(mario.score.current, before + COIN_SCORE);
    }
}
