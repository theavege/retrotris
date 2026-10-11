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
        loop {
            let apple @ Apple(x, y) = Apple::new(self.field);
            if !self.snake.body.contains(&(x, y)) {
                self.apple = apple;
                break;
            }
        }
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
                        State::Menu => crate::Model::connect(window),
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
                            draw::draw_rect_fill(x * cell, y * cell, cell, cell, Color::Background2);
                        }
                    }
                }
                // APPLE
                draw::draw_circle_fill(
                    self.apple.0 * cell + cell / 2,
                    self.apple.1 * cell + cell / 2,
                    cell / 2,
                    Color::Red,
                );
                // SNAKE BODY
                for &seg in &self.snake.body[1..] {
                    draw::draw_rect_fill(seg.0 * cell, seg.1 * cell, cell, cell, Color::Cyan);
                }
                // SNAKE HEAD
                draw::draw_rect_fill(
                    self.snake.body[0].0 * cell,
                    self.snake.body[0].1 * cell,
                    cell,
                    cell,
                    Color::Green,
                );
                self.draw_score(window);
                if self.state == State::Paused {
                    window.draw_overlay("PAUSED", "PRESS ENTER to continue", Color::Magenta);
                }
            }
            State::GameOver => {
                window.draw_overlay(
                    "GAME OVER",
                    &format!("SCORE: {}   BEST: {}", self.score.current, self.score.best),
                    Color::Red,
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
