//! In-game settings overlay. It owns only presentation and input; the host
//! observes `Session::settings` and applies fullscreen, scaling and audio changes.

use crate::audio::Sound;
use crate::controls::ControlHints;
use crate::paint::{AMBER, CORAL, CYAN, CYAN_DIM, DIM, GREEN, GRID, MUTED, Paint, WHITE};
use crate::session::Input;
use crate::settings::Settings;
use crate::{HEIGHT, WIDTH};
use quasar_render_2d::Canvas;

const PANEL_X: f32 = 186.0;
const PANEL_Y: f32 = 56.0;
const PANEL_WIDTH: f32 = 588.0;
const PANEL_HEIGHT: f32 = 446.0;
const ROW_HEIGHT: f32 = 31.0;
const ROWS_TOP: f32 = 150.0;

const WINDOW_PRESETS: [(u32, u32); 5] = [
    (800, 500),
    (960, 600),
    (1280, 800),
    (1440, 900),
    (1600, 1000),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    Resume,
    Fullscreen,
    Window,
    Filtering,
    Scaling,
    Volume,
    Mute,
    Catalog,
    Reset,
    Quit,
}

fn rows(from_game: bool) -> Vec<Row> {
    let mut rows = vec![
        Row::Resume,
        Row::Fullscreen,
        Row::Window,
        Row::Filtering,
        Row::Scaling,
        Row::Volume,
        Row::Mute,
    ];
    if from_game {
        rows.push(Row::Catalog);
    }
    rows.push(Row::Reset);
    rows.push(Row::Quit);
    rows
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Stay,
    Close,
    LeaveToCatalog,
    Quit,
}

#[derive(Clone, Debug)]
pub struct SettingsScreen {
    pub selection: usize,
    pub from_game: bool,
}

impl SettingsScreen {
    pub fn new(from_game: bool) -> Self {
        Self {
            selection: 0,
            from_game,
        }
    }

    pub fn tick(
        &mut self,
        input: Input,
        settings: &mut Settings,
        sounds: &mut Vec<Sound>,
    ) -> Outcome {
        let rows = rows(self.from_game);
        if self.selection >= rows.len() {
            self.selection = rows.len().saturating_sub(1);
        }
        if input.settings || input.back {
            sounds.push(Sound::MenuSelect);
            return Outcome::Close;
        }
        if input.menu_up {
            self.selection = (self.selection + rows.len() - 1) % rows.len();
            sounds.push(Sound::MenuMove);
        }
        if input.menu_down {
            self.selection = (self.selection + 1) % rows.len();
            sounds.push(Sound::MenuMove);
        }
        let direction = i8::from(input.menu_right) - i8::from(input.menu_left);
        let row = rows[self.selection];
        let mut changed = false;
        match row {
            Row::Resume => {
                if input.confirm {
                    sounds.push(Sound::MenuSelect);
                    return Outcome::Close;
                }
            }
            Row::Fullscreen => {
                if direction != 0 || input.confirm {
                    settings.fullscreen = !settings.fullscreen;
                    changed = true;
                }
            }
            Row::Window => {
                if direction != 0 {
                    let current = WINDOW_PRESETS
                        .iter()
                        .position(|preset| {
                            preset.0 == settings.width && preset.1 == settings.height
                        })
                        .unwrap_or(1);
                    let next = (current as isize + direction as isize)
                        .rem_euclid(WINDOW_PRESETS.len() as isize)
                        as usize;
                    (settings.width, settings.height) = WINDOW_PRESETS[next];
                    changed = true;
                }
            }
            Row::Filtering => {
                if direction != 0 || input.confirm {
                    settings.linear = !settings.linear;
                    changed = true;
                }
            }
            Row::Scaling => {
                if direction != 0 || input.confirm {
                    settings.integer = !settings.integer;
                    changed = true;
                }
            }
            Row::Volume => {
                if direction != 0 {
                    settings.volume = (settings.volume + direction as f32 * 0.05).clamp(0.0, 1.0);
                    changed = true;
                }
            }
            Row::Mute => {
                if direction != 0 || input.confirm {
                    settings.muted = !settings.muted;
                    changed = true;
                }
            }
            Row::Catalog => {
                if input.confirm {
                    sounds.push(Sound::MenuSelect);
                    return Outcome::LeaveToCatalog;
                }
            }
            Row::Reset => {
                if input.confirm {
                    *settings = Settings::default();
                    changed = true;
                }
            }
            Row::Quit => {
                if input.confirm {
                    sounds.push(Sound::MenuSelect);
                    return Outcome::Quit;
                }
            }
        }
        if changed {
            sounds.push(if input.confirm {
                Sound::MenuSelect
            } else {
                Sound::MenuMove
            });
        }
        Outcome::Stay
    }

    pub fn scene(&self, settings: &Settings, hints: &ControlHints, base: Canvas) -> Canvas {
        let mut paint = Paint::over(base);
        paint.rect(0.0, 0.0, WIDTH as f32, HEIGHT as f32, [3, 5, 10, 255]);
        paint.rect(
            PANEL_X,
            PANEL_Y,
            PANEL_WIDTH,
            PANEL_HEIGHT,
            [9, 14, 23, 255],
        );
        paint.outline(PANEL_X, PANEL_Y, PANEL_WIDTH, PANEL_HEIGHT, GRID);
        paint.rect(PANEL_X, PANEL_Y, 92.0, 3.0, CYAN);
        paint.rect(
            PANEL_X + 20.0,
            PANEL_Y + 22.0,
            44.0,
            44.0,
            [13, 22, 33, 255],
        );
        paint.outline(PANEL_X + 20.0, PANEL_Y + 22.0, 44.0, 44.0, CYAN_DIM);
        gear(&mut paint, PANEL_X + 42.0, PANEL_Y + 44.0, CYAN);
        paint.text(PANEL_X + 82.0, PANEL_Y + 22.0, "SETTINGS", 3.0, WHITE);
        paint.text(
            PANEL_X + 83.0,
            PANEL_Y + 52.0,
            "QUASAR SYSTEM CONFIGURATION",
            1.0,
            MUTED,
        );
        paint.right(
            PANEL_X + PANEL_WIDTH - 20.0,
            PANEL_Y + 30.0,
            if self.from_game { "PAUSED" } else { "CATALOG" },
            1.0,
            if self.from_game { AMBER } else { CYAN },
        );

        let rows = rows(self.from_game);
        for (index, row) in rows.iter().copied().enumerate() {
            let y = ROWS_TOP + index as f32 * ROW_HEIGHT;
            let selected = index == self.selection;
            if selected {
                paint.rect(
                    PANEL_X + 20.0,
                    y - 5.0,
                    PANEL_WIDTH - 40.0,
                    ROW_HEIGHT - 2.0,
                    [15, 27, 40, 255],
                );
                paint.outline(
                    PANEL_X + 20.0,
                    y - 5.0,
                    PANEL_WIDTH - 40.0,
                    ROW_HEIGHT - 2.0,
                    CYAN_DIM,
                );
                paint.rect(PANEL_X + 20.0, y - 5.0, 4.0, ROW_HEIGHT - 2.0, CYAN);
            }
            if matches!(row, Row::Catalog | Row::Reset) {
                paint.rect(PANEL_X + 34.0, y - 8.0, PANEL_WIDTH - 68.0, 1.0, GRID);
            }
            paint.text(
                PANEL_X + 40.0,
                y,
                row.label(),
                1.0,
                if selected { WHITE } else { MUTED },
            );
            if let Some(value) = row.value(settings) {
                paint.right(
                    PANEL_X + PANEL_WIDTH - 40.0,
                    y,
                    &value,
                    1.0,
                    if selected { CYAN } else { DIM },
                );
            }
            if matches!(
                row,
                Row::Fullscreen
                    | Row::Window
                    | Row::Filtering
                    | Row::Scaling
                    | Row::Volume
                    | Row::Mute
            ) {
                paint.text(
                    PANEL_X + PANEL_WIDTH - 66.0,
                    y,
                    "< >",
                    1.0,
                    if selected { AMBER } else { [43, 59, 72, 255] },
                );
            } else if matches!(row, Row::Resume | Row::Catalog | Row::Reset | Row::Quit) {
                paint.text(
                    PANEL_X + PANEL_WIDTH - 54.0,
                    y,
                    ">",
                    1.0,
                    if selected { AMBER } else { [43, 59, 72, 255] },
                );
            }
        }

        if settings.volume > 0.0 && !settings.muted {
            let width = 110.0 * settings.volume;
            paint.rect(
                PANEL_X + 300.0,
                ROWS_TOP + 5.0 * ROW_HEIGHT + 3.0,
                110.0,
                5.0,
                [22, 35, 47, 255],
            );
            paint.rect(
                PANEL_X + 300.0,
                ROWS_TOP + 5.0 * ROW_HEIGHT + 3.0,
                width,
                5.0,
                GREEN,
            );
        }

        paint.rect(
            PANEL_X + 20.0,
            PANEL_Y + PANEL_HEIGHT - 40.0,
            PANEL_WIDTH - 40.0,
            1.0,
            GRID,
        );
        paint.text(
            PANEL_X + 22.0,
            PANEL_Y + PANEL_HEIGHT - 27.0,
            &format!(
                "UP/DOWN SELECT   LEFT/RIGHT CHANGE   {} ACTIVATE   {} CLOSE",
                hints.confirm, hints.settings
            ),
            1.0,
            MUTED,
        );
        paint.text(
            PANEL_X + 22.0,
            PANEL_Y + PANEL_HEIGHT - 13.0,
            "DISPLAY AND AUDIO CHANGES APPLY IMMEDIATELY AND SAVE ON EXIT",
            1.0,
            [55, 76, 92, 255],
        );
        paint.into_canvas()
    }
}

impl Row {
    fn label(self) -> &'static str {
        match self {
            Self::Resume => "RESUME",
            Self::Fullscreen => "FULLSCREEN",
            Self::Window => "WINDOW SIZE",
            Self::Filtering => "FILTERING",
            Self::Scaling => "SCALING",
            Self::Volume => "MASTER VOLUME",
            Self::Mute => "MUTE",
            Self::Catalog => "BACK TO CATALOG",
            Self::Reset => "RESTORE DEFAULTS",
            Self::Quit => "QUIT CLASSIC CLASSICS",
        }
    }

    fn value(self, settings: &Settings) -> Option<String> {
        match self {
            Self::Fullscreen => Some(if settings.fullscreen { "ON" } else { "OFF" }.into()),
            Self::Window => Some(format!("{} x {}", settings.width, settings.height)),
            Self::Filtering => Some(if settings.linear { "SMOOTH" } else { "SHARP" }.into()),
            Self::Scaling => Some(if settings.integer { "INTEGER" } else { "FIT" }.into()),
            Self::Volume => Some(format!("{:03}%", (settings.volume * 100.0).round())),
            Self::Mute => Some(if settings.muted { "ON" } else { "OFF" }.into()),
            _ => None,
        }
    }
}

fn gear(paint: &mut Paint, cx: f32, cy: f32, color: [u8; 4]) {
    paint.glow_circle([cx, cy], 11.0, 1.2, color);
    paint.circle([cx, cy], 4.0, 1.0, color);
    for index in 0..8 {
        let angle = index as f32 / 8.0 * std::f32::consts::TAU;
        paint.line(
            [cx + angle.cos() * 12.0, cy + angle.sin() * 12.0],
            [cx + angle.cos() * 17.0, cy + angle.sin() * 17.0],
            2.0,
            color,
        );
    }
    paint.rect(cx - 2.0, cy - 17.0, 4.0, 4.0, CORAL);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(from_game: bool) -> (SettingsScreen, Settings, Vec<Sound>) {
        (
            SettingsScreen::new(from_game),
            Settings::default(),
            Vec::new(),
        )
    }

    fn activate(
        screen: &mut SettingsScreen,
        settings: &mut Settings,
        sounds: &mut Vec<Sound>,
        row: Row,
    ) {
        let list = rows(screen.from_game);
        screen.selection = list.iter().position(|candidate| *candidate == row).unwrap();
        assert_eq!(
            screen.tick(
                Input {
                    confirm: true,
                    ..Default::default()
                },
                settings,
                sounds,
            ),
            Outcome::Stay
        );
    }

    #[test]
    fn display_and_audio_rows_change_settings() {
        let (mut screen, mut settings, mut sounds) = screen(false);
        activate(&mut screen, &mut settings, &mut sounds, Row::Fullscreen);
        assert!(settings.fullscreen);
        activate(&mut screen, &mut settings, &mut sounds, Row::Filtering);
        assert!(settings.linear);
        activate(&mut screen, &mut settings, &mut sounds, Row::Scaling);
        assert!(settings.integer);
        activate(&mut screen, &mut settings, &mut sounds, Row::Mute);
        assert!(settings.muted);

        let list = rows(false);
        screen.selection = list.iter().position(|row| *row == Row::Volume).unwrap();
        screen.tick(
            Input {
                menu_right: true,
                ..Default::default()
            },
            &mut settings,
            &mut sounds,
        );
        assert!(settings.volume > 0.8);

        let list = rows(false);
        screen.selection = list.iter().position(|row| *row == Row::Window).unwrap();
        screen.tick(
            Input {
                menu_left: true,
                ..Default::default()
            },
            &mut settings,
            &mut sounds,
        );
        assert_eq!((settings.width, settings.height), (800, 500));
    }

    #[test]
    fn reset_restore_catalog_quit_and_close_are_explicit() {
        let (mut screen, mut settings, mut sounds) = screen(true);
        settings.fullscreen = true;
        settings.linear = true;
        activate(&mut screen, &mut settings, &mut sounds, Row::Reset);
        assert!(!settings.fullscreen);
        assert!(!settings.linear);

        let list = rows(true);
        screen.selection = list.iter().position(|row| *row == Row::Catalog).unwrap();
        assert_eq!(
            screen.tick(
                Input {
                    confirm: true,
                    ..Default::default()
                },
                &mut settings,
                &mut sounds,
            ),
            Outcome::LeaveToCatalog
        );

        let list = rows(true);
        screen.selection = list.iter().position(|row| *row == Row::Quit).unwrap();
        assert_eq!(
            screen.tick(
                Input {
                    confirm: true,
                    ..Default::default()
                },
                &mut settings,
                &mut sounds,
            ),
            Outcome::Quit
        );

        let list = rows(true);
        screen.selection = list.iter().position(|row| *row == Row::Resume).unwrap();
        assert_eq!(
            screen.tick(
                Input {
                    confirm: true,
                    ..Default::default()
                },
                &mut settings,
                &mut sounds,
            ),
            Outcome::Close
        );
    }

    #[test]
    fn escape_closes_without_changing_settings() {
        let (mut screen, mut settings, mut sounds) = screen(false);
        settings.volume = 0.35;
        let before = settings.clone();
        assert_eq!(
            screen.tick(
                Input {
                    settings: true,
                    ..Default::default()
                },
                &mut settings,
                &mut sounds,
            ),
            Outcome::Close
        );
        assert_eq!(settings.volume, before.volume);
    }
}
