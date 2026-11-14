//! Shared emulator front-end used by the NES, SNES, GBA and Atari 2600 cards.
//!
//! The collection owns ROM discovery, the catalog and audio output. These games
//! own one selected cartridge, its battery file and its automatic save state.
//! NES runs the pure-Rust TetaNES core; the other systems load a pinned
//! libretro core from `app/cores`.

mod backend;
mod game;
mod libretro;
mod nes;
mod picker;
mod systems;

pub use backend::{
    AudioBatch, BTN_A, BTN_B, BTN_DOWN, BTN_L, BTN_LEFT, BTN_R, BTN_RIGHT, BTN_SELECT, BTN_START,
    BTN_UP, BTN_X, BTN_Y, Backend, Frame,
};
pub use game::{EmulatorGame, Record};
pub use systems::{Atari2600System, GbaSystem, NesSystem, SnesSystem};

use classicclassics_core::{
    ControlHints, GameInfo, Input,
    paint::{Color, Paint},
};
use std::path::Path;

pub trait EmulatorSystem: Default + Clone + Copy + std::fmt::Debug + Send + 'static {
    const INFO: GameInfo;
    const ASSET_MANIFEST_DIR: Option<&'static str>;
    const ROM_EXTENSIONS: &'static [&'static str];
    const CORE_FILE: Option<&'static str>;
    const FRAME_HZ: f64;
    const ASPECT: f32;
    fn open(
        rom: &[u8],
        path: &Path,
        save_dir: Option<&Path>,
        core_dir: Option<&Path>,
    ) -> Result<Box<dyn Backend>, String>;
    fn motif(paint: &mut Paint, cx: f32, cy: f32, accent: Color);
    /// Smoke autoplay may need a Start press; Atari Start is a console reset.
    fn autoplay_confirm(ticks: u64) -> bool {
        ticks % 300 < 8
    }
    fn controls_help(h: &ControlHints) -> String {
        format!(
            "MOVE {} - A:{} - B:{} - X:{} - Y:{} - L:{} - R:{} - START:{} - SELECT:{} - ESC:{}",
            h.move_x,
            h.emu_a,
            h.emu_b,
            h.emu_x,
            h.emu_y,
            h.emu_l,
            h.emu_r,
            h.start,
            h.select,
            h.settings
        )
    }
    fn buttons(input: &Input) -> [u16; 2] {
        let map = |a, b, x, y, l, r, start, select, axis_x, axis_y| {
            let mut buttons = 0;
            if axis_x < 0 {
                buttons |= BTN_LEFT;
            }
            if axis_x > 0 {
                buttons |= BTN_RIGHT;
            }
            if axis_y < 0 {
                buttons |= BTN_UP;
            }
            if axis_y > 0 {
                buttons |= BTN_DOWN;
            }
            if a {
                buttons |= BTN_A;
            }
            if b {
                buttons |= BTN_B;
            }
            if x {
                buttons |= BTN_X;
            }
            if y {
                buttons |= BTN_Y;
            }
            if l {
                buttons |= BTN_L;
            }
            if r {
                buttons |= BTN_R;
            }
            if start {
                buttons |= BTN_START;
            }
            if select {
                buttons |= BTN_SELECT;
            }
            buttons
        };
        let p1 = map(
            input.emu_a,
            input.emu_b,
            input.emu_x,
            input.emu_y,
            input.emu_l,
            input.emu_r,
            input.start,
            input.select,
            input.move_x,
            input.move_y,
        );
        let other = input.other_players[0];
        let p2 = map(
            other.emu_a,
            other.emu_b,
            other.emu_x,
            other.emu_y,
            other.emu_l,
            other.emu_r,
            false,
            false,
            other.x,
            other.y,
        );
        [p1, p2]
    }
}
