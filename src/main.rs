mod games;
mod gui;
mod models;

use gui::*;

#[derive(Default)]
pub enum Game {
    #[default]
    Tetris,
    Snake,
    Bird,
    Pong,
    Runner,
    Arkanoid,
    Invaders,
    Mines,
    Sokoban,
    Asteroids,
}

/// Saves the live game state's record and reconnects the window handlers to
/// the menu model. Used by every game's "back to menu" action.
pub fn to_menu(window: &mut Window) {
    if let Some(saver) = CURRENT.take() {
        saver(&current_path(window));
    }
    Model::connect(window);
}

impl Game {
    fn switch(&self) -> Self {
        match self {
            Self::Tetris => Self::Snake,
            Self::Snake => Self::Bird,
            Self::Bird => Self::Pong,
            Self::Pong => Self::Runner,
            Self::Runner => Self::Arkanoid,
            Self::Arkanoid => Self::Invaders,
            Self::Invaders => Self::Mines,
            Self::Mines => Self::Sokoban,
            Self::Sokoban => Self::Asteroids,
            Self::Asteroids => Self::Tetris,
        }
    }
    fn to_str(&self) -> &str {
        match self {
            Self::Tetris => "Tetris",
            Self::Snake => "Snake",
            Self::Bird => "Bird",
            Self::Pong => "Pong",
            Self::Runner => "Runner",
            Self::Arkanoid => "Arkanoid",
            Self::Invaders => "Invaders",
            Self::Mines => "Minesweeper",
            Self::Sokoban => "Sokoban",
            Self::Asteroids => "Asteroids",
        }
    }
    fn to_play(&self, window: &mut Window) {
        match self {
            Self::Tetris => games::tetris::Model::connect(window),
            Self::Snake => games::snake::Model::connect(window),
            Self::Bird => games::bird::Model::connect(window),
            Self::Pong => games::pong::Model::connect(window),
            Self::Runner => games::runner::Model::connect(window),
            Self::Arkanoid => games::arkanoid::Model::connect(window),
            Self::Invaders => games::invaders::Model::connect(window),
            Self::Mines => games::mines::Model::connect(window),
            Self::Sokoban => games::sokoban::Model::connect(window),
            Self::Asteroids => games::asteroid::Model::connect(window),
        }
    }
}

#[derive(Default)]
pub struct Model(Game);

thread_local! {
    /// Saves the record of the game currently connected to the window; set by
    /// `Console::connect` and consumed when returning to the menu.
    static CURRENT: std::cell::Cell<Option<Box<dyn Fn(&str)>>> =
        const { std::cell::Cell::new(None) };
}

/// Path of the save file for the game shown in `window` (see `Console::connect`).
fn current_path(window: &Window) -> String {
    format!(
        "{}/.{}",
        std::env::var(match cfg!(target_os = "windows") {
            true => "HOMEPATH",
            false => "HOME",
        })
        .unwrap(),
        window.xclass().unwrap(),
    )
}

impl Console for Model {
    fn load(&mut self, _path: &str) {}
    fn exit(&self, _path: &str) {}
    fn update(&mut self, _dt: f32) {}
    fn handle(&mut self, window: &mut Window, event: Event) -> bool {
        match event {
            Event::Focus => true,
            Event::KeyDown => {
                match event_key() {
                    Key::Tab => self.0 = self.0.switch(),
                    Key::Enter => {
                        window.set_xclass(self.0.to_str());
                        self.0.to_play(window);
                    }
                    Key::Escape => std::process::exit(0),
                    _ => return false,
                }
                window.redraw();
                true
            }
            _ => false,
        }
    }
    fn draw(&self, window: &mut Window) {
        window.draw_background(Color::Background);
        window.draw_welcome(
            "Games",
            &[
                &["GAMES:", self.0.to_str()],
                &["PRESS TAB", "for switch"],
                &["PRESS ENTER", "for play"],
                &["PRESS ESC", "for exit"],
            ],
        );
    }
}

fn main() -> Result<(), FltkError> {
    Model::run(Settings {
        fullscreen: true,
        size: Some((SCREEN_WIDTH, SCREEN_HEIGHT)),
        icon: Some(SvgImage::from_data(include_str!("../assets/logo.svg")).unwrap()),
        ..Default::default()
    })
}
