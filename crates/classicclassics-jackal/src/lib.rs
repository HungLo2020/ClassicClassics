//! Imported game systems with a Classic Classics host adapter.
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod assets;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod boss;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod cinematic;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod enemy;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod game;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod hud;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod input;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod math;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod player;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod renderer;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod sound;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod terrain;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod weapon;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod victory_scene;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod boss_states;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod world_art;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod boss_vehicle;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod boss_child;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod camera;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod rescue;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod boss_hq;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod scoring;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod rescue_helicopter;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod pickup;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod mine;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod hq_effects;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod background_object;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod actor_pool;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod environment;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod boss_controller;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod river_art;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod stage_three_water;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod palette_cycle;
pub const TICKS_PER_SECOND: f64 = 60.0988;
mod adapter;
pub use adapter::{Game, Record};
