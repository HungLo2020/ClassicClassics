//! Imported game systems with a Classic Classics host adapter.
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod game;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod render;
#[rustfmt::skip]
#[allow(clippy::collapsible_if, clippy::manual_is_multiple_of, clippy::derivable_impls)]
pub mod waves;
mod adapter;
pub use adapter::{Game, Record};
