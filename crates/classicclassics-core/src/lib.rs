//! Shared contracts and drawing helpers for deterministic arcade games.
//!
//! This crate has no window, filesystem, controller backend or audio device.
pub mod audio;
pub mod controls;
pub mod input;
pub mod paint;
pub mod simulation;

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

    fn tick(&mut self, input: Input, sounds: &mut Vec<Sound>);
    fn autoplay(&self) -> Input;
    fn scene(&self, hints: &ControlHints) -> Canvas;
    fn record(&mut self, record: &mut Self::Record);
    fn summary(&self) -> String;
    fn catalog_record(record: &Self::Record) -> (&'static str, String);
    fn motif(paint: &mut Paint, cx: f32, cy: f32, accent: Color);
}
