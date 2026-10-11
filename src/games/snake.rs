use crate::gui::*;
use crate::models::snake::*;

#[derive(Default, PartialEq)]
enum State {
    #[default]
    Menu,
    Playing,
    Paused,
    GameOver,
}

#[derive(Default)]
pub struct Model {
    field: (i32, i32),
    snake: Snake,
    apple: Apple,
    score: Score,
    state: State,
    timer: f32,
}

impl Model {
    fn restart(&mut self) {
        self.snake.set(self.field);
        self.score.reset();
        self.set_apple();
        self.timer = 0.0;
        self.state = State::Playing;
    }
    fn set_apple(&mut self) {
        self.apple = Apple::new(&self.snake, self.field);
    }
    fn draw_score(&self, window: &Window) {
        let line = format!(
            "SCORE: {}   BEST: {}   LEVEL: {}",
            self.score.current,
            self.score.best,
            self.score.level() + 1
        );
        let (w, h) = draw::measure(&line, false);
        draw::set_font(Font::CourierBold, 16);
        draw::set_draw_color(Color::Foreground);
        draw::draw_text2(
            &line,
            window.w() / 2 - w / 2,
            PAD,
            w,
            h + i32::from(h == 0),
            Align::Left,
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
                self.field = (COLS, ROWS);
                match self.state {
                    State::Playing => self.state = State::Paused,
                    State::Menu | State::GameOver => self.restart(),
                    State::Paused => {}
                }
                true
            }
            Event::KeyDown => {
                const LEFT: Key = Key::from_char('a');
                const RIGHT: Key = Key::from_char('d');
                const UP: Key = Key::from_char('w');
                const DOWN: Key = Key::from_char('s');
                const SPACE: Key = Key::from_char(' ');
                match event_key() {
                    Key::Escape => match self.state {
                        State::Playing => self.state = State::Paused,
                        State::Paused => self.state = State::Playing,
                        State::GameOver => self.state = State::Menu,
                        State::Menu => crate::to_menu(window),
                    },
                    Key::Enter | SPACE => match self.state {
                        State::Menu | State::GameOver => self.restart(),
                        State::Paused => self.state = State::Playing,
                        State::Playing => self.state = State::Paused,
                    },
                    Key::Up | UP => {
                        if self.state == State::Playing {
                            self.snake.direction(0, -1)
                        }
                    }
                    Key::Down | DOWN => {
                        if self.state == State::Playing {
                            self.snake.direction(0, 1)
                        }
                    }
                    Key::Left | LEFT => {
                        if self.state == State::Playing {
                            self.snake.direction(-1, 0)
                        }
                    }
                    Key::Right | RIGHT => {
                        if self.state == State::Playing {
                            self.snake.direction(1, 0)
                        }
                    }
                    _ => return false,
                };
                window.redraw();
                true
            }
            _ => false,
        }
    }
    fn update(&mut self, dt: f32) {
        if self.state != State::Playing {
            return;
        }
        self.timer += dt;
        if self.timer < 1.0 / (3.0 + self.score.level() as f32) {
            return;
        }
        self.timer = 0.0;
        if let Some((x, y)) = self.snake.update(self.field) {
            let Apple(k, l) = self.apple;
            if (x, y) == (k, l) {
                self.set_apple();
                self.score.inc();
            } else {
                self.snake.body.pop();
            }
            self.score.update();
        } else {
            self.state = State::GameOver;
        }
    }
    fn draw(&self, window: &mut Window) {
        // BACKGROUND
        draw::draw_rect_fill(0, 0, window.width(), window.height(), Color::Background);
        match self.state {
            State::Menu => {
                window.draw_welcome(
                    "Snake",
                    &[
                        &["BEST SCORE", &self.score.best.to_string()],
                        &["PRESS ENTER", "for play"],
                        &["PRESS ESC", "for exit"],
                    ],
                );
            }
            State::Playing | State::Paused => {
                // GRID
                let cell = window.width() / COLS;
                for x in 0..COLS {
                    for y in 0..ROWS {
                        if (x + y) % 2 == 0 {
                            draw::draw_rect_fill(
                                x * cell,
                                y * cell,
                                cell,
                                cell,
                                Color::Background2,
                            );
                        }
                    }
                }
                // APPLE (diameter == full cell; +1 radius so odd cell sizes don't leave a gap)
                let field_w = COLS * cell;
                let ax = self.apple.0 * cell + cell / 2 + (window.width() - field_w) / 2;
                let ay = self.apple.1 * cell + cell / 2;
                draw::draw_circle_fill(ax, ay, cell / 2 + 1, sol::RED);
                // highlight + leaf for readability on the light background
                draw::draw_circle_fill(
                    ax - cell / 6,
                    ay - cell / 6,
                    cell / 7,
                    Color::from_hex(0xef6f5a),
                );
                draw::draw_circle_fill(ax + cell / 5, ay - cell / 2, cell / 8, sol::GREEN);
                // SNAKE BODY (dark base02 segments -> strong contrast vs base2/base3 grid)
                for &seg in &self.snake.body[1..] {
                    draw::draw_rect_fill(seg.0 * cell, seg.1 * cell, cell, cell, sol::BASE02);
                }
                // SNAKE HEAD (bright cyan, outlined)
                let hx = self.snake.body[0].0 * cell;
                let hy = self.snake.body[0].1 * cell;
                draw::draw_rect_fill(hx, hy, cell, cell, sol::CYAN);
                draw::set_draw_color(sol::BASE03);
                draw::draw_rect(hx + 1, hy + 1, cell - 2, cell - 2);
                // eyes
                draw::set_draw_color(Color::Background2);
                draw::draw_point(hx + cell / 4, hy + cell / 3);
                draw::draw_point(hx + cell * 3 / 4, hy + cell / 3);
                self.draw_score(window);
                if self.state == State::Paused {
                    window.draw_overlay("PAUSED", "PRESS ENTER to continue", sol::VIOLET);
                }
            }
            State::GameOver => {
                window.draw_overlay(
                    "GAME OVER",
                    &format!("SCORE: {}   BEST: {}", self.score.current, self.score.best),
                    sol::RED,
                );
                window.draw_text(
                    "PRESS ENTER for retry, ESC for menu",
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
