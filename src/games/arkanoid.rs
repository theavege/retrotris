use crate::gui::*;
use crate::models::arkanoid::*;

#[derive(Default, PartialEq)]
enum State {
    #[default]
    Menu,
    Serving,
    Playing,
    Paused,
    GameOver,
}

const SPEED_BASE: f32 = 4.0;
const SPEED_STEP: f32 = 0.8;

#[derive(Default)]
pub struct Model {
    field: (i32, i32),
    paddle: Paddle,
    ball: Ball,
    wall: Wall,
    score: Score,
    state: State,
    timer: f32,
}

impl Model {
    fn restart(&mut self) {
        self.paddle = Paddle::new(self.field);
        self.wall = Wall::new();
        self.score.reset();
        self.serve();
    }
    /// Put the ball back on the paddle and wait for a launch.
    fn serve(&mut self) {
        self.ball = Ball::attach(&self.paddle);
        self.timer = 0.0;
        self.state = State::Serving;
    }
    fn speed(&self) -> f32 {
        SPEED_BASE + self.score.level() as f32 * SPEED_STEP
    }
    fn draw_score(&self, window: &Window) {
        let line = format!(
            "SCORE {}   BEST {}   LEVEL {}   LIVES {}",
            self.score.points,
            self.score.best,
            self.score.level(),
            self.score.lives
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
                const LEFT: Key = Key::from_char('a');
                const RIGHT: Key = Key::from_char('d');
                const SPACE: Key = Key::from_char(' ');
                match event_key() {
                    Key::Escape => match self.state {
                        State::Playing | State::Serving => self.state = State::Paused,
                        State::Paused => self.state = State::Playing,
                        State::GameOver => self.state = State::Menu,
                        State::Menu => crate::to_menu(window),
                    },
                    Key::Enter | SPACE => match self.state {
                        State::Menu | State::GameOver => self.restart(),
                        State::Serving => {
                            self.ball.launch(self.speed());
                            self.state = State::Playing;
                        }
                        State::Paused => self.state = State::Playing,
                        _ => {}
                    },
                    Key::Left | LEFT => {
                        if self.state == State::Playing {
                            self.paddle.slide(-WIDTH as f32, self.field.0);
                        }
                    }
                    Key::Right | RIGHT => {
                        if self.state == State::Playing {
                            self.paddle.slide(WIDTH as f32, self.field.0);
                        }
                    }
                    _ => return false,
                };
                window.redraw();
                true
            }
            Event::Move => {
                draw::set_cursor(Cursor::None);
                if self.state == State::Playing || self.state == State::Serving {
                    self.paddle.center_on(event_coords().0 as f32, self.field.0);
                    if !self.ball.moving() {
                        // Keep the resting ball glued to the paddle.
                        self.ball = Ball::attach(&self.paddle);
                    }
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
                // The ball follows the paddle until it is launched.
                if !self.ball.moving() {
                    self.ball = Ball::attach(&self.paddle);
                }
            }
            State::Playing => {
                self.timer += dt;
                while self.timer >= TICK {
                    self.timer -= TICK;
                    // Brick collision: check the ball's leading edge point.
                    let (bx, by, size) = self.ball.draw();
                    let lead = if self.ball.vel_y() < 0.0 {
                        by
                    } else {
                        by + size
                    };
                    if let Some((col, row)) = self.wall.hit_at(bx + size / 2.0, lead, self.field) {
                        let health = self.wall.cell(col, row);
                        self.wall.damage(col, row);
                        self.score.brick(health);
                        self.ball.bounce_brick();
                    }
                    self.ball.check_paddle(&self.paddle, self.speed());
                    if self.ball.step(self.field).is_some() {
                        self.score.lose_life();
                        if self.score.lives <= 0 {
                            self.state = State::GameOver;
                        } else {
                            self.serve();
                        }
                        break;
                    }
                    if !self.wall.alive() {
                        // Wall cleared: rebuild it faster and keep playing.
                        self.wall = Wall::new();
                        self.serve();
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    fn draw(&self, window: &mut Window) {
        draw::draw_rect_fill(0, 0, window.width(), window.height(), Color::Background);
        match self.state {
            State::Menu => window.draw_welcome(
                "Arkanoid",
                &[
                    &["BEST SCORE", &self.score.best.to_string()],
                    &["PRESS ENTER", "for play"],
                    &["PRESS ESC", "for exit"],
                ],
            ),
            State::Serving | State::Playing | State::Paused => {
                self.draw_score(window);
                // Bricks: tough cells use the selection color, fresh ones cyan.
                for row in 0..Wall::ROWS {
                    for col in 0..Wall::COLS {
                        let health = self.wall.cell(col, row);
                        if health == 0 {
                            continue;
                        }
                        let (x, y, w, h) = self.wall.rect(col, row, self.field);
                        let color = match health {
                            2 => Color::Selection,
                            _ => Color::Cyan,
                        };
                        draw::draw_rect_fill(x as i32, y as i32, w as i32, h as i32, color);
                    }
                }
                let paddle = self.paddle.draw();
                draw::draw_rect_fill(
                    paddle.0 as i32,
                    paddle.1 as i32,
                    paddle.2 as i32,
                    paddle.3 as i32,
                    Color::Foreground,
                );
                let ball = self.ball.draw();
                draw::draw_circle_fill(
                    ball.0 as i32,
                    ball.1 as i32,
                    ball.2 as i32,
                    Color::Inactive,
                );
                match self.state {
                    State::Paused => {
                        window.draw_overlay("PAUSED", "PRESS ENTER to resume", Color::Background2)
                    }
                    State::Serving => {
                        window.draw_overlay("READY?", "PRESS SPACE to launch", Color::Background2);
                    }
                    _ => {}
                }
            }
            State::GameOver => {
                window.draw_welcome(
                    "Game Over",
                    &[
                        &["FINAL SCORE", &self.score.points.to_string()],
                        &["PRESS ENTER", "to retry"],
                        &["PRESS ESC", "for menu"],
                    ],
                );
            }
        }
    }
}
