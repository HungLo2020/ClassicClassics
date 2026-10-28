//! Imported game systems with a Classic Classics host adapter.
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod assets;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod camera;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod enemy;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod events;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod font;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod game;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod input;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod outcomes;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod physics;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod pickup;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod player;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod projectile;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod scenes;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod video;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod world;
mod adapter;
pub use adapter::{Game, Record};

pub mod audio_cues;
