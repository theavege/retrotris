use crate::gui::*;
use crate::models::tetris::*;

/// Solarized-flavoured tetromino palette, indexed by piece kind (O I S Z T J L).
const COLORS: [Color; PIECE_COUNT] = [
    Color::from_hex(0xB58900), // yellow  - O
    Color::Cyan,               // cyan    - I
    Color::Green,              // green   - S
    Color::from_hex(0xCB4B16), // orange  - Z
    Color::from_hex(0x6C71C4), // violet  - T
    Color::Blue,               // blue    - J
    Color::Magenta,            // magenta - L
];
/// Soft-drop points per cell, hard-drop points per cell.
const SOFT_DROP_PTS: i32 = 1;
const HARD_DROP_PTS: i32 = 2;
/// Width of the side panels (HOLD / NEXT / stats) in cells.
const PANEL_CELLS: i32 = 5;

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
    tetris: Tetris,
    score: Score,
    state: State,
    timer: f32,
}

impl Model {
    fn restart(&mut self) {
        self.tetris.reset();
        self.score.reset();
        self.timer = 0.0;
        self.state = State::Playing;
    }
    /// Gravity tick plus line clearing and drop scoring.
    fn tick(&mut self) {
        let before = self.tetris.curr.y;
        match self.tetris.update() {
            None => {
                self.state = State::GameOver;
                return;
            }
            Some(cleared) => self.after_lock(cleared),
        }
        if self.tetris.curr.y > before {
            self.score.current += SOFT_DROP_PTS;
        }
    }
    fn after_lock(&mut self, cleared: Vec<usize>) {
        if self.tetris.dropped {
            self.score.current += HARD_DROP_PTS
                * (FIELD_ROWS as i32 - self.tetris.grid.ghost(&self.tetris.curr).y).max(0);
        }
        if !cleared.is_empty() {
            self.score.add_lines(cleared.len());
        }
    }
    fn draw_hud(&self, window: &Window, cell: i32) {
        let line = format!(
            "SCORE {}   BEST {}   LEVEL {}",
            self.score.current,
            self.score.best,
            self.score.level() + 1
        );
        let (w, h) = draw::measure(&line, false);
        draw::set_font(Font::CourierBold, 16);
        draw::set_draw_color(Color::Foreground);
        draw::draw_text2(&line, window.w() / 2 - w / 2, PAD, w, h, Align::Left);
        let _ = cell;
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
                if self.state == State::Playing {
                    self.state = State::Paused;
                }
                true
            }
            Event::KeyDown => {
                const LEFT: Key = Key::from_char('a');
                const RIGHT: Key = Key::from_char('d');
                const UP: Key = Key::from_char('w');
                const DOWN: Key = Key::from_char('s');
                const SPACE: Key = Key::from_char(' ');
                const C: Key = Key::from_char('c');
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
                            self.tetris.rotate(true)
                        }
                    }
                    Key::Down | DOWN => {
                        if self.state == State::Playing {
                            match self.tetris.soft_drop() {
                                None => self.state = State::GameOver,
                                Some(cleared) => self.after_lock(cleared),
                            }
                        }
                    }
                    Key::Left | LEFT => {
                        if self.state == State::Playing {
                            self.tetris.move_x(-1)
                        }
                    }
                    Key::Right | RIGHT => {
                        if self.state == State::Playing {
                            self.tetris.move_x(1)
                        }
                    }
                    Key::ShiftL | Key::ShiftR => {
                        if self.state == State::Playing {
                            match self.tetris.hard_drop() {
                                None => self.state = State::GameOver,
                                Some(cleared) => self.after_lock(cleared),
                            }
                        }
                    }
                    Key::ControlL | Key::ControlR | C => {
                        if self.state == State::Playing {
                            self.tetris.hold()
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
        if self.timer < self.score.gravity() {
            return;
        }
        self.timer = 0.0;
        self.tick();
    }
    fn draw(&self, window: &mut Window) {
        window.draw_background(Color::Background);
        match self.state {
            State::Menu => window.draw_welcome(
                "Tetris",
                &[
                    &["BEST SCORE", &self.score.best.to_string()],
                    &["PRESS ENTER", "for play"],
                    &["PRESS ESC", "for exit"],
                ],
            ),
            State::Playing | State::Paused => {
                // Cell size chosen so field + side panels fit the window.
                let cell = (window.width() / (FIELD_COLS as i32 + 2 * PANEL_CELLS))
                    .min(window.height() / FIELD_ROWS as i32);
                let field_w = cell * FIELD_COLS as i32;
                let x0 = (window.width() - field_w) / 2;
                let y0 = (window.height() - cell * FIELD_ROWS as i32) / 2;
                // Playfield frame + empty cells
                draw::draw_rect_fill(
                    x0 - 2,
                    y0 - 2,
                    field_w + 4,
                    cell * FIELD_ROWS as i32 + 4,
                    Color::Foreground,
                );
                for y in 0..FIELD_ROWS as i32 {
                    for x in 0..FIELD_COLS as i32 {
                        draw::draw_rect_fill(
                            x0 + x * cell + 1,
                            y0 + y * cell + 1,
                            cell - 2,
                            cell - 2,
                            Color::Background2,
                        );
                    }
                }
                // Ghost piece (outline preview of the landing position)
                let ghost = self.tetris.ghost();
                for (gx, gy) in ghost.cells() {
                    if gy >= 0 {
                        draw::set_draw_color(COLORS[ghost.kind]);
                        draw::draw_rect(x0 + gx * cell + 3, y0 + gy * cell + 3, cell - 6, cell - 6);
                    }
                }
                // Locked cells + active piece
                for (y, row) in self.tetris.grid.draw(&self.tetris.curr).iter().enumerate() {
                    for (x, cell_kind) in row.iter().enumerate() {
                        if let Some(kind) = cell_kind {
                            draw::draw_rect_fill(
                                x0 + x as i32 * cell + 1,
                                y0 + y as i32 * cell + 1,
                                cell - 2,
                                cell - 2,
                                COLORS[*kind],
                            );
                        }
                    }
                }
                self.draw_slot(
                    "HOLD",
                    x0 - cell * (PANEL_CELLS + 1),
                    y0,
                    cell,
                    self.tetris.hold_kind,
                    self.tetris.hold_used,
                );
                self.draw_slot(
                    "NEXT",
                    x0 + field_w + cell,
                    y0,
                    cell,
                    Some(self.tetris.next_kind),
                    false,
                );
                self.draw_hud(window, cell);
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

impl Model {
    /// Small preview panel (HOLD or NEXT) with a title and a centered piece.
    fn draw_slot(&self, title: &str, x: i32, y: i32, cell: i32, kind: Option<usize>, dim: bool) {
        draw::set_font(Font::CourierBold, cell);
        draw::set_draw_color(Color::Foreground);
        draw::draw_text2(
            title,
            x,
            y - cell - 4,
            cell * PANEL_CELLS,
            cell,
            Align::Left,
        );
        let Some(kind) = kind else { return };
        let color = if dim { Color::Inactive } else { COLORS[kind] };
        let cells = SHAPES[kind][0];
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (i32::MAX, 0, i32::MAX, 0);
        for &(cx, cy) in &cells {
            min_x = min_x.min(cx);
            max_x = max_x.max(cx);
            min_y = min_y.min(cy);
            max_y = max_y.max(cy);
        }
        let off_x = x + (cell * PANEL_CELLS - (max_x - min_x + 1) * cell) / 2 - min_x * cell;
        let off_y = y + (cell * PANEL_CELLS - (max_y - min_y + 1) * cell) / 2 - min_y * cell;
        for (cx, cy) in cells {
            draw::draw_rect_fill(
                off_x + cx * cell + 1,
                off_y + cy * cell + 1,
                cell - 2,
                cell - 2,
                color,
            );
        }
    }
}
