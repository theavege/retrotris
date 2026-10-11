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
    /// Simple CPU: tracks the row where the ball is *predicted* to cross the
    /// goal line (mirrored off top/bottom walls) instead of its current row,
    /// which stops the paddle from jittering when it overshoots the target.
    fn cpu_ai(&mut self) {
        let field_h = self.field.1.max(1);
        let (ball_x, ball_y, size) = self.ball.draw();
        let center = size as f32 / 2.0;
        // Simulate the ball forward until it reaches the CPU's column.
        let mut px = ball_x as f32;
        let mut py = ball_y as f32 + center;
        let mut vy = self.ball.vy();
        if self.ball.moving_right() {
            let limit = field_h as f32 - size as f32;
            while px < self.cpu.x() as f32 {
                px += self.ball.vx();
                py += vy;
                if py <= center {
                    py = center + (center - py);
                    vy = vy.abs();
                } else if py >= limit + center {
                    py = 2.0 * (limit + center) - py;
                    vy = -vy.abs();
                }
            }
        } else {
            // Ball moving away: drift back toward the field center.
            py = field_h as f32 / 2.0;
        }
        let (_, cy, _, ch) = self.cpu.draw();
        let cur = (cy + ch / 2) as f32;
        // Proportional chase with a dead zone: fast when far, never overshoots.
        let err = py - cur;
        let max_step = HEIGHT / 2 + self.score.level() as i32 * 2;
        let step = (err * 0.4).clamp(-max_step as f32, max_step as f32) as i32;
        if step != 0 {
            self.cpu.slide(step, self.field.1);
        }
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
                        State::Menu => crate::to_menu(window),
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
                let dash = HEIGHT;
                let mut y = 0;
                while y < window.h() {
                    draw::draw_rect_fill(
                        window.w() / 2 - WIDTH / 6,
                        y,
                        WIDTH / 3,
                        dash,
                        sol::BASE1,
                    );
                    y += dash * 2;
                }
                let player = self.player.draw();
                let cpu = self.cpu.draw();
                let ball = self.ball.draw();
                // Player paddle: orange with dark outline; CPU: violet; ball: base03.
                for (rect, fill) in [(player, sol::ORANGE), (cpu, sol::VIOLET)] {
                    draw::draw_rect_fill(rect.0, rect.1, rect.2, rect.3, fill);
                    draw::set_draw_color(sol::BASE03);
                    draw::draw_rect(rect.0, rect.1, rect.2, rect.3);
                }
                draw::draw_circle_fill(ball.0, ball.1, ball.2, sol::BASE03);
                match self.state {
                    State::Paused => {
                        window.draw_overlay("PAUSED", "PRESS ENTER to resume", sol::VIOLET)
                    }
                    State::Serving => {
                        window.draw_overlay("READY?", "", sol::BLUE);
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
