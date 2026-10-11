use crate::gui::*;
use crate::models::runner::*;

#[derive(Default)]
pub struct Model {
    scene: Scene,
    difficulty: Difficulty,
    dino: Dino,
    size: (i32, i32),
    run_speed: f32,
    ground_scroll: f32,
    obstacles: Vec<Obstacle>,
    next_gap: f32,
    score: Score,
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
                self.size = (window.w(), window.h());
                match self.scene {
                    Scene::Playing => self.scene = Scene::Paused,
                    Scene::Welcome | Scene::GameOver => {}
                    Scene::Paused => {}
                }
                true
            }
            Event::KeyDown => {
                const SPACE: Key = Key::from_char(' ');
                match event_key() {
                    Key::Escape => match self.scene {
                        Scene::Playing => self.scene = Scene::Paused,
                        Scene::Paused => self.scene = Scene::Playing,
                        Scene::GameOver => self.scene = Scene::Welcome,
                        Scene::Welcome => crate::to_menu(window),
                    },
                    Key::Enter | SPACE => match self.scene {
                        Scene::Playing => self.scene = Scene::Paused,
                        Scene::Paused => self.scene = Scene::Playing,
                        Scene::GameOver => self.setup(),
                        Scene::Welcome => self.setup(),
                    },
                    Key::Up => {
                        if let Scene::Playing = self.scene {
                            self.dino.jump(self.ground_top())
                        }
                    }
                    Key::Down => {
                        if let Scene::Playing = self.scene {
                            self.dino.ducking = true
                        }
                    }
                    Key::Tab => {
                        if let Scene::Welcome = self.scene {
                            self.difficulty = self.difficulty.switch();
                            self.score.set_difficulty(&self.difficulty);
                        }
                    }
                    _ => return false,
                };
                window.redraw();
                true
            }
            Event::KeyUp => {
                match event_key() {
                    Key::Down => {
                        if let Scene::Playing = self.scene {
                            self.dino.ducking = false
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
        if let Scene::Playing = self.scene {
            self.dino.update(dt.min(0.1), self.ground_top());
            // Accelerate gradually, capped per frame for stability.
            self.run_speed *= (1.0 + (SPEED_ACCEL * dt)).min(SPEED_ACCEL_MAX);
            let dx = self.run_speed * dt;
            self.ground_scroll = (self.ground_scroll + dx) % GROUND_TILE;
            self.score.inc(dt);
            self.next_gap -= dx;
            if self.next_gap <= 0.0 {
                let kind = match self.difficulty.ptero_enabled()
                    && self.score.current > PTERO_MIN_SCORE
                    && rand::rng().random_bool(0.35)
                {
                    true => ObstacleKind::Pterodactyl(rand::rng().random_bool(0.5)),
                    false => ObstacleKind::Cactus(rand::rng().random_bool(0.4)),
                };
                let obstacle = Obstacle::spawn(kind, self.ground_y(), self.size.0 as f32);
                self.next_gap = obstacle.next_gap(&self.difficulty);
                self.obstacles.push(obstacle);
            }
            for ob in &mut self.obstacles {
                ob.rect.0 -= dx;
            }
            self.obstacles.retain(|o| o.rect.0 >= PADDING);
            let hitbox = self.dino.hitbox();
            if self
                .obstacles
                .iter()
                .any(|ob| rects_intersect(hitbox, ob.rect))
            {
                self.scene = Scene::GameOver;
            }
        }
    }
    fn draw(&self, window: &mut Window) {
        window.draw_background(Color::Background);
        window.draw_rect(
            PADDING as i32,
            PADDING as i32,
            window.w() - PADDING as i32 * 2,
            window.h() - PADDING as i32 * 2,
            15,
            Color::Background2,
        );
        window.draw_text(
            &format!(
                "SCORE {:05}   BEST {:05}   {}",
                self.score.current,
                self.score.best,
                self.difficulty.label()
            ),
            PADDING as i32,
            2,
            Color::Foreground,
            Align::Left,
            22,
        );
        match self.scene {
            Scene::Welcome => window.draw_welcome(
                "T-Rex Runner",
                &[
                    &["BEST SCORE", &self.score.best.to_string()],
                    &["PRESS <ENTER>", "for play"],
                    &["PRESS <ESC>", "for exit"],
                    &[
                        "PRESS <TAB>",
                        &format!("for level: {}", self.difficulty.label()),
                    ],
                ],
            ),
            Scene::Playing | Scene::Paused => {
                self.draw_game(window);
                if let Scene::Paused = self.scene {
                    window.draw_overlay("PAUSED", "PRESS ENTER to resume", Color::Magenta);
                }
            }
            Scene::GameOver => {
                self.draw_game(window);
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

impl Model {
    fn ground_y(&self) -> f32 {
        (self.size.1 - PADDING as i32) as f32 - GROUND_MARGIN
    }
    fn ground_top(&self) -> f32 {
        self.ground_y() - self.dino.height()
    }
    fn setup(&mut self) {
        self.dino.reset(self.ground_top());
        self.ground_scroll = 0.0;
        self.score.reset();
        self.score.set_difficulty(&self.difficulty);
        self.run_speed = BASE_RUN_SPEED * self.difficulty.speed_mul();
        self.obstacles.clear();
        self.next_gap = self.difficulty.scaled_gap(true);
        self.scene = Scene::Playing;
    }
    fn draw_game(&self, window: &Window) {
        // Ground line with a scrolling dash texture.
        draw::draw_rect_fill(
            PADDING as i32,
            self.ground_y() as i32,
            window.w() - PADDING as i32 * 2,
            4,
            Color::Foreground,
        );
        draw::set_draw_color(Color::Inactive);
        let mut x = PADDING as i32 - self.ground_scroll as i32;
        while x < window.w() - PADDING as i32 {
            draw::draw_rect_fill(x, self.ground_y() as i32 + 10, 8, 2, Color::Inactive);
            x += GROUND_TILE as i32;
        }
        // Dino body + animated foot.
        draw::draw_rect_fill(
            self.dino.x() as i32,
            self.dino.y as i32,
            self.dino.w() as i32,
            self.dino.height() as i32,
            match self.dino.jumping {
                true => Color::Blue,
                false => Color::Green,
            },
        );
        if !self.dino.jumping {
            let (foot_w, foot_h) = Dino::FOOT_SIZE;
            let x = match self.dino.leg_state {
                true => self.dino.x() + Dino::LEG_STEP,
                false => self.dino.x() + self.dino.w() - Dino::LEG_STEP - foot_w,
            };
            draw::draw_rect_fill(
                x as i32,
                (self.dino.y + self.dino.height() - foot_h) as i32,
                foot_w as i32,
                foot_h as i32,
                Color::Blue,
            );
        }
        // Obstacles.
        for obstacle in &self.obstacles {
            let (x, y, w, h) = obstacle.rect;
            draw::draw_rect_fill(
                x as i32,
                y as i32,
                w as i32,
                h as i32,
                match obstacle.kind {
                    ObstacleKind::Cactus(..) => Color::Selection,
                    ObstacleKind::Pterodactyl(..) => Color::Magenta,
                },
            );
            // Wing hint for flying obstacles so they read differently at speed.
            if let ObstacleKind::Pterodactyl(..) = obstacle.kind {
                draw::draw_rect_fill(
                    (x + w / 4.0) as i32,
                    (y - 6.0) as i32,
                    (w / 2.0) as i32,
                    6,
                    Color::Magenta,
                );
            }
        }
    }
}
