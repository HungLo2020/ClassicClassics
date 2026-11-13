use crate::{Backend, EmulatorSystem};
use classicclassics_core::{
    ControlHints, GameInfo, Input,
    paint::{Color, Paint},
};
use std::path::Path;

const ASSETS: &str = env!("CARGO_MANIFEST_DIR");

#[derive(Default, Clone, Copy, Debug)]
pub struct NesSystem;
impl EmulatorSystem for NesSystem {
    const INFO: GameInfo = GameInfo {
        name: "Nintendo Entertainment System",
        aliases: &["nes", "nintendo", "famicom"],
        tagline: "INSERT CARTRIDGE",
        accent: [232, 92, 92, 255],
    };
    const ASSET_MANIFEST_DIR: Option<&'static str> = Some(ASSETS);
    const ROM_EXTENSIONS: &'static [&'static str] = &["nes"];
    const CORE_FILE: Option<&'static str> = None;
    const FRAME_HZ: f64 = 60.0988;
    const ASPECT: f32 = 4.0 / 3.0;
    fn open(
        rom: &[u8],
        path: &Path,
        save_dir: Option<&Path>,
        _core_dir: Option<&Path>,
    ) -> Result<Box<dyn Backend>, String> {
        crate::nes::open(rom, path, save_dir)
    }
    fn controls_help(h: &ControlHints) -> String {
        format!(
            "MOVE {} - A:{} - B:{} - START:{} - SELECT:{} - ESC:{}",
            h.move_x, h.fire, h.jump, h.confirm, h.mode, h.settings
        )
    }
    fn motif(paint: &mut Paint, cx: f32, cy: f32, accent: Color) {
        paint.rect(cx - 78.0, cy - 42.0, 156.0, 84.0, [42, 40, 48, 255]);
        paint.rect(cx - 62.0, cy - 30.0, 124.0, 44.0, [24, 23, 28, 255]);
        paint.rect(cx - 50.0, cy + 22.0, 100.0, 10.0, accent);
        paint.text(cx - 48.0, cy - 20.0, "NES", 2.0, accent);
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct SnesSystem;
impl EmulatorSystem for SnesSystem {
    const INFO: GameInfo = GameInfo {
        name: "Super Nintendo",
        aliases: &["snes", "super-nintendo", "superfamicom"],
        tagline: "16-BIT CARTRIDGE",
        accent: [146, 132, 255, 255],
    };
    const ASSET_MANIFEST_DIR: Option<&'static str> = Some(ASSETS);
    const ROM_EXTENSIONS: &'static [&'static str] = &["sfc", "smc"];
    const CORE_FILE: Option<&'static str> = Some("snes9x_libretro.so");
    const FRAME_HZ: f64 = 60.0988;
    const ASPECT: f32 = 4.0 / 3.0;
    fn open(
        rom: &[u8],
        path: &Path,
        save_dir: Option<&Path>,
        core_dir: Option<&Path>,
    ) -> Result<Box<dyn Backend>, String> {
        crate::libretro::open(Self::CORE_FILE.unwrap(), rom, path, save_dir, core_dir)
    }
    fn motif(paint: &mut Paint, cx: f32, cy: f32, accent: Color) {
        paint.rect(cx - 80.0, cy - 34.0, 160.0, 68.0, [45, 40, 58, 255]);
        for x in [-62.0, -32.0, 32.0, 62.0] {
            paint.circle([cx + x, cy], 11.0, 4.0, accent);
        }
        paint.rect(cx - 16.0, cy - 22.0, 32.0, 44.0, [24, 22, 30, 255]);
        paint.rect(cx - 10.0, cy + 12.0, 20.0, 6.0, accent);
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct GbaSystem;
impl EmulatorSystem for GbaSystem {
    const INFO: GameInfo = GameInfo {
        name: "Game Boy Advance",
        aliases: &["gba", "gameboy-advance"],
        tagline: "32-BIT HANDHELD",
        accent: [122, 220, 146, 255],
    };
    const ASSET_MANIFEST_DIR: Option<&'static str> = Some(ASSETS);
    const ROM_EXTENSIONS: &'static [&'static str] = &["gba", "agb"];
    const CORE_FILE: Option<&'static str> = Some("mgba_libretro.so");
    const FRAME_HZ: f64 = 59.7275;
    const ASPECT: f32 = 3.0 / 2.0;
    fn open(
        rom: &[u8],
        path: &Path,
        save_dir: Option<&Path>,
        core_dir: Option<&Path>,
    ) -> Result<Box<dyn Backend>, String> {
        crate::libretro::open(Self::CORE_FILE.unwrap(), rom, path, save_dir, core_dir)
    }
    fn controls_help(h: &ControlHints) -> String {
        format!(
            "MOVE {} - A:{} - B:{} - L:{} - R:{} - START:{} - SELECT:{} - ESC:{}",
            h.move_x,
            h.fire,
            h.jump,
            h.shoulder_left,
            h.shoulder_right,
            h.confirm,
            h.mode,
            h.settings
        )
    }
    fn motif(paint: &mut Paint, cx: f32, cy: f32, accent: Color) {
        paint.rect(cx - 74.0, cy - 44.0, 148.0, 88.0, [36, 48, 40, 255]);
        paint.rect(cx - 54.0, cy - 30.0, 108.0, 60.0, [22, 32, 26, 255]);
        paint.rect(cx - 72.0, cy - 16.0, 10.0, 32.0, [46, 42, 40, 255]);
        paint.rect(cx + 62.0, cy - 16.0, 10.0, 32.0, [46, 42, 40, 255]);
        paint.rect(cx + 24.0, cy - 22.0, 20.0, 10.0, accent);
        paint.circle([cx + 43.0, cy + 10.0], 6.0, 3.0, accent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BTN_A, BTN_B, BTN_L, BTN_LEFT, BTN_RIGHT};

    #[test]
    fn families_map_the_shared_input_to_their_own_buttons() {
        let input = Input {
            move_x: -1,
            fire: true,
            jump: true,
            shoulder_left: true,
            shoulder_right: true,
            ..Default::default()
        };
        let snes = SnesSystem::buttons(&input)[0];
        assert_eq!(snes & BTN_A, BTN_A);
        assert_eq!(snes & BTN_B, BTN_B);
        assert_eq!(snes & BTN_L, BTN_L);
        assert_eq!(snes & BTN_LEFT, BTN_LEFT);
        assert_eq!(snes & BTN_RIGHT, 0);
        let atari = Atari2600System::buttons(&input)[0];
        assert_eq!(atari & BTN_B, BTN_B);
        assert_eq!(atari & BTN_A, 0);
        assert_eq!(atari & BTN_LEFT, BTN_LEFT);
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Atari2600System;
impl EmulatorSystem for Atari2600System {
    const INFO: GameInfo = GameInfo {
        name: "Atari 2600",
        aliases: &["atari2600", "2600", "atari"],
        tagline: "VIDEO COMPUTER SYSTEM",
        accent: [245, 190, 92, 255],
    };
    const ASSET_MANIFEST_DIR: Option<&'static str> = Some(ASSETS);
    const ROM_EXTENSIONS: &'static [&'static str] = &["a26", "bin"];
    const CORE_FILE: Option<&'static str> = Some("stella2023_libretro.so");
    const FRAME_HZ: f64 = 60.0;
    const ASPECT: f32 = 4.0 / 3.0;
    fn open(
        rom: &[u8],
        path: &Path,
        save_dir: Option<&Path>,
        core_dir: Option<&Path>,
    ) -> Result<Box<dyn Backend>, String> {
        crate::libretro::open(Self::CORE_FILE.unwrap(), rom, path, save_dir, core_dir)
    }
    fn autoplay_confirm(_ticks: u64) -> bool {
        false
    }
    fn controls_help(h: &ControlHints) -> String {
        format!(
            "MOVE {} - FIRE:{} - RESET:{} - SELECT:{} - ESC:{}",
            h.move_x, h.fire, h.confirm, h.mode, h.settings
        )
    }
    fn buttons(input: &Input) -> [u16; 2] {
        let stick = |x: i8, y: i8| {
            let mut b = 0;
            if x < 0 {
                b |= crate::BTN_LEFT;
            }
            if x > 0 {
                b |= crate::BTN_RIGHT;
            }
            if y < 0 {
                b |= crate::BTN_UP;
            }
            if y > 0 {
                b |= crate::BTN_DOWN;
            }
            b
        };
        let mut p1 = stick(input.move_x, input.move_y);
        if input.fire {
            p1 |= crate::BTN_B;
        }
        if input.confirm {
            p1 |= crate::BTN_START;
        }
        if input.mode {
            p1 |= crate::BTN_SELECT;
        }
        let other = input.other_players[0];
        let mut p2 = stick(other.x, other.y);
        if other.fire {
            p2 |= crate::BTN_B;
        }
        [p1, p2]
    }
    fn motif(paint: &mut Paint, cx: f32, cy: f32, accent: Color) {
        paint.rect(cx - 82.0, cy - 36.0, 164.0, 72.0, [52, 42, 30, 255]);
        paint.rect(cx - 70.0, cy - 24.0, 92.0, 48.0, [26, 22, 18, 255]);
        for x in [30.0, 48.0, 66.0] {
            paint.rect(cx + x, cy - 20.0, 8.0, 40.0, accent);
        }
        paint.rect(cx - 50.0, cy + 32.0, 100.0, 6.0, accent);
    }
}
