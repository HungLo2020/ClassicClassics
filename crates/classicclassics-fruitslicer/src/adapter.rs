//! Classic Classics host adapter for the retained FruitSlicer game.
//!
//! The original deterministic simulation, renderer and retained `quasar-ui`
//! menus are reused unchanged. This module maps the shared logical `Input` to
//! FruitSlicer's 1280x720 pointer space, exposes asset-aware draw frames and
//! records high scores in the collection's `Records` entry.

use crate::game::{GameEvent, Mode, Phase, TickInput, Vec2};
use crate::menu::{GameOverInfo, Menu, MenuAction, Screen};
use crate::settings::{ModeRecord, Records, Settings};
use classicclassics_core::{
    ArcadeGame, ControlHints, GameInfo, Input, Sound,
    audio::{AssetSound, MusicLoop},
    paint::{Color, Paint},
    tiled::DrawFrame,
};
use quasar_render_2d::Canvas;
use serde::{Deserialize, Serialize};

pub type Record = Records;

const WIDTH: u32 = crate::game::WIDTH as u32;
const HEIGHT: u32 = crate::game::HEIGHT as u32;
const TICK: f32 = 1.0 / crate::game::TICK_HZ as f32;

/// The retained simulation plus Classic Classics presentation state.
///
/// `Menu`, `Art` and the logo animation are presentation-only and therefore
/// skipped by snapshots. `Clone` rebuilds the retained menu from serialized
/// settings and the current screen.
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Game {
    engine: crate::game::Game,
    settings: Settings,
    records: Records,
    seed: u64,
    time: f32,
    pointer: Option<[f32; 2]>,
    fire_down: bool,
    roll: u64,
    starts: u64,
    saved_run: bool,
    quit: bool,
    music_started: bool,
    #[serde(skip)]
    menu: Menu,
    #[serde(skip)]
    art: Option<crate::art::Art>,
    #[serde(skip)]
    logo: crate::anim::LogoPulse,
    #[serde(skip)]
    last_game_over: Option<GameOverInfo>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new_with(
            Settings::default(),
            Records::default(),
            0x51ce_9a17_d00d_f00d,
        )
    }
}

impl Game {
    fn new_with(settings: Settings, records: Records, seed: u64) -> Self {
        Self {
            engine: crate::game::Game::new(Mode::Classic, settings.difficulty(), seed),
            settings: settings.clone(),
            records: records.clone(),
            seed,
            time: 0.0,
            pointer: None,
            fire_down: false,
            roll: 0x9e37_79b9_7f4a_7c15,
            starts: 0,
            saved_run: false,
            quit: false,
            music_started: false,
            menu: Menu::new(settings, false, records),
            art: None,
            logo: crate::anim::LogoPulse::default(),
            last_game_over: None,
        }
    }

    fn start_run(&mut self, mode: Mode, sounds: &mut Vec<Sound>) {
        self.starts = self.starts.wrapping_add(1);
        self.seed = self
            .seed
            .wrapping_add(self.starts.wrapping_mul(0x9e37_79b9));
        self.engine = crate::game::Game::new(mode, self.settings.difficulty(), self.seed);
        self.pointer = None;
        self.fire_down = false;
        self.saved_run = false;
        self.last_game_over = None;
        self.menu.close();
        push_effect(sounds, "click.wav");
    }

    fn finish_run(&mut self, sounds: &mut Vec<Sound>) {
        if self.saved_run {
            return;
        }
        self.saved_run = true;
        let mode = self.engine.mode;
        let score = self.engine.score;
        let new_best = self.records.record_run(
            mode,
            score,
            self.engine.stats.best_combo,
            self.engine.stats.sliced,
        );
        let info = GameOverInfo {
            mode,
            score,
            best: self.records.best(mode),
            new_best,
            reason: self.engine.reason,
        };
        self.menu.set_records(self.records.clone());
        self.menu.open_game_over(info.clone());
        self.last_game_over = Some(info);
        push_effect(sounds, "game_over.wav");
    }

    fn handle_menu_action(&mut self, action: MenuAction, sounds: &mut Vec<Sound>) {
        match action {
            MenuAction::None => {}
            MenuAction::Back => {}
            MenuAction::Start(mode) => self.start_run(mode, sounds),
            MenuAction::Resume => {
                self.engine.resume();
                self.menu.close();
                self.pointer = None;
                self.fire_down = false;
            }
            MenuAction::Restart => {
                let mode = self.engine.mode;
                self.start_run(mode, sounds);
            }
            MenuAction::QuitToMenu => {
                self.menu.open_main();
                self.pointer = None;
            }
            MenuAction::Quit => self.quit = true,
            MenuAction::Apply { settings, .. } => {
                self.settings = settings;
                self.settings.normalize();
            }
        }
    }

    fn ui_input(&self, input: Input, pressed: bool, released: bool) -> quasar_ui::UiInput {
        quasar_ui::UiInput {
            pointer: self
                .pointer
                .map(|pointer| quasar_ui::Vec2::new(pointer[0], pointer[1])),
            primary_down: input.fire,
            primary_pressed: pressed,
            primary_released: released,
            up: input.menu_up,
            down: input.menu_down,
            left: input.menu_left,
            right: input.menu_right,
            activate: input.confirm,
            cancel: input.back,
            ..quasar_ui::UiInput::default()
        }
    }

    fn rebuild_menu(&mut self) {
        let screen = self.menu.screen();
        let settings = self.settings.clone();
        let fullscreen = self.menu.fullscreen();
        let mut menu = Menu::new(settings, fullscreen, self.records.clone());
        match screen {
            Some(Screen::Main) => menu.open_main(),
            Some(Screen::Pause) => menu.open_pause(self.engine.mode),
            Some(Screen::Options) => {
                menu.open_main();
                menu.open_options();
            }
            Some(Screen::Help) => {
                menu.open_main();
                menu.open_help();
            }
            Some(Screen::GameOver) => {
                if let Some(info) = self.last_game_over.clone() {
                    menu.open_game_over(info);
                }
            }
            None => {}
        }
        self.menu = menu;
    }

    fn target(&self) -> Option<Vec2> {
        self.engine
            .fruits
            .iter()
            .filter(|fruit| !fruit.kind.is_bomb())
            .min_by(|a, b| {
                let da = (a.position.y - HEIGHT as f32 * 0.55).abs()
                    + (a.position.x - WIDTH as f32 * 0.5).abs() * 0.2;
                let db = (b.position.y - HEIGHT as f32 * 0.55).abs()
                    + (b.position.x - WIDTH as f32 * 0.5).abs() * 0.2;
                da.total_cmp(&db)
            })
            .map(|fruit| fruit.position)
    }

    fn draw(&self, hints: &ControlHints) -> Option<DrawFrame> {
        let art = self.art.as_ref()?;
        let best = self.records.best(self.engine.mode);
        let show_hud = self.menu.screen().is_none();
        let mut canvas = crate::render::scene(
            &self.engine,
            art,
            &crate::render::SceneOptions {
                best,
                fps: 0.0,
                show_fps: self.settings.gameplay.show_fps,
                pointer: self.pointer.map(|p| Vec2::new(p[0], p[1])),
                virtual_cursor: self.pointer.is_some(),
                reduced_flash: self.settings.gameplay.reduced_flash,
                show_hud,
            },
        );
        if let Some(ui) = self.menu.ui() {
            if self.menu.screen() == Some(Screen::Main) {
                crate::render::draw_menu_ambience(&mut canvas, art, self.time);
            }
            crate::render::dim(&mut canvas, 168);
            crate::menu_render::draw(&mut canvas, ui, &art.font);
            if self.menu.screen() == Some(Screen::Main) {
                let center = crate::menu::logo_center();
                crate::render::draw_logo(
                    &mut canvas,
                    art,
                    Vec2::new(center[0], center[1]),
                    crate::menu::logo_width() * self.logo.scale(),
                    255,
                );
            }
        }
        Some(DrawFrame {
            canvas,
            images: art.images.clone(),
            help: format!(
                "{} SLICE   {} PAUSE   {} SETTINGS   {} CATALOG",
                hints.fire, hints.pause, hints.settings, hints.back
            ),
        })
    }
}

impl Clone for Game {
    fn clone(&self) -> Self {
        let mut game = Self {
            engine: self.engine.clone(),
            settings: self.settings.clone(),
            records: self.records.clone(),
            seed: self.seed,
            time: self.time,
            pointer: self.pointer,
            fire_down: self.fire_down,
            roll: self.roll,
            starts: self.starts,
            saved_run: self.saved_run,
            quit: self.quit,
            music_started: self.music_started,
            menu: Menu::new(
                self.settings.clone(),
                self.menu.fullscreen(),
                self.records.clone(),
            ),
            art: self.art.clone(),
            logo: crate::anim::LogoPulse::default(),
            last_game_over: self.last_game_over.clone(),
        };
        game.rebuild_menu();
        game
    }
}

impl std::fmt::Debug for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FruitSlicer")
            .field("mode", &self.engine.mode)
            .field("phase", &self.engine.phase)
            .field("score", &self.engine.score)
            .field("menu", &self.menu.screen())
            .finish()
    }
}

impl ArcadeGame for Game {
    type Record = Record;

    const INFO: GameInfo = GameInfo {
        name: "Fruit Slicer",
        aliases: &["fruitslicer", "fruit-slicer", "fruit"],
        tagline: "SWIPE FAST. AVOID THE BOMBS.",
        accent: [255, 150, 86, 255],
    };
    const ASSET_MANIFEST_DIR: Option<&'static str> = Some(env!("CARGO_MANIFEST_DIR"));
    const COVER: Option<&'static [u8]> = Some(include_bytes!("../assets/basalt/cover.png"));

    fn load(root: Option<&classicclassics_assets::Directory>) -> Result<Self, String> {
        let root = root.ok_or("Fruit Slicer asset mount missing")?;
        let art = crate::art::Art::load_directory(root)?;
        let mut game = Self {
            art: Some(art),
            ..Self::default()
        };
        game.menu.open_main();
        Ok(game)
    }

    fn paused(&self) -> bool {
        self.engine.phase == Phase::Paused
    }

    fn quit_requested(&self) -> bool {
        self.quit
    }

    fn canvas_size(&self) -> (u32, u32) {
        (WIDTH, HEIGHT)
    }

    fn tick(&mut self, input: Input, sounds: &mut Vec<Sound>) {
        if !self.music_started && !self.quit {
            self.music_started = true;
            sounds.push(Sound::Asset(AssetSound::Music {
                game: "fruitslicer",
                file: crate::audio_cues::MUSIC,
                region: MusicLoop::Whole,
            }));
        }
        if input.quit {
            self.quit = true;
            return;
        }
        self.time += TICK;
        self.logo.advance(TICK);
        if let Some(pointer) = input.pointer {
            self.pointer = Some(pointer);
        }
        let pressed = input.fire_pressed || (input.fire && !self.fire_down);
        let released = !input.fire && self.fire_down;
        self.fire_down = input.fire;

        if self.menu.is_open() {
            let ui = self.ui_input(input, pressed, released);
            if let Ok(action) = self.menu.update(&ui, TICK) {
                self.handle_menu_action(action, sounds);
            }
            return;
        }

        if input.pause {
            self.engine.pause();
            self.menu.open_pause(self.engine.mode);
            self.pointer = None;
            self.fire_down = false;
            push_effect(sounds, "click.wav");
            return;
        }
        if input.restart {
            self.engine.restart();
            self.saved_run = false;
            self.last_game_over = None;
            push_effect(sounds, "click.wav");
            return;
        }

        self.engine.tick(TickInput {
            pointer: self.pointer.map(|p| Vec2::new(p[0], p[1])),
            pointer_down: input.fire,
            start: input.confirm,
            pause: false,
            restart: false,
        });
        for event in self.engine.take_events() {
            if matches!(event, GameEvent::GameOver(_)) {
                self.finish_run(sounds);
            } else {
                play_event(&event, &mut self.roll, sounds);
            }
        }
    }

    fn autoplay(&self) -> Input {
        if self.menu.is_open() {
            return Input {
                confirm: true,
                ..Input::default()
            };
        }
        let Some(target) = self.target() else {
            return Input {
                fire: true,
                ..Input::default()
            };
        };
        let tick = self.engine.ticks;
        let offset = if tick.is_multiple_of(2) { 72.0 } else { -72.0 };
        Input {
            pointer: Some([target.x + offset, target.y]),
            fire: true,
            fire_pressed: tick.is_multiple_of(2),
            ..Input::default()
        }
    }

    fn scene(&self, hints: &ControlHints) -> Canvas {
        self.draw(hints)
            .map(|frame| frame.canvas)
            .unwrap_or_else(|| Canvas::new(WIDTH, HEIGHT).expect("valid canvas"))
    }

    fn canvas_frame(&self, hints: &ControlHints) -> Option<DrawFrame> {
        self.draw(hints)
    }

    fn record(&mut self, record: &mut Self::Record) {
        let before = self.records.clone();
        merge_records(&mut self.records, record);
        if self.records != before {
            self.menu.set_records(self.records.clone());
        }
        *record = self.records.clone();
    }

    fn summary(&self) -> String {
        format!(
            "mode={:?} phase={:?} score={} best={} fruits={} lives={} combo={} menu={:?}",
            self.engine.mode,
            self.engine.phase,
            self.engine.score,
            self.records.best(self.engine.mode),
            self.engine.fruits.len(),
            self.engine.lives,
            self.engine.stats.best_combo,
            self.menu.screen()
        )
    }

    fn catalog_record(record: &Record) -> (&'static str, String) {
        let best = Mode::ALL
            .into_iter()
            .map(|mode| record.best(mode))
            .max()
            .unwrap_or(0);
        ("BEST SCORE", best.to_string())
    }

    fn motif(paint: &mut Paint, cx: f32, cy: f32, accent: Color) {
        paint.outline(cx - 58.0, cy - 54.0, 116.0, 116.0, accent);
        paint.line([cx - 78.0, cy + 78.0], [cx + 78.0, cy - 78.0], 4.0, accent);
        paint.line([cx - 52.0, cy - 8.0], [cx + 12.0, cy + 56.0], 3.0, accent);
        paint.rect(cx - 15.0, cy - 77.0, 34.0, 12.0, accent);
    }
}

fn merge_records(local: &mut Records, external: &Records) {
    for mode in Mode::ALL {
        merge_mode(local.get_mut(mode), external.get(mode));
    }
}

fn merge_mode(local: &mut ModeRecord, external: &ModeRecord) {
    local.best = local.best.max(external.best);
    local.runs = local.runs.max(external.runs);
    local.best_combo = local.best_combo.max(external.best_combo);
    local.total_sliced = local.total_sliced.max(external.total_sliced);
}

fn play_event(event: &GameEvent, roll: &mut u64, sounds: &mut Vec<Sound>) {
    let file = match event {
        GameEvent::Slice { .. } => {
            *roll ^= *roll << 13;
            *roll ^= *roll >> 7;
            *roll ^= *roll << 17;
            let index = (*roll as usize) % crate::audio_cues::SLICES.len();
            crate::audio_cues::SLICES[index]
        }
        GameEvent::Bomb => "bomb.wav",
        GameEvent::Freeze => "freeze.wav",
        GameEvent::Frenzy => "frenzy.wav",
        GameEvent::Bonus => "bonus.wav",
        GameEvent::Heart | GameEvent::BonusLife => "heart.wav",
        GameEvent::Combo { .. } => "combo.wav",
        GameEvent::Miss => "miss.wav",
        GameEvent::GameOver(_) => "game_over.wav",
    };
    push_effect(sounds, file);
}

fn push_effect(sounds: &mut Vec<Sound>, file: &'static str) {
    sounds.push(Sound::Asset(AssetSound::Effect {
        game: "fruitslicer",
        file,
    }));
}
