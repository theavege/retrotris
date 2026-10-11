use crate::gui::*;
use crate::models::mario::*;

/// Camera follows the player horizontally (the level is wider than the screen).
const HUD_H: i32 = 40;

#[derive(Default)]
pub struct Model {
    game: Mario,
    size: (i32, i32),
    /// Held-key state so movement stays smooth between key events.
    left: bool,
    right: bool,
}

impl Console for Model {
    fn load(&mut self, path: &str) {
        if let Ok(data) = std::fs::read_to_string(path) {
            self.game.score.best = data.trim().parse().unwrap_or_default();
        }
    }
    fn exit(&self, path: &str) {
        let _ = std::fs::write(path, self.game.score.best.to_string());
    }
    fn handle(&mut self, window: &mut Window, event: Event) -> bool {
        match event {
            Event::Focus => true,
            Event::Resize => {
                self.size = (window.w(), window.h());
                if let Scene::Playing = self.game.scene {
                    self.game.scene = Scene::Paused;
                }
                true
            }
            Event::KeyDown => {
                const SPACE: Key = Key::from_char(' ');
                const A: Key = Key::from_char('a');
                const D: Key = Key::from_char('d');
                match event_key() {
                    Key::Escape => match self.game.scene {
                        Scene::Playing => self.game.scene = Scene::Paused,
                        Scene::Paused => self.game.scene = Scene::Playing,
                        Scene::GameOver => self.game.scene = Scene::Welcome,
                        Scene::Welcome => crate::to_menu(window),
                    },
                    Key::Enter | SPACE => match self.game.scene {
                        Scene::Playing => self.game.jump(),
                        Scene::Paused => self.game.scene = Scene::Playing,
                        Scene::GameOver => {
                            if self.game.won {
                                self.game.next_level();
                                self.game.scene = Scene::Playing;
                            } else {
                                self.game.start();
                            }
                        }
                        Scene::Welcome => self.game.start(),
                    },
                    Key::Left | A => self.left = true,
                    Key::Right | D => self.right = true,
                    _ => return false,
                };
                window.redraw();
                true
            }
            Event::KeyUp => {
                const A: Key = Key::from_char('a');
                const D: Key = Key::from_char('d');
                match event_key() {
                    Key::Left | A => self.left = false,
                    Key::Right | D => self.right = false,
                    _ => return false,
                };
                window.redraw();
                true
            }
            _ => false,
        }
    }
    fn update(&mut self, dt: f32) {
        if let Scene::Playing = self.game.scene {
            let dir = self.right as i8 - self.left as i8;
            self.game.player.walk(dir as f32);
            // Clamp dt so a stalled tab cannot tunnel through floors.
            self.game.update(dt.min(0.05));
        }
    }
    fn draw(&self, window: &mut Window) {
        window.draw_background(Color::Background);
        // Play field frame.
        window.draw_rect(
            PAD,
            HUD_H,
            window.w() - PAD * 2,
            window.h() - HUD_H - PAD,
            10,
            Color::Background2,
        );
        self.draw_hud(window);
        match self.game.scene {
            Scene::Welcome => window.draw_welcome(
                "Super Plumber",
                &[
                    &["BEST SCORE", &self.game.score.best.to_string()],
                    &["PRESS <ENTER>", "for play"],
                    &["PRESS <ESC>", "for exit"],
                    &["<LEFT/RIGHT> or <A/D>", "to walk"],
                    &["<SPACE> or <UP>", "to jump"],
                ],
            ),
            Scene::Playing | Scene::Paused => {
                self.draw_world(window);
                if let Scene::Paused = self.game.scene {
                    window.draw_overlay("PAUSED", "PRESS ENTER to resume", sol::VIOLET);
                }
            }
            Scene::GameOver => {
                self.draw_world(window);
                let title = match self.game.won {
                    true => "ROOM CLEAR!",
                    false => "GAME OVER",
                };
                let color = match self.game.won {
                    true => sol::GREEN,
                    false => sol::RED,
                };
                window.draw_overlay(
                    title,
                    &format!(
                        "SCORE: {}   BEST: {}",
                        self.game.score.current, self.game.score.best
                    ),
                    color,
                );
                let hint = match self.game.won {
                    true => "PRESS ENTER for next room, ESC for menu",
                    false => "PRESS ENTER for retry, ESC for menu",
                };
                window.draw_text(
                    hint,
                    0,
                    window.h() / 2 + HEIGHT,
                    Color::Foreground,
                    Align::Center,
                    20,
                );
            }
        }
    }
}

impl Model {
    fn jump(&mut self) {
        self.game.player.jump();
    }
    fn draw_hud(&self, window: &Window) {
        window.draw_text(
            &format!(
                "SCORE {:06}   BEST {:06}   ROOM {}",
                self.game.score.current,
                self.game.score.best,
                self.game.level + 1,
            ),
            PAD,
            6,
            Color::Foreground,
            Align::Left,
            22,
        );
        // Lives as little plumber caps on the right.
        for i in 0..self.game.score.lives {
            let x = window.w() - PAD - 22 - i as i32 * 26;
            draw::draw_rect_fill(x, 10, 18, 12, sol::RED);
            draw::draw_rect_fill(x - 4, 20, 26, 4, sol::BASE02);
        }
    }
    /// Horizontal camera offset keeping the player around 1/3 of the screen.
    fn camera_x(&self) -> f32 {
        let view_w = (self.size.0 - PAD * 2) as f32;
        let world_w = self.game.world.cols as f32 * TILE;
        let raw = self.game.player.x - view_w / 3.0;
        raw.clamp(0.0, (world_w - view_w).max(0.0))
    }
    fn draw_world(&self, window: &Window) {
        let cam = self.camera_x();
        let ox = PAD as f32 - cam;
        let oy = HUD_H as f32;
        let sky = window.w() - PAD * 2;
        let ground_row = self.game.world.rows - 2;
        // Decorative clouds and hills parallax at half camera speed.
        draw::set_draw_color(sol::BASE1);
        for k in 0..6 {
            let cx = ox * 0.5 + (k as f32) * 300.0 + 60.0;
            let cy = oy + 30.0 + (k % 3) as f32 * 24.0;
            if cx > -80.0 && cx < sky as f32 + 80.0 {
                draw::draw_ellipsef(cx + PAD as f32, cy, 70.0, 22.0);
            }
        }
        draw::set_draw_color(sol::BASE2);
        for k in 0..5 {
            let hx = ox * 0.7 + (k as f32) * 420.0;
            let hy = oy + (ground_row as f32) * TILE;
            if hx > -160.0 && hx < sky as f32 + 160.0 {
                draw::draw_pie(hx as i32, hy as i32, 160, 90, 0.0, 180.0);
            }
        }
        // Solid tiles.
        for s in &self.game.world.solids {
            let x = ox + s.x;
            if x + s.w < PAD as f32 || x > window.w() as f32 - PAD as f32 {
                continue;
            }
            let y = oy + s.y;
            let row = (s.y / TILE) as usize;
            let color = match row >= ground_row {
                true => sol::ORANGE, // earth
                false => sol::BASE02, // floating blocks
            };
            draw::draw_rect_fill(x as i32, y as i32, s.w as i32, s.h as i32, color);
            draw::draw_rect_fill(
                x as i32,
                y as i32,
                s.w as i32,
                4,
                match row >= ground_row {
                    true => sol::YELLOW,
                    false => sol::BASE01,
                },
            );
        }
        // Bricks with their type marker.
        for (c, r) in &self.game.world.bricks {
            let x = ox + *c as f32 * TILE;
            if x + TILE < PAD as f32 || x > window.w() as f32 - PAD as f32 {
                continue;
            }
            let y = oy + *r as f32 * TILE;
            let ch = '?' ; // both B and ? are stored together; render a knob
            let _ = ch;
            draw::draw_rect_fill((x + TILE / 4.0) as i32, (y + TILE / 4.0) as i32, 12, 12, sol::YELLOW);
        }
        // Coins: spinning via time-based width wobble.
        let spin = ((self.game.time_ms % 800) as f32 / 800.0 * std::f32::consts::TAU).cos();
        for (cx, cy) in &self.game.world.coins {
            let x = ox + cx;
            if x < PAD as f32 || x > window.w() as f32 - PAD as f32 {
                continue;
            }
            let w = (8.0 * spin.abs()).max(2.0);
            draw::draw_ellipsef(x, oy + cy, w * 2.0, 16.0);
            draw::set_draw_color(sol::YELLOW);
            draw::draw_arc(x as i32, (oy + cy) as i32, w as i32 * 2, 16, 0, 360);
        }
        // Flagpole with banner.
        let fx = ox + self.game.world.flag_x;
        if fx > PAD as f32 && fx < window.w() as f32 - PAD as f32 {
            let fy = oy + self.game.world.flag_y;
            draw::draw_rect_fill(fx as i32, (fy - FLAG_HEIGHT) as i32, 4, FLAG_HEIGHT as i32, sol::BASE01);
            draw::draw_circle(fx + 2.0, fy - FLAG_HEIGHT, 6.0, sol::YELLOW);
            draw::set_draw_color(sol::GREEN);
            draw::draw_polygon(&[
                (fx as i32 + 4, (fy - FLAG_HEIGHT) as i32 + 4),
                (fx as i32 + 30, (fy - FLAG_HEIGHT) as i32 + 12),
                (fx as i32 + 4, (fy - FLAG_HEIGHT) as i32 + 24),
            ]);
        }
        // Goombas.
        for g in &self.game.world.goombas {
            let x = ox + g.x;
            if x + TILE < PAD as f32 || x > window.w() as f32 - PAD as f32 {
                continue;
            }
            let y = oy + g.y;
            let h = TILE - 6.0;
            if g.dead {
                // Squashed pancake.
                draw::draw_rect_fill(x as i32, (y + h - 6.0) as i32, h as i32 + 6, 6, sol::BASE00);
            } else {
                draw::draw_rect_fill(x as i32, y as i32, (TILE - 6.0) as i32, (h * 0.7) as i32, sol::BASE00);
                draw::draw_rect_fill(
                    (x + 3.0) as i32,
                    (y + h * 0.7) as i32,
                    (TILE - 12.0) as i32,
                    (h * 0.3) as i32,
                    sol::BASE02,
                );
                // Eyes facing the walk direction.
                let ex = match g.dir >= 0.0 {
                    true => x + TILE - 12.0,
                    false => x + 6.0,
                };
                draw::draw_rect_fill(ex as i32, (y + 5.0) as i32, 4, 6, sol::BASE3);
            }
        }
        // The plumber: red cap, blue overalls, animated legs.
        let p = &self.game.player;
        let px = ox + p.x;
        let py = oy + p.y;
        draw::draw_rect_fill(px as i32, py as i32, PLAYER_W as i32, 7, sol::RED); // cap
        draw::draw_rect_fill(
            (px + 2.0) as i32,
            (py + 7.0) as i32,
            (PLAYER_W - 4.0) as i32,
            8,
            sol::BASE3,
        ); // face
        draw::draw_rect_fill(px as i32, (py + 15.0) as i32, PLAYER_W as i32, 4, sol::BLUE); // body
        let leg_off = match p.frame {
            1 => 3.0,
            _ => 0.0,
        };
        draw::draw_rect_fill((px + 1.0) as i32, (py + 19.0) as i32, 6, 3 + leg_off as i32, sol::BLUE);
        draw::draw_rect_fill(
            (px + PLAYER_W - 7.0) as i32,
            (py + 19.0) as i32,
            6,
            3 + (3.0 - leg_off) as i32,
            sol::BLUE,
        );
        // Facing marker (nose).
        let nx = match p.facing >= 0.0 {
            true => px + PLAYER_W - 5.0,
            false => px + 1.0,
        };
        draw::draw_point(nx as i32, (py + 11.0) as i32, sol::BASE02);
    }
}
