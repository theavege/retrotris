use crate::gui::*;
use crate::models::asteroid::*;

#[derive(Default, PartialEq)]
enum Scene {
    #[default]
    Welcome,
    Playing,
    Paused,
    GameOver,
}

/// Logical field units map onto the window with a uniform scale that keeps
/// the whole square playfield visible (letterboxed when the window is wide).
struct View {
    scale: f32,
    ox: f32,
    oy: f32,
}

impl View {
    fn new(field: (i32, i32)) -> Self {
        let scale = (field.0 as f32 / FIELD_W).min(field.1 as f32 / FIELD_H);
        Self {
            scale,
            ox: (field.0 as f32 - FIELD_W * scale) / 2.0,
            oy: (field.1 as f32 - FIELD_H * scale) / 2.0,
        }
    }
    fn px(&self, p: (f32, f32)) -> (i32, i32) {
        (
            (self.ox + p.0 * self.scale) as i32,
            (self.oy + p.1 * self.scale) as i32,
        )
    }
    fn r(&self, rad: f32) -> i32 {
        (rad * self.scale) as i32
    }
}

const SHOT_COOLDOWN: f32 = 0.22;
const BASE01: Color = Color::from_rgb(147, 153, 153);
const BASE1: Color = Color::from_rgb(131, 148, 150);

#[derive(Default)]
pub struct Model {
    field: (i32, i32),
    scene: Scene,
    ship: Ship,
    belt: Belt,
    shots: Shots,
    score: Score,
    timer: f32,
    fire_timer: f32,
    left: bool,
    right: bool,
    thrust: bool,
    shot_held: bool,
}

impl Model {
    fn restart(&mut self) {
        self.ship = Ship::new();
        self.belt = Belt::default();
        self.shots.clear();
        self.score.reset();
        self.belt.seed(self.score.seeds(), &mut rand::rng());
        self.timer = 0.0;
        self.fire_timer = 0.0;
        self.left = false;
        self.right = false;
        self.thrust = false;
        self.shot_held = false;
        self.scene = Scene::Playing;
    }
    fn shoot(&mut self) {
        if self.scene != Scene::Playing || self.fire_timer > 0.0 {
            return;
        }
        self.fire_timer = SHOT_COOLDOWN;
        let (x, y) = self.ship.muzzle();
        self.shots.fire((x, y), self.ship.angle);
    }
    /// Respawn after losing a life, or end the run when lives are exhausted.
    fn lose_life(&mut self) {
        self.score.lose_life();
        self.shots.clear();
        if self.score.lives <= 0 {
            self.scene = Scene::GameOver;
        } else {
            self.ship = Ship::new();
        }
    }
    fn draw_score(&self, window: &Window) {
        let line = format!(
            "SCORE {}   BEST {}   WAVE {}   LIVES {}",
            self.score.points, self.score.best, self.score.wave, self.score.lives
        );
        window.draw_text(&line, window.w() / 2, PAD, Color::Foreground, Align::Center, 24);
    }
    /// Wireframe triangle for the ship, rotated by its heading.
    fn draw_ship(&self, view: &View) {
        let a = self.ship.angle;
        let r = self.ship.body.rad;
        let nose = (
            self.ship.body.pos.0 + a.cos() * r,
            self.ship.body.pos.1 + a.sin() * r,
        );
        let back = (
            self.ship.body.pos.0 - a.cos() * r * 0.8 - a.sin() * r * 0.7,
            self.ship.body.pos.1 - a.sin() * r * 0.8 + a.cos() * r * 0.7,
        );
        let front = (
            self.ship.body.pos.0 - a.cos() * r * 0.8 + a.sin() * r * 0.7,
            self.ship.body.pos.1 - a.sin() * r * 0.8 - a.cos() * r * 0.7,
        );
        draw::set_draw_color(if self.ship.vulnerable() {
            Color::Foreground
        } else {
            Color::Inactive // dimmed while the respawn shield is up
        });
        let (x1, y1) = view.px(nose);
        let (x2, y2) = view.px(back);
        let (x3, y3) = view.px(front);
        draw::draw_line(x1, y1, x2, y2);
        draw::draw_line(x2, y2, x3, y3);
        draw::draw_line(x3, y3, x1, y1);
    }
}

impl Console for Model {
    fn load(&mut self, path: &str) {
        if let Ok(data) = std::fs::read_to_string(path) {
            self.score.best = data.trim().parse().unwrap_or_default();
        }
    }
    fn exit(&self, path: &str) {
        let _ = std::fs::write(path, self.score.best.to_string());
    }
    fn handle(&mut self, window: &mut Window, event: Event) -> bool {
        const LEFT: Key = Key::Left;
        const RIGHT: Key = Key::Right;
        const UP: Key = Key::Up;
        const A: Key = Key::from_char('a');
        const D: Key = Key::from_char('d');
        const W: Key = Key::from_char('w');
        const SPACE: Key = Key::from_char(' ');
        match event {
            Event::Focus => true,
            Event::Resize => {
                self.field = (window.w(), window.h());
                if self.scene == Scene::Playing {
                    self.scene = Scene::Paused;
                }
                true
            }
            Event::KeyDown => {
                match event_key() {
                    Key::Escape => match self.scene {
                        Scene::Playing => self.scene = Scene::Paused,
                        Scene::Paused => self.scene = Scene::Playing,
                        Scene::GameOver => self.scene = Scene::Welcome,
                        Scene::Welcome => crate::to_menu(window),
                    },
                    Key::Enter => match self.scene {
                        Scene::Welcome | Scene::GameOver => self.restart(),
                        Scene::Paused => self.scene = Scene::Playing,
                        Scene::Playing => {}
                    },
                    LEFT | A => self.left = true,
                    RIGHT | D => self.right = true,
                    UP | W => self.thrust = true,
                    SPACE => {
                        self.shot_held = true;
                        self.shoot();
                    }
                    _ => return false,
                };
                window.redraw();
                true
            }
            Event::KeyUp => {
                match event_key() {
                    LEFT | A => self.left = false,
                    RIGHT | D => self.right = false,
                    UP | W => self.thrust = false,
                    SPACE => self.shot_held = false,
                    _ => return false,
                };
                true
            }
            _ => false,
        }
    }
    fn update(&mut self, dt: f32) {
        if self.scene != Scene::Playing {
            return;
        }
        // Buffered fire: holding space keeps shooting as soon as the cooldown ends.
        if self.shot_held {
            self.shoot();
        }
        self.timer += dt;
        while self.timer >= TICK {
            self.timer -= TICK;
            self.fire_timer -= TICK;
            if self.left {
                self.ship.turn(-1.0, TICK);
            }
            if self.right {
                self.ship.turn(1.0, TICK);
            }
            if self.thrust {
                self.ship.thrust(TICK);
            }
            self.ship.fly(TICK);
            self.belt.fly(TICK);
            self.shots.fly(TICK);
            let mut rng = rand::rng();
            // Bullets vs. rocks: overlapping bullets shatter the rocks.
            for scale in self.belt.shot(&mut self.shots, &mut rng) {
                self.score.rock(scale);
            }
            // Ship vs. rocks (only when the respawn shield has faded).
            if self.ship.vulnerable() && self.belt.hits(&self.ship.body) {
                self.lose_life();
                if self.scene != Scene::Playing {
                    return;
                }
            }
            // Wave cleared: seed a slightly denser belt.
            if self.belt.is_empty() {
                self.score.wave_cleared();
                let count = self.score.seeds();
                self.belt.seed(count, &mut rng);
                self.ship.protect();
            }
        }
    }
    fn draw(&self, window: &mut Window) {
        window.draw_background(Color::Background);
        match self.scene {
            Scene::Welcome => window.draw_welcome(
                "Asteroids",
                &[
                    &["BEST SCORE", &self.score.best.to_string()],
                    &["PRESS ENTER", "for play"],
                    &["PRESS ESC", "for exit"],
                ],
            ),
            Scene::Playing | Scene::Paused => {
                self.draw_score(window);
                let view = View::new(self.field);
                // Playfield border so the wrap-around edges are visible.
                let (x, y) = view.px((0.0, 0.0));
                window.draw_rect(x, y, view.r(FIELD_W), view.r(FIELD_H), 0, BASE1);
                // Rocks: wireframe circles with a rotating notch for spin.
                draw::set_draw_color(BASE01);
                for rock in self.belt.rocks() {
                    let (cx, cy) = view.px(rock.body.pos);
                    let r = view.r(rock.body.rad);
                    draw::draw_circle(f64::from(cx - r), f64::from(cy - r), f64::from(2 * r));
                    let hx = cx + (rock.heading.cos() * r as f32) as i32;
                    let hy = cy + (rock.heading.sin() * r as f32) as i32;
                    draw::draw_line(cx, cy, hx, hy);
                }
                // Bullets: tiny filled squares.
                for b in self.shots.bodies() {
                    let (x, y) = view.px(b.pos);
                    draw::draw_rect_fill(x - 2, y - 2, 4, 4, Color::Magenta);
                }
                self.draw_ship(&view);
                // Engine flame while thrusting.
                if self.thrust {
                    let a = self.ship.angle;
                    let tail = (
                        self.ship.body.pos.0 - a.cos() * self.ship.body.rad * 1.2,
                        self.ship.body.pos.1 - a.sin() * self.ship.body.rad * 1.2,
                    );
                    let (tx, ty) = view.px(tail);
                    draw::draw_circle_fill(tx - 4, ty - 4, 8, Color::from_rgb(203, 75, 22));
                }
                if self.scene == Scene::Paused {
                    window.draw_overlay("PAUSED", "PRESS ENTER to resume", Color::Background2);
                }
            }
            Scene::GameOver => {
                window.draw_welcome(
                    "Game Over",
                    &[
                        &["FINAL SCORE", &self.score.points.to_string()],
                        &["WAVES CLEARED", &self.score.wave.to_string()],
                        &["PRESS ENTER", "to retry"],
                        &["PRESS ESC", "for menu"],
                    ],
                );
            }
        }
    }
}
