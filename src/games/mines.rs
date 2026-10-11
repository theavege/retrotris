use crate::gui::*;
use crate::models::mines::*;

#[derive(Default, PartialEq)]
enum Scene {
    #[default]
    Welcome,
    Playing,
    Paused,
    GameOver,
}

/// Board geometry: a fixed 12 x 8 grid of 40 px cells with a one-cell margin
/// around it, centered in the window.
const GRID_COLS: usize = 12;
const GRID_ROWS: usize = 8;
const MINES: usize = 18;
const CELL: i32 = 40;

/// Extra Solarized palette entries (FLTK has no named constants for these).
const BASE2: Color = Color::from_rgb(238, 232, 213); // base2 / background
const BASE3: Color = Color::from_rgb(253, 246, 227); // base3 / background2
const ORANGE: Color = Color::from_rgb(203, 75, 22);

/// Solarized number colors for the classic 1..8 mine counts.
const NUMBERS: [Color; 8] = [
    Color::from_rgb(38, 139, 210), // blue
    Color::from_rgb(42, 161, 152), // cyan
    Color::from_rgb(133, 153, 0),  // green
    Color::from_rgb(203, 75, 22),  // orange
    Color::from_rgb(211, 54, 130), // magenta
    Color::from_rgb(181, 137, 0),  // yellow
    Color::from_rgb(147, 153, 153), // base1
    Color::from_rgb(101, 123, 131), // base01
];

#[derive(Default)]
pub struct Model {
    field: (i32, i32),
    scene: Scene,
    board: Board,
    score: Score,
    timer: f32,
}

impl Model {
    fn restart(&mut self) {
        self.board = Board::new(GRID_ROWS, GRID_COLS, MINES);
        self.score.reset();
        self.timer = 0.0;
        self.scene = Scene::Playing;
    }
    /// Top-left pixel of the board, centered in the current field.
    fn origin(&self) -> (i32, i32) {
        let w = GRID_COLS as i32 * CELL;
        let h = GRID_ROWS as i32 * CELL;
        (
            (self.field.0 - w) / 2,
            PAD * 4 + (self.field.1 - PAD * 4 - h) / 2,
        )
    }
    /// Pixel rectangle of one cell, inset by a 1 px border.
    fn rect(&self, r: usize, c: usize) -> (i32, i32, i32, i32) {
        let (ox, oy) = self.origin();
        (
            ox + c as i32 * CELL + 1,
            oy + r as i32 * CELL + 1,
            CELL - 2,
            CELL - 2,
        )
    }
    /// Map mouse coordinates to a board cell, if the pointer is over the grid.
    fn cell_at(&self, x: i32, y: i32) -> Option<(usize, usize)> {
        let (ox, oy) = self.origin();
        let dx = x - ox;
        let dy = y - oy;
        if dx < 0 || dy < 0 {
            return None;
        }
        let c = (dx / CELL) as usize;
        let r = (dy / CELL) as usize;
        if r < GRID_ROWS && c < GRID_COLS {
            Some((r, c))
        } else {
            None
        }
    }
    fn draw_score(&self, window: &Window) {
        let line = format!(
            "MINES {:02}   TIME {:03}   BEST {:03}",
            self.board.remaining(),
            self.score.time as i32,
            self.score.best,
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
    /// Reveal/flag/chord handling shared by the mouse buttons.
    fn click(&mut self, button: i32, x: i32, y: i32) {
        if self.scene != Scene::Playing {
            return;
        }
        let Some((r, c)) = self.cell_at(x, y) else {
            return;
        };
        let mut rng = rand::rng();
        let changed = match button {
            1 => self.board.open(r, c, &mut rng),
            3 => self.board.toggle_flag(r, c),
            2 => self.board.chord(r, c, &mut rng),
            _ => false,
        };
        if !changed {
            return;
        }
        if self.board.lost {
            self.scene = Scene::GameOver;
        } else if self.board.won {
            self.score.finish();
            self.scene = Scene::GameOver;
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
                if self.scene == Scene::Playing {
                    self.scene = Scene::Paused;
                }
                true
            }
            Event::Push => {
                let (x, y) = event_coords();
                self.click(event_button(), x as i32, y as i32);
                window.redraw();
                true
            }
            Event::KeyDown => {
                match event_key() {
                    Key::Escape => match self.scene {
                        Scene::Playing => self.scene = Scene::Paused,
                        Scene::Paused => self.scene = Scene::Playing,
                        Scene::GameOver => self.scene = Scene::Welcome,
                        Scene::Welcome => crate::Game::to_menu(window),
                    },
                    Key::Enter => match self.scene {
                        Scene::Welcome | Scene::GameOver => self.restart(),
                        Scene::Paused => self.scene = Scene::Playing,
                        Scene::Playing => {}
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
        if self.scene != Scene::Playing {
            return;
        }
        self.timer += dt;
        while self.timer >= TICK {
            self.timer -= TICK;
            self.score.tick(TICK);
        }
    }
    fn draw(&self, window: &mut Window) {
        draw::draw_rect_fill(0, 0, window.width(), window.height(), Color::Background);
        match self.scene {
            Scene::Welcome => window.draw_welcome(
                "Mines",
                &[
                    &["BEST TIME", &format!("{:03} s", self.score.best)],
                    &["PRESS ENTER", "for play"],
                    &["PRESS ESC", "for exit"],
                ],
            ),
            Scene::Playing | Scene::Paused => {
                self.draw_score(window);
                for (r, c) in self.board.coords() {
                    let (x, y, w, h) = self.rect(r, c);
                    let cell = self.board.cell(r, c);
                    if cell.open {
                        // Opened: flat background2 tile with its count or mine.
                        draw::set_draw_color(Color::Background2);
                        draw::draw_rect_fill(x, y, w, h, Color::Background2);
                        if cell.mine {
                            draw::set_draw_color(Color::Red);
                            draw::draw_circle_fill(x + w / 4, y + h / 4, w / 2, Color::Red);
                        } else if cell.adj > 0 {
                            let n = cell.adj as usize;
                            draw::set_font(Font::HelveticaBold, CELL * 3 / 4);
                            draw::set_draw_color(NUMBERS[n - 1]);
                            let (tw, th) = draw::measure(&n.to_string(), false);
                            draw::draw_text2(
                                &n.to_string(),
                                x + w / 2 - tw / 2,
                                y + h / 2 - th / 2,
                                tw,
                                th,
                                Align::Center,
                            );
                        }
                    } else {
                        // Closed: raised base2 tile, flags drawn on top.
                        draw::set_draw_color(BASE2);
                        draw::draw_rect_fill(x, y, w, h, BASE2);
                        draw::set_draw_color(BASE3);
                        draw::set_draw_color(BASE3);
                        draw::draw_line(x, y + h, x + w, y + h);
                        draw::set_draw_color(BASE3);
                        draw::draw_line(x + w, y, x + w, y + h);
                        if cell.flag {
                            draw::set_draw_color(ORANGE);
                            draw::draw_rect_fill(x + w / 3, y + h / 4, w / 3, h / 2, ORANGE);
                        }
                    }
                }
                if self.scene == Scene::Paused {
                    window.draw_overlay("PAUSED", "PRESS ENTER to resume", Color::Background2);
                }
            }
            Scene::GameOver => {
                let title = if self.board.won { "You Win!" } else { "Game Over" };
                let secs = self.score.time.round() as i32;
                window.draw_welcome(
                    title,
                    &[
                        &["TIME", &format!("{secs:03} s")],
                        &["BEST", &format!("{:03} s", self.score.best)],
                        &["PRESS ENTER", "to retry"],
                        &["PRESS ESC", "for menu"],
                    ],
                );
            }
        }
    }
}
