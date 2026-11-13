//! One common surface for native and libretro emulator cores.

pub const BTN_B: u16 = 1 << 0;
pub const BTN_Y: u16 = 1 << 1;
pub const BTN_SELECT: u16 = 1 << 2;
pub const BTN_START: u16 = 1 << 3;
pub const BTN_UP: u16 = 1 << 4;
pub const BTN_DOWN: u16 = 1 << 5;
pub const BTN_LEFT: u16 = 1 << 6;
pub const BTN_RIGHT: u16 = 1 << 7;
pub const BTN_A: u16 = 1 << 8;
pub const BTN_X: u16 = 1 << 9;
pub const BTN_L: u16 = 1 << 10;
pub const BTN_R: u16 = 1 << 11;

#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub width: usize,
    pub height: usize,
    pub aspect: f32,
    pub pixels: Vec<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct AudioBatch {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

pub trait Backend {
    fn frame_hz(&self) -> f64 {
        60.0
    }
    fn run_frame(&mut self);
    fn frame(&self) -> Option<&Frame>;
    fn set_buttons(&mut self, port: usize, buttons: u16);
    fn take_audio(&mut self) -> Option<AudioBatch>;
    fn save_state(&mut self) -> Option<Vec<u8>>;
    fn load_state(&mut self, data: &[u8]) -> bool;
    fn sram(&mut self) -> Option<Vec<u8>>;
    fn set_sram(&mut self, data: &[u8]);
    fn reset(&mut self);
}
