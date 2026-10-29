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
use serde::{Serialize, de::DeserializeOwned};

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
    const COVER: Option<&'static [u8]> = None;
    fn draw_frame(&self, _hints: &ControlHints) -> Option<tiled::DrawFrame> {
        None
    }
    fn pixel_frame(&self, _hints: &ControlHints) -> Option<imported::PixelFrame> {
        None
    }

    fn paused(&self) -> bool {
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
