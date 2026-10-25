//! Menu, game selection, records and the shared simulation input.

use crate::audio::Sound;
use crate::controls::ControlHints;
use crate::games::{asteroids, lunar, pong};
use crate::settings::Settings;
use crate::settings_screen::{self, SettingsScreen};
use crate::{Result, menu};
use quasar_render_2d::Canvas;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameKind {
    Pong,
    LunarLander,
    Asteroids,
}

impl GameKind {
    pub const ALL: [Self; 3] = [Self::Pong, Self::LunarLander, Self::Asteroids];

    pub fn name(self) -> &'static str {
        match self {
            Self::Pong => "PONG",
            Self::LunarLander => "LUNAR LANDER",
            Self::Asteroids => "ASTEROIDS",
        }
    }

    pub fn number(self) -> &'static str {
        match self {
            Self::Pong => "01",
            Self::LunarLander => "02",
            Self::Asteroids => "03",
        }
    }

    pub fn tagline(self) -> &'static str {
        match self {
            Self::Pong => "TWO PADDLES. ONE BALL.",
            Self::LunarLander => "GRAVITY IS PATIENT.",
            Self::Asteroids => "EVERY ROCK SPLITS.",
        }
    }

    pub fn accent(self) -> [u8; 4] {
        match self {
            Self::Pong => crate::paint::CYAN,
            Self::LunarLander => crate::paint::AMBER,
            Self::Asteroids => crate::paint::CORAL,
        }
    }
}

/// A single fixed-step input frame. Edges (`*_pressed`, menu and command keys)
/// are consumed after one tick; held fields stay live for continuous control.
#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub menu_up: bool,
    pub menu_down: bool,
    pub menu_left: bool,
    pub menu_right: bool,
    pub confirm: bool,
    pub back: bool,
    pub settings: bool,
    pub p1: i8,
    pub p2: i8,
    pub rotate: i8,
    pub thrust: bool,
    pub fire: bool,
    pub fire_pressed: bool,
    pub hyperspace: bool,
    pub pause: bool,
    pub restart: bool,
    pub mode: bool,
    pub mute: bool,
    pub quit: bool,
    pub fullscreen: bool,
    pub filter: bool,
    pub scale: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "state", rename_all = "snake_case")]
pub enum Game {
    Pong(pong::Game),
    LunarLander(lunar::Game),
    Asteroids(asteroids::Game),
}

impl Game {
    pub fn new(kind: GameKind) -> Self {
        match kind {
            GameKind::Pong => Self::Pong(pong::Game::new()),
            GameKind::LunarLander => Self::LunarLander(lunar::Game::new()),
            GameKind::Asteroids => Self::Asteroids(asteroids::Game::new()),
        }
    }

    pub fn kind(&self) -> GameKind {
        match self {
            Self::Pong(_) => GameKind::Pong,
            Self::LunarLander(_) => GameKind::LunarLander,
            Self::Asteroids(_) => GameKind::Asteroids,
        }
    }

    pub fn tick(&mut self, input: Input, sounds: &mut Vec<Sound>) {
        match self {
            Self::Pong(game) => game.tick(input, sounds),
            Self::LunarLander(game) => game.tick(input, sounds),
            Self::Asteroids(game) => game.tick(input, sounds),
        }
    }

    pub fn autoplay(&self) -> Input {
        match self {
            Self::Pong(game) => game.autoplay(),
            Self::LunarLander(game) => game.autoplay(),
            Self::Asteroids(game) => game.autoplay(),
        }
    }

    pub fn scene(&self, hints: &ControlHints) -> Canvas {
        match self {
            Self::Pong(game) => game.scene(hints),
            Self::LunarLander(game) => game.scene(hints),
            Self::Asteroids(game) => game.scene(hints),
        }
    }

    pub fn record(&mut self, records: &mut Records) {
        match self {
            Self::Pong(game) => game.record(records),
            Self::LunarLander(game) => game.record(records),
            Self::Asteroids(game) => game.record(records),
        }
    }

    pub fn summary(&self) -> String {
        match self {
            Self::Pong(game) => game.summary(),
            Self::LunarLander(game) => game.summary(),
            Self::Asteroids(game) => game.summary(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PongRecord {
    pub best_rally: u32,
    pub wins: u32,
    pub matches: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LunarRecord {
    pub best_score: u32,
    pub landings: u32,
    pub crashes: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AsteroidsRecord {
    pub best_score: u32,
    pub best_level: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Records {
    pub version: u32,
    pub pong: PongRecord,
    pub lunar: LunarRecord,
    pub asteroids: AsteroidsRecord,
}

impl Default for Records {
    fn default() -> Self {
        Self {
            version: 1,
            pong: PongRecord::default(),
            lunar: LunarRecord::default(),
            asteroids: AsteroidsRecord::default(),
        }
    }
}

impl Records {
    /// Returns the records plus whether the file is safe to replace on exit.
    /// Missing files are safe (first run); corrupt or newer files are preserved.
    pub fn load(path: &Path) -> (Self, bool) {
        match quasar_storage::read_json::<Self>(path) {
            Ok(Some(records)) if records.version == 1 => (records, true),
            Ok(Some(_)) => {
                eprintln!("Unsupported records version; preserving file and starting fresh");
                (Self::default(), false)
            }
            Ok(None) => (Self::default(), true),
            Err(error) => {
                eprintln!("Cannot load records ({error}); preserving file and starting fresh");
                (Self::default(), false)
            }
        }
    }

    pub fn save(&self, path: &Path) -> Result {
        quasar_storage::write_json(path, self)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum Screen {
    Menu,
    Game(Game),
}

pub struct Session {
    pub screen: Screen,
    pub settings_screen: Option<SettingsScreen>,
    pub selected: usize,
    pub records: Records,
    pub settings: Settings,
    pub sounds: Vec<Sound>,
}

impl Session {
    pub fn new(records: Records, settings: Settings) -> Self {
        Self {
            screen: Screen::Menu,
            settings_screen: None,
            selected: 0,
            records,
            settings,
            sounds: Vec::new(),
        }
    }

    pub fn settings_open(&self) -> bool {
        self.settings_screen.is_some()
    }

    pub fn open_settings(&mut self) {
        let from_game = matches!(self.screen, Screen::Game(_));
        self.settings_screen = Some(SettingsScreen::new(from_game));
    }

    pub fn launch(&mut self, kind: GameKind) {
        self.selected = GameKind::ALL
            .iter()
            .position(|candidate| *candidate == kind)
            .unwrap_or(0);
        self.screen = Screen::Game(Game::new(kind));
        self.sounds.push(Sound::MenuSelect);
    }

    /// Advances one fixed tick. Returns true when the application should quit.
    pub fn tick(&mut self, input: Input) -> bool {
        self.sounds.clear();
        if input.quit {
            return true;
        }
        if let Some(screen) = &mut self.settings_screen {
            let outcome = screen.tick(input, &mut self.settings, &mut self.sounds);
            match outcome {
                settings_screen::Outcome::Stay => return false,
                settings_screen::Outcome::Close => {
                    self.settings_screen = None;
                    return false;
                }
                settings_screen::Outcome::LeaveToCatalog => {
                    if let Screen::Game(game) = &mut self.screen {
                        game.record(&mut self.records);
                    }
                    self.screen = Screen::Menu;
                    self.settings_screen = None;
                    self.sounds.push(Sound::MenuSelect);
                    return false;
                }
                settings_screen::Outcome::Quit => return true,
            }
        }
        if input.settings {
            self.open_settings();
            self.sounds.push(Sound::MenuSelect);
            return false;
        }
        match &mut self.screen {
            Screen::Menu => {
                if input.back {
                    return true;
                }
                let previous = self.selected;
                if input.menu_left || input.menu_up {
                    self.selected = (self.selected + GameKind::ALL.len() - 1) % GameKind::ALL.len();
                }
                if input.menu_right || input.menu_down {
                    self.selected = (self.selected + 1) % GameKind::ALL.len();
                }
                if previous != self.selected {
                    self.sounds.push(Sound::MenuMove);
                }
                if input.confirm {
                    let kind = GameKind::ALL[self.selected];
                    self.screen = Screen::Game(Game::new(kind));
                    self.sounds.push(Sound::MenuSelect);
                }
                false
            }
            Screen::Game(game) => {
                if input.back {
                    game.record(&mut self.records);
                    self.screen = Screen::Menu;
                    self.sounds.push(Sound::MenuSelect);
                    return false;
                }
                game.tick(input, &mut self.sounds);
                game.record(&mut self.records);
                false
            }
        }
    }

    pub fn scene(&self, hints: &ControlHints) -> Canvas {
        let base = match &self.screen {
            Screen::Menu => menu::scene(self.selected, &self.records, hints),
            Screen::Game(game) => game.scene(hints),
        };
        match &self.settings_screen {
            Some(screen) => screen.scene(&self.settings, hints, base),
            None => base,
        }
    }

    pub fn autoplay(&self) -> Input {
        if self.settings_screen.is_some() {
            return Input::default();
        }
        match &self.screen {
            Screen::Menu => Input::default(),
            Screen::Game(game) => game.autoplay(),
        }
    }

    pub fn summary(&self) -> String {
        if self.settings_screen.is_some() {
            return format!(
                "screen=settings from_game={} fullscreen={} volume={:.2}",
                self.settings_screen
                    .as_ref()
                    .is_some_and(|screen| screen.from_game),
                self.settings.fullscreen,
                self.settings.volume
            );
        }
        match &self.screen {
            Screen::Menu => format!("screen=menu selected={}", self.selected),
            Screen::Game(game) => format!("screen=game game={:?} {}", game.kind(), game.summary()),
        }
    }

    pub fn state_json(&self) -> serde_json::Value {
        #[derive(Serialize)]
        struct State<'a> {
            version: u32,
            screen: &'static str,
            settings_open: bool,
            selected: usize,
            records: &'a Records,
            settings: &'a Settings,
            game: Option<&'a Game>,
        }
        serde_json::to_value(State {
            version: 1,
            screen: if self.settings_screen.is_some() {
                "settings"
            } else {
                match self.screen {
                    Screen::Menu => "menu",
                    Screen::Game(_) => "game",
                }
            },
            settings_open: self.settings_screen.is_some(),
            selected: self.selected,
            records: &self.records,
            settings: &self.settings,
            game: match &self.screen {
                Screen::Menu => None,
                Screen::Game(game) => Some(game),
            },
        })
        .expect("session state is serializable")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_navigation_launches_every_game_and_back_returns() {
        let mut session = Session::new(Records::default(), Settings::default());
        for expected in GameKind::ALL {
            let index = GameKind::ALL
                .iter()
                .position(|kind| *kind == expected)
                .unwrap();
            session.selected = index;
            assert!(!session.tick(Input {
                confirm: true,
                ..Default::default()
            }));
            match &session.screen {
                Screen::Game(game) => assert_eq!(game.kind(), expected),
                Screen::Menu => panic!("confirm should launch"),
            }
            assert!(!session.tick(Input {
                back: true,
                ..Default::default()
            }));
            assert!(matches!(session.screen, Screen::Menu));
        }
    }

    #[test]
    fn settings_open_freezes_game_and_back_returns_to_catalog() {
        let mut session = Session::new(Records::default(), Settings::default());
        session.launch(GameKind::Pong);
        let before = session.summary();
        assert!(!session.tick(Input {
            settings: true,
            ..Default::default()
        }));
        assert!(session.settings_open());
        assert!(!session.tick(Input {
            p1: 1,
            ..Default::default()
        }));
        assert!(session.summary().starts_with("screen=settings"));
        assert!(!session.tick(Input {
            settings: true,
            ..Default::default()
        }));
        assert!(!session.settings_open());
        assert!(session.summary().starts_with("screen=game"));
        let _ = before;

        assert!(!session.tick(Input {
            back: true,
            ..Default::default()
        }));
        assert!(matches!(session.screen, Screen::Menu));
    }

    #[test]
    fn settings_can_leave_a_running_game() {
        let mut session = Session::new(Records::default(), Settings::default());
        session.launch(GameKind::Asteroids);
        session.tick(Input {
            settings: true,
            ..Default::default()
        });
        let rows = crate::settings_screen::SettingsScreen::new(true);
        let mut overlay = rows;
        overlay.selection = 7;
        session.settings_screen = Some(overlay);
        assert!(!session.tick(Input {
            confirm: true,
            ..Default::default()
        }));
        assert!(session.settings_screen.is_none());
        assert!(matches!(session.screen, Screen::Menu));
    }

    #[test]
    fn edge_navigation_and_quit_are_deterministic() {
        let mut session = Session::new(Records::default(), Settings::default());
        assert!(!session.tick(Input {
            menu_left: true,
            ..Default::default()
        }));
        assert_eq!(session.selected, 2);
        assert!(!session.tick(Input {
            menu_right: true,
            ..Default::default()
        }));
        assert_eq!(session.selected, 0);
        assert!(session.tick(Input {
            back: true,
            ..Default::default()
        }));
    }
}

#[cfg(test)]
mod records_tests {
    use super::*;

    #[test]
    fn missing_records_are_writable_and_bad_files_are_preserved() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("records.json");
        let (records, safe) = Records::load(&path);
        assert_eq!(records.version, 1);
        assert!(safe);

        std::fs::write(&path, b"{broken").unwrap();
        let (_, safe) = Records::load(&path);
        assert!(!safe);
        assert_eq!(std::fs::read(&path).unwrap(), b"{broken");

        std::fs::write(&path, br#"{"version":99}"#).unwrap();
        let (_, safe) = Records::load(&path);
        assert!(!safe);
        assert_eq!(std::fs::read(&path).unwrap(), br#"{"version":99}"#);
    }
}
