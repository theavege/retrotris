use crate::gui::*;
use crate::models::sokoban::*;

#[derive(Default, PartialEq)]
enum Scene {
    #[default]
    Welcome,
    Playing,
    Paused,
    Solved,
}

/// Cell size in pixels; the board is centered in the window below the HUD.
const CELL: i32 = 48;

#[derive(Default)]
pub struct Model {
    field: (i32, i32),
    scene: Scene,
    board: Sokoban,
    score: Score,
    history: Vec<(isize, isize)>,
}

impl Model {
    fn restart(&mut self) {
        self.board = Sokoban::level(self.score.level);
        self.history.clear();
        self.scene = Scene::Playing;
    }
    /// Top-left pixel of the board, centered in the current field.
    fn origin(&self) -> (i32, i32) {
        let (rows, cols) = self.board.dims();
        (
            (self.field.0 - cols as i32 * CELL) / 2,
            PAD * 4 + (self.field.1 - PAD * 4 - rows as i32 * CELL) / 2,
        )
    }
    /// Pixel rectangle of one cell, inset by a small margin.
    fn rect(&self, r: usize, c: usize) -> (i32, i32, i32, i32) {
        let (ox, oy) = self.origin();
        (
            ox + c as i32 * CELL + 2,
            oy + r as i32 * CELL + 2,
            CELL - 4,
            CELL - 4,
        )
    }
    /// Apply one move and remember it for undo.
    fn step(&mut self, dr: isize, dc: isize) {
        if self.scene != Scene::Playing {
            return;
        }
        if self.board.push(dr, dc) {
            self.history.push((dr, dc));
            if self.board.solved() {
                self.score.solved(LEVELS.len());
                self.scene = Scene::Solved;
            }
        }
    }
    /// Replay every recorded move except the last one (classic Sokoban undo).
    fn undo(&mut self) {
        if self.scene != Scene::Playing || self.history.is_empty() {
            return;
        }
        self.history.pop();
        let moves = std::mem::take(&mut self.history);
        self.restart();
        for (dr, dc) in moves {
            self.board.push(dr, dc);
            self.history.push((dr, dc));
        }
    }
    fn draw_score(&self, window: &Window) {
        let (moves, pushes) = self.board.stats();
        let line = format!(
            "ROOM {}   MOVES {}   PUSHES {}   SOLVED {}",
            self.score.level + 1,
            moves,
            pushes,
            self.score.best
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
                if self.scene == Scene::Playing {
                    self.scene = Scene::Paused;
                }
                true
            }
            Event::KeyDown => {
                const LEFT: Key = Key::from_char('a');
                const RIGHT: Key = Key::from_char('d');
                const UP: Key = Key::from_char('w');
                const DOWN: Key = Key::from_char('s');
                match event_key() {
                    Key::Escape => match self.scene {
                        Scene::Playing => self.scene = Scene::Paused,
                        Scene::Paused => self.scene = Scene::Playing,
                        Scene::Solved => self.scene = Scene::Welcome,
                        Scene::Welcome => crate::Model::connect(window),
                    },
                    Key::Enter => match self.scene {
                        Scene::Welcome => self.restart(),
                        Scene::Paused => self.scene = Scene::Playing,
                        Scene::Solved => self.restart(),
                        Scene::Playing => {}
                    },
                    Key::BackSpace | Key::from_char('u') => self.undo(),
                    Key::Left | LEFT => self.step(0, -1),
                    Key::Right | RIGHT => self.step(0, 1),
                    Key::Up | UP => self.step(-1, 0),
                    Key::Down | DOWN => self.step(1, 0),
                    _ => return false,
                };
                window.redraw();
                true
            }
            _ => false,
        }
    }
    fn update(&mut self, _dt: f32) {
        // Turn-based game: everything happens in `handle`.
    }
    fn draw(&self, window: &mut Window) {
        draw::draw_rect_fill(0, 0, window.width(), window.height(), Color::Background);
        match self.scene {
            Scene::Welcome => window.draw_welcome(
                "Sokoban",
                &[
                    &["ROOMS SOLVED", &self.score.best.to_string()],
                    &["PRESS ENTER", "for play"],
                    &["PRESS ESC", "for exit"],
                ],
            ),
            Scene::Playing | Scene::Paused | Scene::Solved => {
                self.draw_score(window);
                let (rows, cols) = self.board.dims();
                let (pr, pc) = self.board.player();
                for r in 0..rows {
                    for c in 0..cols {
                        let (x, y, w, h) = self.rect(r, c);
                        if self.board.wall(r, c) {
                            draw::draw_rect_fill(x, y, w, h, Color::Base01);
                            continue;
                        }
                        // Floor tile under everything else.
                        draw::draw_rect_fill(x, y, w, h, Color::Background2);
                        if self.board.goal(r, c) {
                            // Goal marker: a small diamond outline.
                            let d = w / 4;
                            draw::set_draw_color(Color::Yellow);
                            draw::draw_line(x + w / 2, y + d, x + w - d, y + h / 2, Color::Yellow);
                            draw::draw_line(x + w - d, y + h / 2, x + w / 2, y + h - d, Color::Yellow);
                            draw::draw_line(x + w / 2, y + h - d, x + d, y + h / 2, Color::Yellow);
                            draw::draw_line(x + d, y + h / 2, x + w / 2, y + d, Color::Yellow);
                        }
                        if self.board.crate_at(r, c) {
                            let color = if self.board.goal(r, c) {
                                Color::Green
                            } else {
                                Color::Orange
                            };
                            draw::draw_rect_fill(x + w / 8, y + h / 8, w * 3 / 4, h * 3 / 4, color);
                        }
                        if (r, c) == (pr, pc) {
                            draw::set_draw_color(Color::Cyan);
                            draw::draw_circle_fill(
                                x + w / 4,
                                y + h / 4,
                                w / 2,
                                Color::Cyan,
                            );
                        }
                    }
                }
                match self.scene {
                    Scene::Paused => {
                        window.draw_overlay("PAUSED", "PRESS ENTER to resume", Color::Background2);
                    }
                    Scene::Solved => {
                        window.draw_overlay(
                            "ROOM SOLVED!",
                            "PRESS ENTER for the next room",
                            Color::Background2,
                        );
                    }
                    Scene::Playing => {}
                    Scene::Welcome => unreachable!(),
                }
            }
        }
    }
}
