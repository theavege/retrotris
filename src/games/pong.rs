use crate::gui::*;
use crate::models::pong::*;

#[derive(Default, PartialEq)]
enum State {
    #[default]
    Menu,
    Serving,
    Playing,
    Paused,
    GameOver,
}

const MAX_SCORE: i32 = 11;
const SERVE_TIME: f32 = 0.8;
const SPEED_BASE: f32 = 5.0;
const SPEED_STEP: f32 = 0.7;

#[derive(Default)]
pub struct Model {
    field: (i32, i32),
    player: Paddle,
    cpu: Paddle,
    ball: Ball,
    score: Score,
    state: State,
    timer: f32,
}

impl Model {
    fn restart(&mut self) {
        self.player = Paddle::new(Side::Left, self.field);
        self.cpu = Paddle::new(Side::Right, self.field);
        self.score.reset();
        self.serve();
    }
    fn serve(&mut self) {
        self.ball = Ball::new(self.field, self.speed());
        self.timer = 0.0;
        self.state = State::Serving;
    }
    fn speed(&self) -> f32 {
        SPEED_BASE + self.score.level() as f32 * SPEED_STEP
    }
    /// Simple CPU: tracks the ball vertically with a limited paddle speed so
    /// it stays beatable at higher levels.
    fn cpu_ai(&mut self) {
        let (_, ball_y, _) = self.ball.draw();
        let max_step = HEIGHT / 2 + self.score.level() as i32 * 2;
        let (_, cy, _, ch) = self.cpu.draw();
        let center = cy + ch / 2;
        let target = if ball_y < center {
            cy - max_step
        } else {
            cy + max_step
        };
        self.cpu.set_y(target, self.field.1);
    }
    fn draw_score(&self, window: &Window) {
        let line = format!(
            "YOU {}   CPU {}   BEST {}",
            self.score.left, self.score.right, self.score.best
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
                match self.state {
                    State::Playing | State::Serving => self.state = State::Paused,
                    State::Menu | State::GameOver => self.restart(),
                    State::Paused => {}
                }
                true
            }
            Event::KeyDown => {
                const UP: Key = Key::from_char('w');
                const DOWN: Key = Key::from_char('s');
                match event_key() {
                    Key::Escape => match self.state {
                        State::Playing | State::Serving => self.state = State::Paused,
                        State::Paused => self.state = State::Playing,
                        State::GameOver => self.state = State::Menu,
                        State::Menu => crate::Model::connect(window),
                    },
                    Key::Enter => match self.state {
                        State::Menu | State::GameOver => self.restart(),
                        State::Paused => self.state = State::Playing,
                        _ => {}
                    },
                    Key::Up | UP => {
                        if self.state == State::Playing {
                            self.player.slide(-HEIGHT, self.field.1);
                        }
                    }
                    Key::Down | DOWN => {
                        if self.state == State::Playing {
                            self.player.slide(HEIGHT, self.field.1);
                        }
                    }
                    _ => return false,
                };
                window.redraw();
                true
            }
            Event::Move => {
                draw::set_cursor(Cursor::None);
                if self.state == State::Playing {
                    self.player.center_on(event_coords().1, self.field.1);
                }
                true
            }
            Event::Leave => {
                draw::set_cursor(Cursor::Arrow);
                true
            }
            _ => false,
        }
    }
    fn update(&mut self, dt: f32) {
        match self.state {
            State::Serving => {
                self.timer += dt;
                if self.timer >= SERVE_TIME {
                    self.state = State::Playing;
                }
            }
            State::Playing => {
                self.timer += dt;
                while self.timer >= TICK {
                    self.timer -= TICK;
                    self.cpu_ai();
                    if let Some(side) = self.ball.step(self.field) {
                        // The escaping side loses the point.
                        self.score.inc(match side {
                            Side::Left => Side::Right,
                            Side::Right => Side::Left,
                        });
                        if self.score.left >= MAX_SCORE || self.score.right >= MAX_SCORE {
                            self.state = State::GameOver;
                        } else {
                            self.serve();
                        }
                        break;
                    }
                    let speed = self.speed();
                    self.ball.check_paddle(&self.player, Side::Left, speed);
                    self.ball.check_paddle(&self.cpu, Side::Right, speed);
                }
            }
            _ => {}
        }
    }
    fn draw(&self, window: &mut Window) {
        draw::draw_rect_fill(0, 0, window.width(), window.height(), Color::Background);
        match self.state {
            State::Menu => window.draw_welcome(
                "Pong",
                &[
                    &["BEST SCORE", &self.score.best.to_string()],
                    &["PRESS ENTER", "for play"],
                    &["PRESS ESC", "for exit"],
                ],
            ),
            State::Serving | State::Playing | State::Paused => {
                self.draw_score(window);
                // Center dashed net.
                draw::set_draw_color(Color::Inactive);
                let dash = HEIGHT;
                let mut y = 0;
                while y < window.h() {
                    draw::draw_rect_fill(
                        window.w() / 2 - WIDTH / 6,
                        y,
                        WIDTH / 3,
                        dash,
                        Color::Inactive,
                    );
                    y += dash * 2;
                }
                let player = self.player.draw();
                let cpu = self.cpu.draw();
                let ball = self.ball.draw();
                draw::draw_rect_fill(player.0, player.1, player.2, player.3, Color::Selection);
                draw::draw_rect_fill(cpu.0, cpu.1, cpu.2, cpu.3, Color::Inactive);
                draw::draw_circle_fill(ball.0, ball.1, ball.2, Color::Foreground);
                match self.state {
                    State::Paused => {
                        window.draw_overlay("PAUSED", "PRESS ENTER to resume", Color::Background2)
                    }
                    State::Serving => {
                        window.draw_overlay("READY?", "", Color::Background2);
                    }
                    _ => {}
                }
            }
            State::GameOver => {
                let won = self.score.right < self.score.left;
                window.draw_welcome(
                    if won { "You win!" } else { "Game Over" },
                    &[
                        &[
                            "FINAL SCORE",
                            &format!("{} : {}", self.score.left, self.score.right),
                        ],
                        &["PRESS ENTER", "to retry"],
                        &["PRESS ESC", "for menu"],
                    ],
                );
            }
        }
    }
}
