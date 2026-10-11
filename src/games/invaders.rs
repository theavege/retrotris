use crate::gui::*;
use crate::models::invaders::*;

#[derive(Default, PartialEq)]
enum Scene {
    #[default]
    Welcome,
    Playing,
    Paused,
    GameOver,
}

/// Alien formation geometry: the horde occupies `Horde::ROWS` rows starting
/// under the score HUD; the ship patrols the last playfield row.
const FIELD_TOP: i32 = PAD * 5;
/// Cell size in pixels (the field is COLS x ROWS of these).
fn cell(field: (i32, i32)) -> (f32, f32) {
    (
        field.0 as f32 / COLS as f32,
        (field.1 - FIELD_TOP) as f32 / (Horde::ROWS + 4) as f32,
    )
}

#[derive(Default)]
pub struct Model {
    field: (i32, i32),
    scene: Scene,
    ship: Ship,
    horde: Horde,
    shots: Shots,
    score: Score,
    timer: f32,
    fire_timer: f32,
    beat: bool,
    shot_held: bool,
}

impl Model {
    fn restart(&mut self) {
        self.ship = Ship::new();
        self.horde = Horde::new();
        self.shots.clear();
        self.score.reset();
        self.timer = 0.0;
        self.fire_timer = 0.0;
        self.beat = false;
        self.scene = Scene::Playing;
    }
    /// Grid coordinates of the ship row (bottom-most playfield row).
    fn ship_row(&self) -> usize {
        Horde::ROWS + 2
    }
    fn draw_score(&self, window: &Window) {
        let line = format!(
            "SCORE {}   BEST {}   WAVE {}   LIVES {}",
            self.score.points, self.score.best, self.score.wave, self.score.lives
        );
        let (w, h) = draw::measure(&line, false);
        draw::set_font(Font::CourierBold, 24);
        draw::set_draw_color(Color::Foreground);
        draw::draw_text2(
            &line,
            window.w() / 2 - w / 2,
            PAD,
            w,
            h + i32::from(h == 0),
            Align::Center,
        );
    }
    /// Pixel rectangle of a grid cell (col, row), inset by a small margin.
    fn rect(&self, col: usize, row: usize) -> (i32, i32, i32, i32) {
        let (cw, ch) = cell(self.field);
        (
            (col as f32 * cw + cw * 0.1) as i32,
            (FIELD_TOP as f32 + row as f32 * ch + ch * 0.1) as i32,
            (cw * 0.8) as i32,
            (ch * 0.8) as i32,
        )
    }
    /// Convert a continuous projectile position to its grid cell.
    fn cell_of(&self, p: (f32, f32)) -> (usize, usize) {
        (
            p.0.clamp(0.0, COLS as f32 - 0.01) as usize,
            p.1.max(0.0) as usize,
        )
    }
    fn shoot(&mut self) {
        if self.scene != Scene::Playing {
            return;
        }
        let level = self.score.level();
        if self.ship.shoot(level) {
            self.shots
                .fire_bullet(self.ship.center(), self.ship_row() as f32 - 0.5);
        }
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
        match event {
            Event::Focus => true,
            Event::Resize => {
                self.field = (window.w(), window.h());
                match self.scene {
                    Scene::Playing => self.scene = Scene::Paused,
                    Scene::Welcome | Scene::GameOver => {}
                    Scene::Paused => {}
                }
                true
            }
            Event::KeyDown => {
                const LEFT: Key = Key::from_char('a');
                const RIGHT: Key = Key::from_char('d');
                const SPACE: Key = Key::from_char(' ');
                match event_key() {
                    Key::Escape => match self.scene {
                        Scene::Playing => self.scene = Scene::Paused,
                        Scene::Paused => self.scene = Scene::Playing,
                        Scene::GameOver => self.scene = Scene::Welcome,
                        Scene::Welcome => crate::Model::connect(window),
                    },
                    Key::Enter => match self.scene {
                        Scene::Welcome | Scene::GameOver => self.restart(),
                        Scene::Paused => self.scene = Scene::Playing,
                        Scene::Playing => {}
                    },
                    SPACE => {
                        self.shot_held = true;
                        self.shoot();
                    }
                    Key::Left | LEFT => {
                        if self.scene == Scene::Playing {
                            self.ship.slide(-1, COLS as usize);
                        }
                    }
                    Key::Right | RIGHT => {
                        if self.scene == Scene::Playing {
                            self.ship.slide(1, COLS as usize);
                        }
                    }
                    _ => return false,
                };
                window.redraw();
                true
            }
            Event::KeyUp => {
                if event_key() == Key::from_char(' ') {
                    self.shot_held = false;
                    return true;
                }
                false
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
            let dt = TICK;
            self.ship.reload(dt);
            // March the horde; each column step flips the two-frame animation.
            let step = self.score.march_speed() * dt;
            if self.horde.march(step) {
                self.beat = !self.beat;
            }
            // Aliens drop bombs at a wave-dependent rate from their column.
            self.fire_timer += dt;
            if self.fire_timer >= 1.0 / self.score.alien_rate() {
                self.fire_timer = 0.0;
                if let Some(col) = self.horde.shooter(&mut rand::rng()) {
                    if let Some(row) = self.horde.deepest_in(col) {
                        self.shots.drop_bomb(col as f32 + 0.5, row as f32 + 1.0);
                    }
                }
            }
            self.shots.fly(dt, self.score.level(), self.ship_row());
            // Player bullets vs. the horde.
            let hits: Vec<(usize, usize)> = self
                .shots
                .bullets()
                .iter()
                .map(|&p| self.cell_of(p))
                .filter(|&(c, r)| r < Horde::ROWS && self.horde.alive(c, r))
                .collect();
            for (col, row) in hits {
                if self.shots.take_bullet_at(col, row) {
                    self.horde.kill(col, row);
                    self.score.alien();
                }
            }
            // Alien bombs vs. the ship.
            let ship_cell = (self.ship.col, self.ship_row());
            if self.shots.take_bomb_at(ship_cell.0, ship_cell.1) {
                self.score.lose_life();
                self.shots.clear();
                if self.score.lives <= 0 {
                    self.scene = Scene::GameOver;
                    return;
                }
            }
            // The horde reached the ship row: instant defeat.
            if self.horde.deepest_row() == Some(self.ship_row() - 1) {
                self.scene = Scene::GameOver;
                return;
            }
            // Wave cleared: rebuild a faster formation.
            if !self.horde.any() {
                self.score.wave_cleared();
                self.horde = Horde::new();
                self.shots.clear();
                self.fire_timer = 0.0;
            }
        }
    }
    fn draw(&self, window: &mut Window) {
        draw::draw_rect_fill(0, 0, window.width(), window.height(), Color::Background);
        match self.scene {
            Scene::Welcome => window.draw_welcome(
                "Invaders",
                &[
                    &["BEST SCORE", &self.score.best.to_string()],
                    &["PRESS ENTER", "for play"],
                    &["PRESS ESC", "for exit"],
                ],
            ),
            Scene::Playing | Scene::Paused => {
                self.draw_score(window);
                // Aliens: outer rows magenta, middle rows cyan; the two-frame
                // march alternates between square and diamond silhouettes.
                for row in 0..Horde::ROWS {
                    for col in 0..COLS as usize {
                        if !self.horde.alive(col, row) {
                            continue;
                        }
                        let (x, y, w, h) = self.rect(col, row);
                        let color = match row {
                            0 | 3 => Color::Magenta,
                            _ => Color::Cyan,
                        };
                        draw::set_draw_color(color);
                        if self.beat {
                            draw::draw_rect_fill(x, y, w, h, color);
                        } else {
                            let (dx, dy) = (w / 4, h / 4);
                            // Chamfered "space invader" silhouette: an octagon.
                            draw::draw_circle_fill(x + dx, y + dy, w - 2 * dx, color);
                        }
                    }
                }
                // Ship: a solid block with a turret notch.
                let (x, y, w, h) = self.rect(self.ship.col, self.ship_row());
                draw::draw_rect_fill(x, y, w, h, Color::Foreground);
                draw::draw_rect_fill(x + w / 3, y - h / 3, w / 3, h / 3, Color::Foreground);
                // Projectiles.
                for &p in self.shots.bullets() {
                    let (cx, cy) = (
                        p.0 * cell(self.field).0,
                        FIELD_TOP as f32 + p.1 * cell(self.field).1,
                    );
                    draw::draw_rect_fill(cx as i32, cy as i32, 4, 12, Color::Inactive);
                }
                for &p in self.shots.bombs() {
                    let (cx, cy) = (
                        p.0 * cell(self.field).0,
                        FIELD_TOP as f32 + p.1 * cell(self.field).1,
                    );
                    draw::draw_rect_fill(cx as i32, cy as i32, 6, 6, Color::Red);
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
