//! Menu, game selection, records and the shared simulation input.

use crate::audio::Sound;
use crate::controls::Controls;
use crate::controls_screen::{CaptureEvent, ControlsScreen};
use crate::menu;
use crate::settings::Settings;
use crate::settings_screen::{self, SettingsScreen};
use quasar_render_2d::Canvas;
use serde::Serialize;

pub use crate::games::{Game, GameKind, Records};
pub use classicclassics_core::Input;

#[derive(Clone, Debug)]
pub enum Screen {
    Menu,
    Game(Game),
}

pub struct Session {
    pub screen: Screen,
    pub settings_screen: Option<SettingsScreen>,
    pub controls: Controls,
    pub controls_screen: Option<ControlsScreen>,
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
            controls: Controls::default(),
            controls_screen: None,
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

    pub fn open_controls(&mut self) {
        if !self.settings_open() {
            self.open_settings();
        }
        self.controls_screen = Some(ControlsScreen::new(&self.controls));
    }

    pub fn capturing_controls(&self) -> bool {
        self.controls_screen
            .as_ref()
            .is_some_and(ControlsScreen::capturing)
    }

    pub fn capture_control(&mut self, event: CaptureEvent) {
        if let Some(screen) = &mut self.controls_screen {
            screen.capture(event, &mut self.controls);
        }
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
        if let Some(screen) = &mut self.controls_screen {
            if screen.tick(input, &mut self.controls) {
                self.controls_screen = None;
            }
            return false;
        }
        if input.fullscreen {
            self.settings.fullscreen = !self.settings.fullscreen;
        }
        if input.filter {
            self.settings.linear = !self.settings.linear;
        }
        if input.scale {
            self.settings.integer = !self.settings.integer;
        }
        if input.mute {
            self.settings.muted = !self.settings.muted;
        }
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
                settings_screen::Outcome::Controls => {
                    self.controls_screen = Some(ControlsScreen::new(&self.controls));
                    return false;
                }
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
                self.selected = menu::navigate(self.selected, input, GameKind::ALL.len());
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

    pub fn canvas_size(&self) -> (u32, u32) {
        match self.screen {
            Screen::Menu => (menu::WIDTH, menu::HEIGHT),
            Screen::Game(_) => (crate::WIDTH, crate::HEIGHT),
        }
    }

    pub fn scene(&mut self) -> Canvas {
        let hints = self.controls.hints();
        let base = match &self.screen {
            Screen::Menu => menu::scene(self.selected, &self.records, &hints),
            Screen::Game(game) => game.scene(&hints),
        };
        if let Some(screen) = &mut self.controls_screen {
            return screen.scene(base);
        }
        match &self.settings_screen {
            Some(screen) => screen.scene(&self.settings, &hints, base),
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
        if self.controls_screen.is_some() {
            return format!("screen=controls capturing={}", self.capturing_controls());
        }
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
            controls_open: bool,
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
            controls_open: self.controls_screen.is_some(),
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
        overlay.selection = 8;
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
