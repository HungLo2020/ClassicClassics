use crate::{
    AudioBatch, BTN_A, BTN_B, BTN_DOWN, BTN_LEFT, BTN_RIGHT, BTN_SELECT, BTN_START, BTN_UP,
    Backend, Frame,
};
use std::borrow::Cow;
use std::io::Cursor;
use std::path::Path;
use tetanes_core::common::ResetKind;
use tetanes_core::control_deck::{Clocked, Config, ControlDeck};
use tetanes_core::input::{Joypad, JoypadBtn, Player};

pub struct NesBackend {
    deck: ControlDeck,
    frame: Frame,
    audio: Vec<f32>,
    buttons: [u16; 4],
}

pub fn open(rom: &[u8], path: &Path, _save_dir: Option<&Path>) -> Result<Box<dyn Backend>, String> {
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("cartridge");
    let config = Config::default().with_sram_dir(None);
    let mut deck = ControlDeck::with_config(config);
    deck.set_sample_rate(48_000.0);
    let rom = repair_ines(rom);
    deck.load_rom(name, &mut Cursor::new(rom.as_ref()))
        .map_err(|e| format!("NES ROM rejected: {e}"))?;
    deck.set_frame_speed(1.0);
    Ok(Box::new(NesBackend {
        deck,
        frame: Frame {
            width: 256,
            height: 240,
            aspect: 4.0 / 3.0,
            pixels: vec![0; 256 * 240],
        },
        audio: Vec::new(),
        buttons: [0; 4],
    }))
}

/// Old dumpers wrote their tags into iNES header bytes 7..16. TetaNES
/// correctly rejects that; clear the tag while preserving the mapper nibble
/// already stored in byte 6. NES 2.0 headers are left untouched.
fn repair_ines(rom: &[u8]) -> Cow<'_, [u8]> {
    if rom.len() > 16
        && &rom[..4] == b"NES\x1a"
        && rom[7] & 0x0C != 0x08
        && rom[7..16].iter().any(|byte| *byte != 0)
    {
        let mut repaired = rom.to_vec();
        repaired[7..16].fill(0);
        return Cow::Owned(repaired);
    }
    Cow::Borrowed(rom)
}

impl NesBackend {
    fn apply_buttons(&mut self, port: usize, buttons: u16) {
        let Some(player) = [Player::One, Player::Two, Player::Three, Player::Four].get(port) else {
            return;
        };
        let joypad: &mut Joypad = self.deck.joypad_mut(*player);
        joypad.set_button(JoypadBtn::A, buttons & BTN_A != 0);
        joypad.set_button(JoypadBtn::B, buttons & BTN_B != 0);
        joypad.set_button(JoypadBtn::Select, buttons & BTN_SELECT != 0);
        joypad.set_button(JoypadBtn::Start, buttons & BTN_START != 0);
        joypad.set_button(JoypadBtn::Up, buttons & BTN_UP != 0);
        joypad.set_button(JoypadBtn::Down, buttons & BTN_DOWN != 0);
        joypad.set_button(JoypadBtn::Left, buttons & BTN_LEFT != 0);
        joypad.set_button(JoypadBtn::Right, buttons & BTN_RIGHT != 0);
    }
}

impl Backend for NesBackend {
    fn frame_hz(&self) -> f64 {
        60.0988
    }

    fn run_frame(&mut self) {
        loop {
            match self.deck.clock_frame() {
                Ok(Clocked::Continue) => {}
                Ok(_) => break,
                Err(error) => {
                    eprintln!("NES core stopped: {error}");
                    break;
                }
            }
        }
        let rgba = self.deck.frame_buffer();
        self.frame.pixels.clear();
        self.frame.pixels.extend(
            rgba.chunks_exact(4)
                .map(|p| (u32::from(p[0]) << 16) | (u32::from(p[1]) << 8) | u32::from(p[2])),
        );
        self.audio.clear();
        self.audio.extend_from_slice(self.deck.audio_samples());
    }

    fn frame(&self) -> Option<&Frame> {
        Some(&self.frame)
    }

    fn set_buttons(&mut self, port: usize, buttons: u16) {
        if port < self.buttons.len() {
            self.buttons[port] = buttons;
        }
        self.apply_buttons(port, buttons);
    }

    fn take_audio(&mut self) -> Option<AudioBatch> {
        if self.audio.is_empty() {
            return None;
        }
        Some(AudioBatch {
            sample_rate: 48_000,
            channels: 1,
            samples: std::mem::take(&mut self.audio),
        })
    }

    fn save_state(&mut self) -> Option<Vec<u8>> {
        let len = self.deck.serialized_state_len().ok()?;
        let mut data = vec![0; len];
        let written = self.deck.serialize_state_into(&mut data).ok()?;
        data.truncate(written);
        Some(data)
    }

    fn load_state(&mut self, data: &[u8]) -> bool {
        self.deck.deserialize_state(data).is_ok()
    }

    fn sram(&mut self) -> Option<Vec<u8>> {
        let data = self.deck.sram();
        (!data.is_empty()).then(|| data.to_vec())
    }

    fn set_sram(&mut self, data: &[u8]) {
        self.deck.set_sram(data);
    }

    fn reset(&mut self) {
        self.deck.reset(ResetKind::Hard);
    }
}
