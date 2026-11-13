//! Shared contracts and drawing helpers for deterministic arcade games.
//!
//! This crate has no window, filesystem, controller backend or audio device.
pub mod audio;
pub mod controls;
pub mod imported;
pub mod input;
pub mod paint;
pub mod simulation;
pub mod tiled;

pub use audio::Sound;
pub use controls::ControlHints;
pub use input::Input;
use paint::{Color, Paint};
use quasar_render_2d::Canvas;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::path::PathBuf;

/// Games keep their original logical playfield regardless of window size.
pub const WIDTH: u32 = 960;
pub const HEIGHT: u32 = 600;
pub const TICK_HZ: f64 = 120.0;

pub struct GameInfo {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub tagline: &'static str,
    pub accent: Color,
}

/// A cartridge discovered by the collection and offered to an emulated system.
/// Discovery and filesystem policy belong to the collection; the emulated game
/// only reads the selected file and its own save files.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RomEntry {
    pub system: String,
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    /// Stable FNV-1a identity for save files and records.
    pub id: u64,
    pub builtin: bool,
}

/// Inputs supplied by the collection when an emulated system starts.
#[derive(Clone)]
pub struct GameLoadContext {
    pub assets: Option<classicclassics_assets::Directory>,
    pub roms: Vec<RomEntry>,
    pub initial_rom: Option<RomEntry>,
    pub save_dir: Option<PathBuf>,
    pub core_dir: Option<PathBuf>,
    pub save_enabled: bool,
}

impl Default for GameLoadContext {
    fn default() -> Self {
        Self {
            assets: None,
            roms: Vec::new(),
            initial_rom: None,
            save_dir: None,
            core_dir: None,
            save_enabled: true,
        }
    }
}

/// One batch of emulator PCM at the core's native rate. The collection host
/// resamples and queues it; headless runs simply drop it.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EmulatorAudio {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

/// A game owns its state, records, catalog presentation and fixed-step logic.
/// The collection handles selection, persistence and platform devices.
pub trait ArcadeGame: Default + Clone + std::fmt::Debug + Serialize + DeserializeOwned {
    type Record: Default + Clone + std::fmt::Debug + Serialize + DeserializeOwned;
    const INFO: GameInfo;
    const ASSET_MANIFEST_DIR: Option<&'static str> = None;
    /// The collection supplies a mounted namespace. Default remains a convenience
    /// for procedural games and standalone simulation fixtures.
    fn load(_assets: Option<&classicclassics_assets::Directory>) -> Result<Self, String> {
        Ok(Self::default())
    }
    /// Collection-aware constructor. Procedural games keep using `load`.
    fn load_with(ctx: &GameLoadContext) -> Result<Self, String> {
        Self::load(ctx.assets.as_ref())
    }
    const COVER: Option<&'static [u8]> = None;
    fn draw_frame(&self, _hints: &ControlHints) -> Option<tiled::DrawFrame> {
        None
    }
    /// A native-resolution canvas with its own images, presented without the
    /// 4:3 letterboxing that `draw_frame` applies to tile scenes.
    fn canvas_frame(&self, _hints: &ControlHints) -> Option<tiled::DrawFrame> {
        None
    }
    fn pixel_frame(&self, _hints: &ControlHints) -> Option<imported::PixelFrame> {
        None
    }
    /// Emulated systems return native-rate PCM produced by the last video frame.
    fn take_audio(&mut self) -> Option<EmulatorAudio> {
        None
    }
    /// Handles Back inside the game. Emulated systems leave a running cartridge
    /// for their ROM picker; returning false lets the collection show the catalog.
    fn back_to_picker(&mut self) -> bool {
        false
    }
    /// Flush persistence before the collection leaves or exits.
    fn save(&mut self) {}

    /// Logical canvas used for pointer mapping and presentation.
    fn canvas_size(&self) -> (u32, u32) {
        (WIDTH, HEIGHT)
    }

    fn paused(&self) -> bool {
        false
    }

    /// True when the game's own menu asked the host to exit.
    fn quit_requested(&self) -> bool {
        false
    }
    fn tick(&mut self, input: Input, sounds: &mut Vec<Sound>);
    fn autoplay(&self) -> Input;
    fn scene(&self, hints: &ControlHints) -> Canvas;
    fn record(&mut self, record: &mut Self::Record);
    fn summary(&self) -> String;
    fn catalog_record(record: &Self::Record) -> (&'static str, String);
    fn motif(paint: &mut Paint, cx: f32, cy: f32, accent: Color);
}
