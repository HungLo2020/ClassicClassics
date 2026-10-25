//! Classic Classics: a Quasar-powered single-window arcade collection.
//!
//! The library owns the simulation, drawing, persistence and menu logic. The
//! native desktop host lives in `main`/`desktop` and is only compiled with the
//! `desktop` feature, so the same simulation and draw lists run headless in
//! tests and release verification.

pub mod audio;
pub mod cli;
pub mod controls;
pub mod games;
pub mod menu;
pub mod paint;
pub mod session;
pub mod settings;
pub mod settings_screen;

pub use session::{Game, GameKind, Input, Records, Screen, Session};

/// Logical canvas width. The window scales this with Quasar's viewport math.
pub const WIDTH: u32 = 960;
/// Logical canvas height.
pub const HEIGHT: u32 = 600;
/// Simulation rate shared by every game and the menu.
pub const TICK_HZ: f64 = 120.0;

/// Collection version, used by state files and the CLI.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
