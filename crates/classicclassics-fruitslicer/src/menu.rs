//! Retained menus, pause screen and options built with `quasar-ui`.

use crate::game::{Difficulty, Mode, OverReason};
use crate::settings::{Records, Settings};
use quasar_ui::{
    ActionId, Align, Edges, Layer, LayoutItem, Length, MonospaceMeasure, PanelLayout, Rect,
    StackDirection, Theme, Ui, UiEvent, UiInput, UiText, Widget,
};

const WIDTH: f32 = 1280.0;
const HEIGHT: f32 = 720.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Main,
    Pause,
    Options,
    Help,
    GameOver,
}

#[derive(Debug, Clone)]
pub enum MenuAction {
    None,
    Start(Mode),
    Resume,
    Restart,
    QuitToMenu,
    Quit,
    Apply {
        settings: Settings,
        fullscreen: bool,
    },
    Back,
}

#[derive(Debug, Clone)]
pub struct GameOverInfo {
    pub mode: Mode,
    pub score: i64,
    pub best: i64,
    pub new_best: bool,
    pub reason: Option<OverReason>,
}

pub struct Menu {
    ui: Ui,
    screen: Option<Screen>,
    return_screen: Screen,
    settings: Settings,
    fullscreen: bool,
    records: Records,
    game_over: GameOverInfo,
    master: Option<quasar_ui::NodeId>,
    sfx: Option<quasar_ui::NodeId>,
    music: Option<quasar_ui::NodeId>,
    difficulty: Option<quasar_ui::NodeId>,
    fullscreen_node: Option<quasar_ui::NodeId>,
    show_fps: Option<quasar_ui::NodeId>,
    reduced_flash: Option<quasar_ui::NodeId>,
    pub theme: Theme,
}

impl Default for Menu {
    fn default() -> Self {
        Self::new(Settings::default(), false, Records::default())
    }
}

impl Menu {
    pub fn new(settings: Settings, fullscreen: bool, records: Records) -> Self {
        let mut menu = Self {
            ui: Ui::new(theme()),
            screen: None,
            return_screen: Screen::Main,
            settings,
            fullscreen,
            records,
            game_over: GameOverInfo {
                mode: Mode::Classic,
                score: 0,
                best: 0,
                new_best: false,
                reason: None,
            },
            master: None,
            sfx: None,
            music: None,
            difficulty: None,
            fullscreen_node: None,
            show_fps: None,
            reduced_flash: None,
            theme: theme(),
        };
        menu.rebuild();
        menu
    }

    pub fn open_main(&mut self) {
        self.screen = Some(Screen::Main);
        self.rebuild();
    }

    pub fn open_pause(&mut self, mode: Mode) {
        self.game_over.mode = mode;
        self.screen = Some(Screen::Pause);
        self.rebuild();
    }

    pub fn open_game_over(&mut self, info: GameOverInfo) {
        self.game_over = info;
        self.screen = Some(Screen::GameOver);
        self.rebuild();
    }

    pub fn open_help(&mut self) {
        self.return_screen = self.screen.unwrap_or(Screen::Main);
        self.screen = Some(Screen::Help);
        self.rebuild();
    }

    pub fn open_options(&mut self) {
        self.return_screen = self.screen.unwrap_or(Screen::Main);
        self.screen = Some(Screen::Options);
        self.rebuild();
    }

    pub fn close(&mut self) {
        self.screen = None;
        self.ui = Ui::new(theme());
    }

    pub fn is_open(&self) -> bool {
        self.screen.is_some()
    }

    pub fn screen(&self) -> Option<Screen> {
        self.screen
    }

    pub fn ui(&self) -> Option<&Ui> {
        self.is_open().then_some(&self.ui)
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn fullscreen(&self) -> bool {
        self.fullscreen
    }

    pub fn set_records(&mut self, records: Records) {
        self.records = records;
        if self.screen.is_some() {
            self.rebuild();
        }
    }

    pub fn update(&mut self, input: &UiInput, dt: f32) -> Result<MenuAction, String> {
        if !self.is_open() {
            return Ok(MenuAction::None);
        }
        let frame = self.ui.update(
            input,
            Rect::new(0.0, 0.0, WIDTH, HEIGHT),
            1.0,
            Edges::ZERO,
            &MonospaceMeasure,
            dt,
        );
        self.sync_settings();
        let mut action = MenuAction::None;
        for event in &frame.events {
            if matches!(event, UiEvent::Cancelled) {
                action = self.cancel();
                if !matches!(action, MenuAction::None) {
                    break;
                }
                continue;
            }
            let UiEvent::Pressed(id) = event else {
                continue;
            };
            action = match (self.screen, id.0) {
                (Some(Screen::Main), 1) => {
                    self.close();
                    MenuAction::Start(Mode::Classic)
                }
                (Some(Screen::Main), 2) => {
                    self.close();
                    MenuAction::Start(Mode::Arcade)
                }
                (Some(Screen::Main), 3) => {
                    self.close();
                    MenuAction::Start(Mode::Zen)
                }
                (Some(Screen::Main), 4) => {
                    self.open_options();
                    MenuAction::None
                }
                (Some(Screen::Main), 5) => {
                    self.open_help();
                    MenuAction::None
                }
                (Some(Screen::Main), 6) => MenuAction::Quit,
                (Some(Screen::Help), 1) => {
                    self.screen = Some(self.return_screen);
                    self.rebuild();
                    MenuAction::None
                }
                (Some(Screen::Pause), 1) => {
                    self.close();
                    MenuAction::Resume
                }
                (Some(Screen::Pause), 2) => MenuAction::Restart,
                (Some(Screen::Pause), 3) => {
                    self.open_options();
                    MenuAction::None
                }
                (Some(Screen::Pause), 4) => MenuAction::QuitToMenu,
                (Some(Screen::GameOver), 1) => MenuAction::Restart,
                (Some(Screen::GameOver), 2) => MenuAction::QuitToMenu,
                (Some(Screen::GameOver), 3) => MenuAction::Quit,
                (Some(Screen::Options), 1) => {
                    let action = MenuAction::Apply {
                        settings: self.settings.clone(),
                        fullscreen: self.fullscreen,
                    };
                    self.screen = Some(self.return_screen);
                    self.rebuild();
                    action
                }
                (Some(Screen::Options), 2) => {
                    self.settings = Settings::default();
                    self.screen = Some(Screen::Options);
                    self.rebuild();
                    MenuAction::Apply {
                        settings: self.settings.clone(),
                        fullscreen: self.fullscreen,
                    }
                }
                _ => MenuAction::None,
            };
            if !matches!(action, MenuAction::None) {
                break;
            }
        }
        Ok(action)
    }

    fn cancel(&mut self) -> MenuAction {
        match self.screen {
            Some(Screen::Main) => MenuAction::None,
            Some(Screen::Pause) => {
                self.close();
                MenuAction::Resume
            }
            Some(Screen::Options) | Some(Screen::Help) => {
                self.screen = Some(self.return_screen);
                self.rebuild();
                MenuAction::Back
            }
            Some(Screen::GameOver) => MenuAction::None,
            None => MenuAction::None,
        }
    }

    fn sync_settings(&mut self) {
        if let Some(id) = self.master
            && let Some(value) = slider_value(&self.ui, id)
        {
            self.settings.audio.master = value;
        }
        if let Some(id) = self.sfx
            && let Some(value) = slider_value(&self.ui, id)
        {
            self.settings.audio.sfx = value;
        }
        if let Some(id) = self.music
            && let Some(value) = slider_value(&self.ui, id)
        {
            self.settings.audio.music = value;
        }
        if let Some(id) = self.difficulty
            && let Some(node) = self.ui.node(id)
            && let Widget::Select { selected, .. } = &node.widget
        {
            self.settings.gameplay.difficulty = Difficulty::ALL
                .get(*selected)
                .copied()
                .unwrap_or(Difficulty::Normal)
                .key()
                .into();
        }
        if let Some(id) = self.fullscreen_node
            && let Some(node) = self.ui.node(id)
            && let Widget::Checkbox { checked, .. } = &node.widget
        {
            self.fullscreen = *checked;
        }
        if let Some(id) = self.show_fps
            && let Some(node) = self.ui.node(id)
            && let Widget::Checkbox { checked, .. } = &node.widget
        {
            self.settings.gameplay.show_fps = *checked;
        }
        if let Some(id) = self.reduced_flash
            && let Some(node) = self.ui.node(id)
            && let Widget::Checkbox { checked, .. } = &node.widget
        {
            self.settings.gameplay.reduced_flash = *checked;
        }
    }

    fn rebuild(&mut self) {
        self.master = None;
        self.sfx = None;
        self.music = None;
        self.difficulty = None;
        self.fullscreen_node = None;
        self.show_fps = None;
        self.reduced_flash = None;
        let Some(screen) = self.screen else {
            self.ui = Ui::new(theme());
            return;
        };
        let mut ui = Ui::new(theme());
        let root = ui.add_root(
            Widget::Panel {
                layout: PanelLayout::Absolute,
                padding: Edges::ZERO,
                clip: false,
                scrollable: false,
            },
            LayoutItem::fill(),
            Layer::Normal,
            screen == Screen::GameOver,
        );
        let (width, offset) = match screen {
            Screen::Options => (600.0, 10.0),
            Screen::Help => (720.0, 0.0),
            Screen::GameOver => (520.0, 30.0),
            Screen::Main => (520.0, 96.0),
            Screen::Pause => (460.0, 20.0),
        };
        let panel = ui
            .add_child(
                root,
                Widget::Panel {
                    layout: PanelLayout::Stack {
                        direction: StackDirection::Vertical,
                        gap: 8.0,
                    },
                    padding: Edges::all(14.0),
                    clip: false,
                    scrollable: false,
                },
                LayoutItem::size(Length::fixed(width), Length::Auto)
                    .with_align(Align::Center)
                    .with_offset(quasar_ui::Vec2::new(0.0, offset)),
            )
            .expect("root accepts panel");

        let title = match screen {
            Screen::Main => "SELECT A RUN",
            Screen::Pause => "PAUSED",
            Screen::Options => "OPTIONS",
            Screen::Help => "HOW TO PLAY",
            Screen::GameOver => "RUN OVER",
        };
        label(&mut ui, panel, title, quasar_ui::TextStyle::Heading);
        match screen {
            Screen::Main => {
                let best = format!(
                    "CLASSIC {}   ARCADE {}   ZEN {}",
                    self.records.best(Mode::Classic),
                    self.records.best(Mode::Arcade),
                    self.records.best(Mode::Zen)
                );
                label(&mut ui, panel, &best, quasar_ui::TextStyle::Small);
                button(&mut ui, panel, "CLASSIC", 1, 46.0);
                button(&mut ui, panel, "ARCADE", 2, 46.0);
                button(&mut ui, panel, "ZEN", 3, 46.0);
                button(&mut ui, panel, "OPTIONS", 4, 42.0);
                button(&mut ui, panel, "HOW TO PLAY", 5, 42.0);
                button(&mut ui, panel, "QUIT", 6, 42.0);
            }
            Screen::Pause => {
                label(
                    &mut ui,
                    panel,
                    &format!(
                        "{}  /  {}",
                        self.game_over.mode.label(),
                        self.settings.difficulty().label()
                    ),
                    quasar_ui::TextStyle::Small,
                );
                button(&mut ui, panel, "RESUME", 1, 52.0);
                button(&mut ui, panel, "RESTART", 2, 46.0);
                button(&mut ui, panel, "OPTIONS", 3, 46.0);
                button(&mut ui, panel, "QUIT TO MENU", 4, 46.0);
            }
            Screen::GameOver => {
                let reason = match self.game_over.reason {
                    Some(OverReason::Bomb) => "A BOMB ENDED THE RUN",
                    Some(OverReason::Lives) => "NO LIVES LEFT",
                    Some(OverReason::Time) => "TIME UP",
                    None => "RUN COMPLETE",
                };
                label(&mut ui, panel, reason, quasar_ui::TextStyle::Body);
                label(
                    &mut ui,
                    panel,
                    &format!("SCORE {}", self.game_over.score),
                    quasar_ui::TextStyle::Heading,
                );
                if self.game_over.new_best {
                    label(&mut ui, panel, "NEW BEST", quasar_ui::TextStyle::Body);
                } else {
                    label(
                        &mut ui,
                        panel,
                        &format!("BEST {}", self.game_over.best),
                        quasar_ui::TextStyle::Small,
                    );
                }
                button(&mut ui, panel, "RETRY", 1, 54.0);
                button(&mut ui, panel, "CHANGE MODE", 2, 46.0);
                button(&mut ui, panel, "QUIT", 3, 46.0);
            }
            Screen::Help => {
                for line in [
                    "HOLD THE BLADE AND SWIPE THROUGH FRUIT.",
                    "LINK SLICES QUICKLY FOR COMBO BONUSES.",
                    "BOMBS END CLASSIC RUNS AND COST ARCADE POINTS.",
                    "FROST SLOWS TIME. FRENZY DOUBLES POINTS.",
                    "POMEGRANATE PAYS OUT AND SPAWNS A NEW WAVE.",
                    "HEART FRUIT RESTORES A LIFE.",
                    "MOUSE, TOUCHPAD OR GAMEPAD: HOLD SLICE, THEN MOVE.",
                    "P / ESC PAUSE   R RESTART   F11 FULLSCREEN",
                ] {
                    label(&mut ui, panel, line, quasar_ui::TextStyle::Small);
                }
                button(&mut ui, panel, "BACK", 1, 48.0);
            }
            Screen::Options => {
                self.master = Some(slider(
                    &mut ui,
                    panel,
                    "MASTER VOLUME",
                    self.settings.audio.master,
                ));
                self.sfx = Some(slider(
                    &mut ui,
                    panel,
                    "EFFECTS VOLUME",
                    self.settings.audio.sfx,
                ));
                self.music = Some(slider(
                    &mut ui,
                    panel,
                    "MUSIC VOLUME",
                    self.settings.audio.music,
                ));
                self.difficulty = Some(
                    ui.add_child(
                        panel,
                        Widget::Select {
                            label: UiText::literal("DIFFICULTY"),
                            options: Difficulty::ALL
                                .iter()
                                .map(|difficulty| UiText::literal(difficulty.label()))
                                .collect(),
                            selected: Difficulty::ALL
                                .iter()
                                .position(|difficulty| *difficulty == self.settings.difficulty())
                                .unwrap_or(1),
                            action: None,
                        },
                        LayoutItem::size(Length::Fill, Length::fixed(56.0)),
                    )
                    .expect("panel accepts select"),
                );
                self.fullscreen_node =
                    Some(checkbox(&mut ui, panel, "FULLSCREEN", self.fullscreen));
                self.show_fps = Some(checkbox(
                    &mut ui,
                    panel,
                    "SHOW FPS",
                    self.settings.gameplay.show_fps,
                ));
                self.reduced_flash = Some(checkbox(
                    &mut ui,
                    panel,
                    "REDUCE SCREEN FLASH",
                    self.settings.gameplay.reduced_flash,
                ));
                label(
                    &mut ui,
                    panel,
                    "SAVED IN SHARED SETTINGS",
                    quasar_ui::TextStyle::Small,
                );
                button(&mut ui, panel, "APPLY AND BACK", 1, 50.0);
                button(&mut ui, panel, "RESET DEFAULTS", 2, 44.0);
            }
        }
        if screen == Screen::GameOver {
            ui.set_modal(Some(root));
        }
        self.ui = ui;
    }
}

/// Logo placement for the main menu, used by the desktop renderer.
pub fn logo_center() -> [f32; 2] {
    [WIDTH * 0.5, 176.0]
}

pub fn logo_width() -> f32 {
    780.0
}

fn theme() -> Theme {
    let mut theme = Theme::retro();
    theme.name = "dojo".into();
    theme.background = quasar_ui::Color::rgba(8, 7, 10, 235);
    theme.panel = quasar_ui::Color::rgba(24, 18, 20, 246);
    theme.panel_alt = quasar_ui::Color::rgba(34, 26, 28, 138);
    theme.text = quasar_ui::Color::rgb(247, 236, 210);
    theme.dim_text = quasar_ui::Color::rgb(198, 178, 148);
    theme.accent = quasar_ui::Color::rgb(255, 198, 98);
    theme.focus = quasar_ui::Color::rgb(255, 238, 196);
    theme.disabled = quasar_ui::Color::rgb(108, 96, 88);
    theme.button = quasar_ui::Color::rgb(62, 40, 38);
    theme.button_hover = quasar_ui::Color::rgb(96, 58, 48);
    theme.button_active = quasar_ui::Color::rgb(132, 76, 54);
    theme.selection = quasar_ui::Color::rgb(110, 62, 52);
    theme.track = quasar_ui::Color::rgb(18, 14, 15);
    theme.fill = quasar_ui::Color::rgb(255, 178, 74);
    theme.danger = quasar_ui::Color::rgb(244, 94, 82);
    theme.overlay = quasar_ui::Color::rgba(0, 0, 0, 170);
    theme.font_size = 24.0;
    theme.heading_size = 36.0;
    theme.small_size = 16.0;
    theme.row_height = 40.0;
    theme.padding = 8.0;
    theme.gap = 6.0;
    theme.border_width = 2.0;
    theme.focus_width = 3.0;
    theme
}

fn label(ui: &mut Ui, parent: quasar_ui::NodeId, text: &str, style: quasar_ui::TextStyle) {
    let _ = ui.add_child(
        parent,
        Widget::Label {
            text: UiText::literal(text),
            style,
        },
        LayoutItem::size(Length::Fill, Length::Auto),
    );
}

fn button(ui: &mut Ui, parent: quasar_ui::NodeId, text: &str, action: u64, height: f32) {
    let _ = ui.add_child(
        parent,
        Widget::Button {
            text: UiText::literal(text),
            action: Some(ActionId::new(action)),
        },
        LayoutItem::size(Length::Fill, Length::fixed(height)),
    );
}

fn slider(
    ui: &mut Ui,
    parent: quasar_ui::NodeId,
    label_text: &str,
    value: f32,
) -> quasar_ui::NodeId {
    ui.add_child(
        parent,
        Widget::Slider {
            label: UiText::literal(label_text),
            value,
            min: 0.0,
            max: 1.0,
            step: 0.05,
            action: None,
        },
        LayoutItem::size(Length::Fill, Length::fixed(40.0)),
    )
    .expect("panel accepts slider")
}

fn checkbox(
    ui: &mut Ui,
    parent: quasar_ui::NodeId,
    label_text: &str,
    checked: bool,
) -> quasar_ui::NodeId {
    ui.add_child(
        parent,
        Widget::Checkbox {
            label: UiText::literal(label_text),
            checked,
            action: None,
        },
        LayoutItem::size(Length::Fill, Length::fixed(38.0)),
    )
    .expect("panel accepts checkbox")
}

fn slider_value(ui: &Ui, id: quasar_ui::NodeId) -> Option<f32> {
    let node = ui.node(id)?;
    if let Widget::Slider { value, .. } = &node.widget {
        Some(*value)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu() -> Menu {
        Menu::new(Settings::default(), false, Records::default())
    }

    #[test]
    fn every_screen_lays_out_and_draws() {
        use quasar_render_2d::{Canvas, Textures};
        use quasar_ui::UiRenderer;

        let mut menu = menu();
        menu.open_main();
        menu.update(&UiInput::default(), 1.0 / 60.0).unwrap();

        menu.open_pause(Mode::Arcade);
        menu.update(&UiInput::default(), 1.0 / 60.0).unwrap();

        menu.open_options();
        menu.update(&UiInput::default(), 1.0 / 60.0).unwrap();

        menu.open_help();
        menu.update(&UiInput::default(), 1.0 / 60.0).unwrap();

        menu.open_game_over(GameOverInfo {
            mode: Mode::Classic,
            score: 120,
            best: 140,
            new_best: false,
            reason: Some(OverReason::Bomb),
        });
        menu.update(&UiInput::default(), 1.0 / 60.0).unwrap();

        let ui = menu.ui().expect("game-over menu is open");
        let mut canvas = Canvas::new(1280, 720).unwrap();
        let mut renderer = UiRenderer::new();
        renderer.draw(&mut canvas, ui);
        assert!(!canvas.quads().is_empty());
        let textures = Textures::default();
        let pixels = quasar_render_2d::reference::rasterize(&canvas, &textures).unwrap();
        assert_eq!(pixels.len(), 1280 * 720 * 4);
    }

    #[test]
    fn cancel_from_pause_resumes() {
        let mut menu = menu();
        menu.open_pause(Mode::Zen);
        let input = UiInput {
            cancel: true,
            ..UiInput::default()
        };
        let action = menu.update(&input, 1.0 / 60.0).unwrap();
        assert!(matches!(action, MenuAction::Resume));
        assert!(!menu.is_open());
    }
}
