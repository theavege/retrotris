use crate::gui::*;
use crate::models::bird::*;

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
    bird: Bird,
    pipes: Vec<Pipe>,
    score: i32,
    best: i32,
    state: State,
    timer: f32,
    /// One flap input may be buffered while airborne and consumed on the next
    /// update tick, so fast taps during lag are never dropped.
    flap_buffered: bool,
}

impl Model {
    fn scaled(&self) -> Scaled {
        Config::default().scale(self.field)
    }
    fn restart(&mut self) {
        self.bird = Bird::new(self.field.1);
        self.pipes = Vec::new();
        self.score = 0;
        // Fair start: the first pipe spawns only after a full interval,
        // giving the player time to settle into a rhythm.
        self.timer = 0.0;
        self.flap_buffered = false;
        self.state = State::Playing;
    }
    fn die(&mut self) {
        self.state = State::GameOver;
        if self.score > self.best {
            self.best = self.score;
        }
    }
    fn flap(&mut self) {
        match self.state {
            State::Menu | State::GameOver => self.restart(),
            State::Paused => self.state = State::Playing,
            State::Playing => self.flap_buffered = true,
        }
    }
    fn draw_score(&self, window: &Window) {
        let line = format!("SCORE: {}   BEST: {}", self.score, self.best);
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
    fn draw_bird(&self) {
        let s = self.scaled();
        let r = s.bird_radius;
        let x = s.bird_x;
        let y = self.bird.y;
        // Tilt the bird by its vertical velocity: nose up when rising,
        // nose down when falling (classic Floppy Bird feel).
        let tilt = self.bird.tilt(&s);
        let (sin, cos) = (tilt.sin(), tilt.cos());
        let px = |dx: f32, dy: f32| -> (i32, i32) {
            (
                (x + dx * cos - dy * sin) as i32,
                (y + dx * sin + dy * cos) as i32,
            )
        };
        // body
        draw::draw_circle_fill(x as i32, y as i32, (2.0 * r) as i32, Color::Selection);
        // wing (rotates with the body)
        let (wx, wy) = px(-r * 0.35, 0.0);
        draw::draw_circle_fill(wx, wy, (0.9 * r) as i32, Color::Inactive);
        // eye
        let (ex, ey) = px(r * 0.4, -r * 0.35);
        draw::draw_circle_fill(ex, ey, (0.36 * r) as i32, Color::Background);
        // beak
        let (bx, by) = px(r * 0.9, r * 0.1);
        draw::draw_circle_fill(bx, by, (0.5 * r) as i32, Color::Yellow);
    }
    fn draw_pipes(&self) {
        let h = self.field.1 as f32;
        for pipe in &self.pipes {
            let (tx, ty, tw, th) = pipe.top_rect();
            let (bx, by, bw, bh) = pipe.bottom_rect(h);
            // shafts
            draw::draw_rect_fill(tx as i32, ty as i32, tw as i32, th as i32, Color::Green);
            draw::draw_rect_fill(bx as i32, by as i32, bw as i32, bh as i32, Color::Green);
            // caps (distinct lip at each gap edge)
            let cap_h = (tw * 0.3).max(6.0) as i32;
            let over = (tw * 0.1) as i32;
            if th > 0.0 {
                draw::draw_rect_fill(
                    tx as i32 - over,
                    (ty + th) as i32 - cap_h,
                    tw as i32 + 2 * over,
                    cap_h,
                    Color::Cyan,
                );
            }
            if bh > 0.0 {
                draw::draw_rect_fill(
                    bx as i32 - over,
                    by as i32,
                    bw as i32 + 2 * over,
                    cap_h,
                    Color::Cyan,
                );
            }
        }
    }
}

impl Console for Model {
    fn load(&mut self, path: &str) {
        if let Ok(data) = std::fs::read_to_string(path) {
            self.best = data.trim().parse().unwrap_or_default();
        }
    }
    fn exit(&self, path: &str) {
        let _ = std::fs::write(path, self.best.to_string());
    }
    fn handle(&mut self, window: &mut Window, event: Event) -> bool {
        match event {
            Event::Focus => true,
            Event::Resize => {
                self.field = (window.w(), window.h());
                // A mid-game resize changes all pixel-space geometry, so pause
                // instead of letting the bird die against a shifted hitbox.
                if self.state == State::Playing {
                    self.state = State::Paused;
                }
                true
            }
            Event::Push | Event::KeyDown => {
                const SPACE: Key = Key::from_char(' ');
                const W: Key = Key::from_char('w');
                const P: Key = Key::from_char('p');
                let key = match event {
                    Event::Push => None,
                    _ => Some(event_key()),
                };
                match key {
                    None => self.flap(),
                    Some(Key::Escape) => match self.state {
                        State::Playing => self.state = State::Paused,
                        State::Paused => self.state = State::Playing,
                        State::GameOver => self.state = State::Menu,
                        State::Menu => crate::Model::connect(window),
                    },
                    Some(Key::Enter) | Some(Key::Up) | Some(SPACE) | Some(W) => self.flap(),
                    Some(P) => match self.state {
                        State::Playing => self.state = State::Paused,
                        State::Paused => self.state = State::Playing,
                        _ => return false,
                    },
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
        // Clamp dt so a stalled tab / window drag cannot teleport the bird.
        let dt = dt.min(0.1);
        let config = Config::default();
        let s = config.scale(self.field);
        let field_h = self.field.1 as f32;

        if self.flap_buffered {
            self.bird.flap(&s);
            self.flap_buffered = false;
        }
        self.bird.update(dt, &s);

        let speed = s.pipe_speed * speed_multiplier(&config, self.score);
        for pipe in &mut self.pipes {
            pipe.x -= speed * dt;
        }

        self.timer += dt;
        if self.timer >= s.pipe_interval {
            self.timer = 0.0;
            let gap = gap_for_score(&s, &config, field_h, self.score);
            self.pipes.push(Pipe {
                x: self.field.0 as f32,
                width: s.pipe_width,
                gap_center_y: random_gap_center(field_h, gap),
                gap,
                scored: false,
            });
        }

        self.pipes
            .retain(|pipe| pipe.x + pipe.width > -s.pipe_width);

        for pipe in &mut self.pipes {
            if !pipe.scored && s.bird_x > pipe.x + pipe.width {
                self.score += 1;
                pipe.scored = true;
            }
        }

        // Ceiling is a soft wall (bird clamps at the top but survives),
        // the floor and pipes are fatal.
        if self.bird.y - s.bird_radius < 0.0 {
            self.bird.y = s.bird_radius;
            self.bird.velocity = self.bird.velocity.max(0.0);
        }
        if self.bird.y + s.bird_radius >= field_h
            || self
                .pipes
                .iter()
                .any(|pipe| pipe.collides(s.bird_x, self.bird.y, s.bird_radius, field_h))
        {
            self.die();
        }
    }
    fn draw(&self, window: &mut Window) {
        match self.state {
            State::Menu => window.draw_welcome(
                "Bird",
                &[
                    &["BEST SCORE", &self.best.to_string()],
                    &["PRESS ENTER", "for play"],
                    &["PRESS ESC", "for exit"],
                    &["PRESS UP / SPACE / CLICK", "for fly"],
                ],
            ),
            State::Playing | State::Paused => {
                draw::draw_rect_fill(0, 0, window.width(), window.height(), Color::Background);
                self.draw_pipes();
                self.draw_bird();
                self.draw_score(window);
                if self.state == State::Paused {
                    window.draw_overlay("PAUSED", "PRESS ENTER to continue", Color::Magenta);
                }
            }
            State::GameOver => {
                draw::draw_rect_fill(0, 0, window.width(), window.height(), Color::Background);
                self.draw_pipes();
                self.draw_bird();
                window.draw_overlay(
                    "GAME OVER",
                    &format!("SCORE: {}   BEST: {}", self.score, self.best),
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

