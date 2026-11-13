//! Minimal, synchronous libretro frontend for the pinned C cores.
//!
//! Only the parts the four systems need are implemented: software video,
//! signed-16-bit audio, two joypads, SRAM and save states. Everything else
//! answers "unsupported" so cores fall back to their defaults.

use crate::{AudioBatch, Backend, Frame};
use libloading::{Library, Symbol};
use std::cell::Cell;
use std::ffi::{CStr, CString, c_char, c_void};
use std::path::{Path, PathBuf};
use std::ptr;

const RETRO_DEVICE_JOYPAD: u32 = 1;
const RETRO_MEMORY_SAVE_RAM: u32 = 0;
const RETRO_PIXEL_FORMAT_0RGB1555: u32 = 0;
const RETRO_PIXEL_FORMAT_XRGB8888: u32 = 1;
const RETRO_PIXEL_FORMAT_RGB565: u32 = 2;

#[repr(C)]
struct RetroGameInfo {
    path: *const c_char,
    data: *const c_void,
    size: usize,
    meta: *const c_char,
}

#[repr(C)]
struct RetroSystemInfo {
    library_name: *const c_char,
    library_version: *const c_char,
    valid_extensions: *const c_char,
    need_fullpath: bool,
    block_extract: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RetroGameGeometry {
    base_width: u32,
    base_height: u32,
    max_width: u32,
    max_height: u32,
    aspect_ratio: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RetroSystemTiming {
    fps: f64,
    sample_rate: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RetroSystemAvInfo {
    geometry: RetroGameGeometry,
    timing: RetroSystemTiming,
}

#[repr(C)]
struct RetroVariable {
    key: *const c_char,
    value: *const c_char,
}

type EnvironmentFn = unsafe extern "C" fn(u32, *mut c_void) -> bool;
type VideoFn = unsafe extern "C" fn(*const c_void, u32, u32, usize);
type AudioFn = unsafe extern "C" fn(i16, i16);
type AudioBatchFn = unsafe extern "C" fn(*const i16, usize) -> usize;
type InputPollFn = unsafe extern "C" fn();
type InputStateFn = unsafe extern "C" fn(u32, u32, u32, u32) -> i16;

struct CoreState {
    core_path: CString,
    save_dir: CString,
    pixel_format: u32,
    buttons: [u16; 2],
    frame: Frame,
    audio: Vec<f32>,
    sample_rate: u32,
    fps: f64,
}

thread_local! {
    static ACTIVE: Cell<*mut CoreState> = const { Cell::new(ptr::null_mut()) };
}

unsafe fn active() -> Option<&'static mut CoreState> {
    let ptr = ACTIVE.with(Cell::get);
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { &mut *ptr })
    }
}

unsafe extern "C" fn environment(cmd: u32, data: *mut c_void) -> bool {
    let Some(state) = (unsafe { active() }) else {
        return false;
    };
    const EXPERIMENTAL: u32 = 0x10000;
    let base = cmd & !EXPERIMENTAL;
    if data.is_null() {
        return matches!(
            base,
            2 | 3 | 39 | 47 | 50 | 51 | 57 | 59 | 61 | 72 | 73 | 77
        );
    }
    unsafe {
        match base {
            1 => true, // SET_ROTATION
            2 => {
                *(data as *mut bool) = true;
                true
            } // GET_OVERSCAN
            3 => {
                *(data as *mut bool) = true;
                true
            } // GET_CAN_DUPE
            6 | 60 => true, // SET_MESSAGE / SET_MESSAGE_EXT
            7 | 8 => true, // SHUTDOWN / SET_PERFORMANCE_LEVEL
            9 | 30 | 31 | 79 | 80 => {
                *(data as *mut *const c_char) = state.save_dir.as_ptr();
                true
            } // system/assets/save/playlist/browser directories
            10 => {
                state.pixel_format = *(data as *const u32);
                true
            } // SET_PIXEL_FORMAT
            11 | 12 | 13 | 18 | 35 | 36 | 42 | 44 | 53 | 54 | 55 | 58 | 63 | 67 | 68 | 69 => true, // input/controller/options metadata
            14 | 41 | 43 => false, // hardware render
            15 => {
                let variable = &mut *(data as *mut RetroVariable);
                variable.value = ptr::null();
                false
            } // GET_VARIABLE
            16 | 70 => true,       // SET_VARIABLES / SET_VARIABLE
            17 => false,           // GET_VARIABLE_UPDATE
            19 => {
                *(data as *mut *const c_char) = state.core_path.as_ptr();
                true
            } // GET_LIBRETRO_PATH
            38 | 66 | 74 => false, // username/game-info/microphone
            21 | 22 | 62 => false, // frame/audio callbacks
            23 | 25 | 26 | 27 | 28 | 29 | 46 | 48 => false, // optional interfaces
            32 => {
                let av = *(data as *const RetroSystemAvInfo);
                update_av(state, av);
                true
            }
            33 => true, // SET_PROC_ADDRESS_CALLBACK
            37 => {
                let geometry = *(data as *const RetroGameGeometry);
                update_geometry(state, geometry);
                true
            }
            39 => {
                *(data as *mut u32) = 0;
                true
            } // GET_LANGUAGE
            40 => false, // GET_CURRENT_SOFTWARE_FRAMEBUFFER
            45 => false, // GET_VFS_INTERFACE
            47 => {
                *(data as *mut i32) = 3;
                true
            } // GET_AUDIO_VIDEO_ENABLE
            49 => false, // GET_FASTFORWARDING
            50 => {
                *(data as *mut f32) = 60.0;
                true
            } // GET_TARGET_REFRESH_RATE
            51 => {
                if !data.is_null() {
                    *(data as *mut bool) = true;
                }
                true
            } // GET_INPUT_BITMASKS
            52 => false, // GET_CORE_OPTIONS_VERSION (legacy SET_VARIABLES)
            56 => false, // GET_PREFERRED_HW_RENDER
            57 => {
                *(data as *mut u32) = 0;
                true
            } // GET_DISK_CONTROL_INTERFACE_VERSION
            59 => {
                *(data as *mut u32) = 1;
                true
            } // GET_MESSAGE_INTERFACE_VERSION
            61 => {
                *(data as *mut u32) = 2;
                true
            } // GET_INPUT_MAX_USERS
            64 => true,  // SET_FASTFORWARDING_OVERRIDE
            71 => false, // GET_THROTTLE_STATE
            72 => {
                *(data as *mut u32) = 0;
                true
            } // GET_SAVESTATE_CONTEXT
            73 => {
                *(data as *mut bool) = false;
                true
            } // GET_JIT_CAPABLE
            77 => {
                *(data as *mut i32) = 100;
                true
            } // GET_DEVICE_POWER
            _ => false,
        }
    }
}

unsafe fn update_geometry(state: &mut CoreState, geometry: RetroGameGeometry) {
    state.frame.width = geometry.base_width.max(1) as usize;
    state.frame.height = geometry.base_height.max(1) as usize;
    state.frame.aspect = if geometry.aspect_ratio > 0.0 {
        geometry.aspect_ratio
    } else {
        state.frame.width as f32 / state.frame.height.max(1) as f32
    };
}

unsafe fn update_av(state: &mut CoreState, av: RetroSystemAvInfo) {
    unsafe { update_geometry(state, av.geometry) };
    if av.timing.sample_rate > 0.0 {
        state.sample_rate = av.timing.sample_rate.round() as u32;
    }
    if av.timing.fps > 1.0 {
        state.fps = av.timing.fps;
    }
}

unsafe extern "C" fn video(data: *const c_void, width: u32, height: u32, pitch: usize) {
    let Some(state) = (unsafe { active() }) else {
        return;
    };
    if data.is_null() || width == 0 || height == 0 {
        return;
    }
    let width = width as usize;
    let height = height as usize;
    state.frame.width = width;
    state.frame.height = height;
    state.frame.pixels.clear();
    state.frame.pixels.reserve(width * height);
    unsafe {
        match state.pixel_format {
            RETRO_PIXEL_FORMAT_XRGB8888 => {
                for y in 0..height {
                    let row = (data as *const u32).add(y * pitch / 4);
                    for x in 0..width {
                        let p = *row.add(x);
                        state.frame.pixels.push(p & 0x00FF_FFFF);
                    }
                }
            }
            RETRO_PIXEL_FORMAT_RGB565 => {
                for y in 0..height {
                    let row = (data as *const u16).add(y * pitch / 2);
                    for x in 0..width {
                        let p = *row.add(x);
                        let r = ((p >> 11) & 0x1F) as u32;
                        let g = ((p >> 5) & 0x3F) as u32;
                        let b = (p & 0x1F) as u32;
                        state
                            .frame
                            .pixels
                            .push(((r * 255 / 31) << 16) | ((g * 255 / 63) << 8) | (b * 255 / 31));
                    }
                }
            }
            _ => {
                for y in 0..height {
                    let row = (data as *const u16).add(y * pitch / 2);
                    for x in 0..width {
                        let p = *row.add(x);
                        let r = ((p >> 10) & 0x1F) as u32;
                        let g = ((p >> 5) & 0x1F) as u32;
                        let b = (p & 0x1F) as u32;
                        state
                            .frame
                            .pixels
                            .push(((r * 255 / 31) << 16) | ((g * 255 / 31) << 8) | (b * 255 / 31));
                    }
                }
            }
        }
    }
}

unsafe extern "C" fn audio_sample(left: i16, right: i16) {
    if let Some(state) = unsafe { active() } {
        state.audio.push(f32::from(left) / 32768.0);
        state.audio.push(f32::from(right) / 32768.0);
    }
}

unsafe extern "C" fn audio_batch(data: *const i16, frames: usize) -> usize {
    let Some(state) = (unsafe { active() }) else {
        return 0;
    };
    if data.is_null() {
        return 0;
    }
    state.audio.reserve(frames * 2);
    for i in 0..frames * 2 {
        state
            .audio
            .push(f32::from(unsafe { *data.add(i) }) / 32768.0);
    }
    frames
}

unsafe extern "C" fn input_poll() {}

unsafe extern "C" fn input_state(port: u32, device: u32, _index: u32, id: u32) -> i16 {
    if device != RETRO_DEVICE_JOYPAD {
        return 0;
    }
    let Some(state) = (unsafe { active() }) else {
        return 0;
    };
    let Some(buttons) = state.buttons.get(port as usize) else {
        return 0;
    };
    if id == 256 {
        return *buttons as i16;
    }
    if id >= 16 {
        return 0;
    }
    i16::from(buttons & (1u16 << id) != 0)
}

struct Symbols {
    retro_init: unsafe extern "C" fn(),
    retro_deinit: unsafe extern "C" fn(),
    retro_get_system_info: unsafe extern "C" fn(*mut RetroSystemInfo),
    retro_get_system_av_info: unsafe extern "C" fn(*mut RetroSystemAvInfo),
    retro_set_environment: unsafe extern "C" fn(EnvironmentFn),
    retro_set_video_refresh: unsafe extern "C" fn(VideoFn),
    retro_set_audio_sample: unsafe extern "C" fn(AudioFn),
    retro_set_audio_sample_batch: unsafe extern "C" fn(AudioBatchFn),
    retro_set_input_poll: unsafe extern "C" fn(InputPollFn),
    retro_set_input_state: unsafe extern "C" fn(InputStateFn),
    retro_load_game: unsafe extern "C" fn(*const RetroGameInfo) -> bool,
    retro_unload_game: unsafe extern "C" fn(),
    retro_run: unsafe extern "C" fn(),
    retro_reset: unsafe extern "C" fn(),
    retro_serialize_size: unsafe extern "C" fn() -> usize,
    retro_serialize: unsafe extern "C" fn(*mut c_void, usize) -> bool,
    retro_unserialize: unsafe extern "C" fn(*const c_void, usize) -> bool,
    retro_get_memory_data: unsafe extern "C" fn(u32) -> *mut c_void,
    retro_get_memory_size: unsafe extern "C" fn(u32) -> usize,
}

unsafe fn load_symbol<T: Copy>(library: &Library, name: &[u8]) -> Result<T, String> {
    let symbol: Symbol<T> = unsafe { library.get(name) }
        .map_err(|e| format!("core is missing {}: {e}", String::from_utf8_lossy(name)))?;
    Ok(*symbol)
}

impl Symbols {
    unsafe fn new(library: &Library) -> Result<Self, String> {
        Ok(Self {
            retro_init: unsafe { load_symbol(library, b"retro_init\0") }?,
            retro_deinit: unsafe { load_symbol(library, b"retro_deinit\0") }?,
            retro_get_system_info: unsafe { load_symbol(library, b"retro_get_system_info\0") }?,
            retro_get_system_av_info: unsafe {
                load_symbol(library, b"retro_get_system_av_info\0")
            }?,
            retro_set_environment: unsafe { load_symbol(library, b"retro_set_environment\0") }?,
            retro_set_video_refresh: unsafe { load_symbol(library, b"retro_set_video_refresh\0") }?,
            retro_set_audio_sample: unsafe { load_symbol(library, b"retro_set_audio_sample\0") }?,
            retro_set_audio_sample_batch: unsafe {
                load_symbol(library, b"retro_set_audio_sample_batch\0")
            }?,
            retro_set_input_poll: unsafe { load_symbol(library, b"retro_set_input_poll\0") }?,
            retro_set_input_state: unsafe { load_symbol(library, b"retro_set_input_state\0") }?,
            retro_load_game: unsafe { load_symbol(library, b"retro_load_game\0") }?,
            retro_unload_game: unsafe { load_symbol(library, b"retro_unload_game\0") }?,
            retro_run: unsafe { load_symbol(library, b"retro_run\0") }?,
            retro_reset: unsafe { load_symbol(library, b"retro_reset\0") }?,
            retro_serialize_size: unsafe { load_symbol(library, b"retro_serialize_size\0") }?,
            retro_serialize: unsafe { load_symbol(library, b"retro_serialize\0") }?,
            retro_unserialize: unsafe { load_symbol(library, b"retro_unserialize\0") }?,
            retro_get_memory_data: unsafe { load_symbol(library, b"retro_get_memory_data\0") }?,
            retro_get_memory_size: unsafe { load_symbol(library, b"retro_get_memory_size\0") }?,
        })
    }
}

pub struct LibretroBackend {
    state: Box<CoreState>,
    _library: Library,
    symbols: Symbols,
    loaded: bool,
}

impl LibretroBackend {
    fn set_core_state(&self) {
        ACTIVE.with(|cell| cell.set(&*self.state as *const CoreState as *mut CoreState));
    }
}

impl Drop for LibretroBackend {
    fn drop(&mut self) {
        self.set_core_state();
        unsafe {
            if self.loaded {
                (self.symbols.retro_unload_game)();
            }
            (self.symbols.retro_deinit)();
        }
        ACTIVE.with(|cell| cell.set(ptr::null_mut()));
    }
}

pub fn open(
    core_file: &str,
    rom: &[u8],
    path: &Path,
    save_dir: Option<&Path>,
    core_dir: Option<&Path>,
) -> Result<Box<dyn Backend>, String> {
    let core_path: PathBuf = core_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("app/cores"))
        .join(core_file);
    if !core_path.is_file() {
        return Err(format!(
            "emulator core missing: {} (expected under app/cores)",
            core_path.display()
        ));
    }
    let library = unsafe { Library::new(&core_path) }
        .map_err(|e| format!("cannot load {}: {e}", core_path.display()))?;
    let symbols = unsafe { Symbols::new(&library) }?;

    let core_dir_string = core_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .display()
        .to_string();
    let save_dir_string = save_dir
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| ".".into());
    let state = Box::new(CoreState {
        core_path: CString::new(core_dir_string).unwrap_or_default(),
        save_dir: CString::new(save_dir_string).unwrap_or_default(),
        pixel_format: RETRO_PIXEL_FORMAT_0RGB1555,
        buttons: [0; 2],
        frame: Frame::default(),
        audio: Vec::new(),
        sample_rate: 48_000,
        fps: 60.0,
    });
    let backend = LibretroBackend {
        state,
        _library: library,
        symbols,
        loaded: false,
    };
    backend.set_core_state();

    unsafe {
        (backend.symbols.retro_set_environment)(environment);
        (backend.symbols.retro_set_video_refresh)(video);
        (backend.symbols.retro_set_audio_sample)(audio_sample);
        (backend.symbols.retro_set_audio_sample_batch)(audio_batch);
        (backend.symbols.retro_set_input_poll)(input_poll);
        (backend.symbols.retro_set_input_state)(input_state);
        (backend.symbols.retro_init)();
    }
    let mut backend = backend;

    let mut system_info = RetroSystemInfo {
        library_name: ptr::null(),
        library_version: ptr::null(),
        valid_extensions: ptr::null(),
        need_fullpath: false,
        block_extract: true,
    };
    unsafe { (backend.symbols.retro_get_system_info)(&mut system_info) };
    let c_path = CString::new(path.display().to_string()).unwrap_or_default();
    let info = RetroGameInfo {
        path: c_path.as_ptr(),
        data: if system_info.need_fullpath {
            ptr::null()
        } else {
            rom.as_ptr().cast()
        },
        size: if system_info.need_fullpath {
            0
        } else {
            rom.len()
        },
        meta: ptr::null(),
    };
    let loaded = unsafe { (backend.symbols.retro_load_game)(&info) };
    if !loaded {
        let name = if system_info.library_name.is_null() {
            core_file.to_owned()
        } else {
            unsafe { CStr::from_ptr(system_info.library_name) }
                .to_string_lossy()
                .into_owned()
        };
        return Err(format!("{name} rejected this ROM"));
    }
    backend.loaded = true;
    let mut av = RetroSystemAvInfo::default();
    unsafe { (backend.symbols.retro_get_system_av_info)(&mut av) };
    unsafe { update_av(&mut backend.state, av) };
    if backend.state.frame.pixels.is_empty() {
        backend.state.frame.pixels = vec![0; 256 * 240];
        backend.state.frame.width = 256;
        backend.state.frame.height = 240;
    }
    Ok(Box::new(backend))
}

impl Backend for LibretroBackend {
    fn frame_hz(&self) -> f64 {
        self.state.fps
    }

    fn run_frame(&mut self) {
        self.set_core_state();
        self.state.audio.clear();
        unsafe { (self.symbols.retro_run)() };
    }

    fn frame(&self) -> Option<&Frame> {
        Some(&self.state.frame)
    }

    fn set_buttons(&mut self, port: usize, buttons: u16) {
        if let Some(slot) = self.state.buttons.get_mut(port) {
            *slot = buttons;
        }
    }

    fn take_audio(&mut self) -> Option<AudioBatch> {
        if self.state.audio.is_empty() {
            return None;
        }
        Some(AudioBatch {
            sample_rate: self.state.sample_rate,
            channels: 2,
            samples: std::mem::take(&mut self.state.audio),
        })
    }

    fn save_state(&mut self) -> Option<Vec<u8>> {
        self.set_core_state();
        let size = unsafe { (self.symbols.retro_serialize_size)() };
        if size == 0 {
            return None;
        }
        let mut data = vec![0u8; size];
        let ok = unsafe { (self.symbols.retro_serialize)(data.as_mut_ptr().cast(), data.len()) };
        ok.then_some(data)
    }

    fn load_state(&mut self, data: &[u8]) -> bool {
        self.set_core_state();
        unsafe { (self.symbols.retro_unserialize)(data.as_ptr().cast(), data.len()) }
    }

    fn sram(&mut self) -> Option<Vec<u8>> {
        self.set_core_state();
        let size = unsafe { (self.symbols.retro_get_memory_size)(RETRO_MEMORY_SAVE_RAM) };
        let data = unsafe { (self.symbols.retro_get_memory_data)(RETRO_MEMORY_SAVE_RAM) };
        if size == 0 || data.is_null() {
            return None;
        }
        Some(unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size) }.to_vec())
    }

    fn set_sram(&mut self, data: &[u8]) {
        self.set_core_state();
        let size = unsafe { (self.symbols.retro_get_memory_size)(RETRO_MEMORY_SAVE_RAM) };
        let target = unsafe { (self.symbols.retro_get_memory_data)(RETRO_MEMORY_SAVE_RAM) };
        if target.is_null() || size == 0 {
            return;
        }
        let count = size.min(data.len());
        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr(), target.cast::<u8>(), count);
        }
    }

    fn reset(&mut self) {
        self.set_core_state();
        unsafe { (self.symbols.retro_reset)() };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BTN_A, BTN_LEFT};

    #[test]
    fn joypad_queries_and_the_bitmask_see_the_same_buttons() {
        let mut state = Box::new(CoreState {
            core_path: CString::new("").unwrap(),
            save_dir: CString::new("").unwrap(),
            pixel_format: RETRO_PIXEL_FORMAT_0RGB1555,
            buttons: [BTN_A | BTN_LEFT, 0],
            frame: Frame::default(),
            audio: Vec::new(),
            sample_rate: 48_000,
            fps: 60.0,
        });
        ACTIVE.with(|cell| cell.set(&mut *state as *mut CoreState));
        assert_eq!(unsafe { input_state(0, RETRO_DEVICE_JOYPAD, 0, 8) }, 1);
        assert_eq!(unsafe { input_state(0, RETRO_DEVICE_JOYPAD, 0, 6) }, 1);
        assert_eq!(
            unsafe { input_state(0, RETRO_DEVICE_JOYPAD, 0, 256) },
            (BTN_A | BTN_LEFT) as i16
        );
        assert_eq!(unsafe { input_state(1, RETRO_DEVICE_JOYPAD, 0, 8) }, 0);
        ACTIVE.with(|cell| cell.set(ptr::null_mut()));
    }
}
