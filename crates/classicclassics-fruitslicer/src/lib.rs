//! FruitSlicer: a polished Fruit Ninja-style game built on Quasar.
//!
//! The simulation, renderer and retained menus are retained from the upstream
//! FruitSlicer project. [`adapter`] connects them to the Classic Classics
//! `ArcadeGame` contract, catalog, persistence and shared controls.
pub mod anim;
pub mod art;
pub mod game;
pub mod menu;
pub mod menu_render;
pub mod render;
pub mod session;
pub mod settings;

mod adapter;

pub use adapter::{Game, Record};

/// Original audio file names shipped under `app/games/fruitslicer/audio`.
pub mod audio_cues {
    pub const MUSIC: &str = "music_dojo.ogg";
    pub const EFFECTS: &[&str] = &[
        "slice_1.wav",
        "slice_2.wav",
        "slice_3.wav",
        "bomb.wav",
        "freeze.wav",
        "frenzy.wav",
        "bonus.wav",
        "combo.wav",
        "miss.wav",
        "game_over.wav",
        "click.wav",
        "heart.wav",
    ];
    pub const SLICES: &[&str] = &["slice_1.wav", "slice_2.wav", "slice_3.wav"];
}
