//! Collection orchestration shared by the native and headless hosts.
pub mod audio;
pub mod cli;
mod config_file;
pub mod controls;
pub mod controls_screen;
pub mod games;
pub mod menu;
mod records;
pub mod rendering;
pub mod session;
pub mod settings;
pub mod settings_screen;

pub use classicclassics_core::{HEIGHT, Input, TICK_HZ, WIDTH, paint};
pub use games::{Game, GameKind, Records};
pub use session::{Screen, Session};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

pub use classicclassics_core::input;
