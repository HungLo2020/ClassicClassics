use crate::{Backend, EmulatorSystem, backend::AudioBatch, picker::Picker};
use classicclassics_core::{
    ArcadeGame, ControlHints, EmulatorAudio, GameInfo, GameLoadContext, Input, RomEntry, Sound,
    imported::{FrameCache, PixelFrame, VideoClock},
    paint::{Color, Paint},
};
use quasar_render_2d::Canvas;
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    marker::PhantomData,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Record {
    pub rom_count: u32,
    pub last_rom: String,
    pub play_seconds: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct EmulatorGame<S: EmulatorSystem> {
    pub picker: Picker,
    pub roms: Vec<RomEntry>,
    pub rom: Option<RomEntry>,
    pub paused: bool,
    pub running: bool,
    pub save_dir: Option<PathBuf>,
    pub core_dir: Option<PathBuf>,
    pub save_enabled: bool,
    pub clock: VideoClock,
    pub pending: Input,
    pub ticks: u64,
    save_counter: u64,
    #[serde(skip)]
    audio: Option<EmulatorAudio>,
    #[serde(skip)]
    backend: Option<Box<dyn Backend>>,
    #[serde(skip)]
    frame_cache: FrameCache,
    #[serde(skip)]
    marker: PhantomData<S>,
}

impl<S: EmulatorSystem> Default for EmulatorGame<S> {
    fn default() -> Self {
        Self {
            picker: Picker::default(),
            roms: Vec::new(),
            rom: None,
            paused: false,
            running: false,
            save_dir: None,
            core_dir: None,
            save_enabled: true,
            clock: VideoClock::default(),
            pending: Input::default(),
            ticks: 0,
            save_counter: 0,
            audio: None,
            backend: None,
            frame_cache: FrameCache::default(),
            marker: PhantomData,
        }
    }
}

impl<S: EmulatorSystem> Clone for EmulatorGame<S> {
    fn clone(&self) -> Self {
        Self {
            picker: self.picker.clone(),
            roms: self.roms.clone(),
            rom: self.rom.clone(),
            paused: self.paused,
            running: self.running,
            save_dir: self.save_dir.clone(),
            core_dir: self.core_dir.clone(),
            save_enabled: self.save_enabled,
            clock: self.clock.clone(),
            pending: self.pending,
            ticks: self.ticks,
            save_counter: self.save_counter,
            audio: None,
            backend: None,
            frame_cache: FrameCache::default(),
            marker: PhantomData,
        }
    }
}

impl<S: EmulatorSystem> std::fmt::Debug for EmulatorGame<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmulatorGame")
            .field("system", &S::INFO.name)
            .field("roms", &self.roms.len())
            .field("rom", &self.rom.as_ref().map(|rom| &rom.name))
            .field("running", &self.running)
            .field("paused", &self.paused)
            .field("ticks", &self.ticks)
            .finish()
    }
}

impl<S: EmulatorSystem> EmulatorGame<S> {
    fn ensure_backend(&mut self) {
        if !self.running || self.backend.is_some() {
            return;
        }
        let Some(rom) = self.rom.clone() else {
            self.running = false;
            return;
        };
        let index = self
            .roms
            .iter()
            .position(|candidate| candidate.path == rom.path)
            .unwrap_or_else(|| {
                self.roms.push(rom);
                self.roms.len() - 1
            });
        if let Err(error) = self.start(index) {
            self.picker.error = Some(error);
            self.running = false;
        }
    }

    fn start(&mut self, index: usize) -> Result<(), String> {
        let entry = self
            .roms
            .get(index)
            .cloned()
            .ok_or_else(|| "no cartridge selected".to_owned())?;
        let bytes = std::fs::read(&entry.path)
            .map_err(|e| format!("cannot read {}: {e}", entry.path.display()))?;
        let mut backend = S::open(
            &bytes,
            &entry.path,
            self.save_dir.as_deref(),
            self.core_dir.as_deref(),
        )?;
        if self.save_enabled
            && let Some(path) = self.sram_path(&entry)
            && let Ok(data) = std::fs::read(path)
        {
            backend.set_sram(&data);
        }
        self.backend = Some(backend);
        self.rom = Some(entry);
        self.running = true;
        self.paused = false;
        self.clock = VideoClock::default();
        self.pending = Input::default();
        self.ticks = 0;
        self.save_counter = 0;
        self.frame_cache.invalidate();
        self.picker.error = None;
        Ok(())
    }

    fn system_dir(&self, rom: &RomEntry) -> Option<PathBuf> {
        self.save_dir.as_ref().map(|dir| dir.join(&rom.system))
    }

    fn sram_path(&self, rom: &RomEntry) -> Option<PathBuf> {
        self.system_dir(rom)
            .map(|dir| dir.join(format!("{:016x}.srm", rom.id)))
    }

    fn state_path(&self, rom: &RomEntry) -> Option<PathBuf> {
        self.system_dir(rom)
            .map(|dir| dir.join(format!("{:016x}.state", rom.id)))
    }

    fn save_sram(&mut self) {
        if !self.save_enabled {
            return;
        }
        let Some(rom) = self.rom.clone() else {
            return;
        };
        let Some(path) = self.sram_path(&rom) else {
            return;
        };
        let Some(backend) = self.backend.as_mut() else {
            return;
        };
        if let Some(data) = backend.sram() {
            let _ = atomic_write(&path, &data);
        }
    }

    pub fn state_supported(&self) -> bool {
        self.save_enabled && self.running && self.backend.is_some()
    }

    pub fn state_available(&self) -> bool {
        self.rom
            .as_ref()
            .and_then(|rom| self.state_path(rom))
            .is_some_and(|path| path.is_file())
    }

    pub fn save_quick_state(&mut self) -> Result<(), String> {
        if !self.state_supported() {
            return Err("save states are not available for this game".into());
        }
        let rom = self.rom.clone().ok_or("no cartridge loaded")?;
        let path = self
            .state_path(&rom)
            .ok_or("save-state directory unavailable")?;
        let backend = self.backend.as_mut().ok_or("emulator not running")?;
        let data = backend
            .save_state()
            .ok_or("this emulator could not create a save state")?;
        atomic_write(&path, &data).map_err(|error| format!("cannot write save state: {error}"))
    }

    pub fn load_quick_state(&mut self) -> Result<(), String> {
        if !self.state_supported() {
            return Err("save states are not available for this game".into());
        }
        let rom = self.rom.clone().ok_or("no cartridge loaded")?;
        let path = self
            .state_path(&rom)
            .ok_or("save-state directory unavailable")?;
        let data =
            std::fs::read(&path).map_err(|error| format!("cannot read save state: {error}"))?;
        let backend = self.backend.as_mut().ok_or("emulator not running")?;
        backend
            .load_state(&data)
            .then_some(())
            .ok_or_else(|| "the emulator rejected this save state".to_owned())
    }

    pub fn delete_quick_state(&mut self) -> Result<(), String> {
        if !self.save_enabled {
            return Err("save states are not available for this game".into());
        }
        let Some(rom) = self.rom.clone() else {
            return Err("no cartridge loaded".into());
        };
        let Some(path) = self.state_path(&rom) else {
            return Err("save-state directory unavailable".into());
        };
        std::fs::remove_file(&path).map_err(|error| format!("cannot delete save state: {error}"))
    }

    fn leave_running(&mut self) {
        self.save_sram();
        if let Some(rom) = &self.rom
            && let Some(index) = self
                .roms
                .iter()
                .position(|candidate| candidate.path == rom.path)
        {
            self.picker.selected = index;
        }
        self.running = false;
        self.paused = false;
        self.backend = None;
        self.audio = None;
        self.frame_cache.invalidate();
    }

    fn current_audio(&mut self) -> Option<EmulatorAudio> {
        let batch: AudioBatch = self.backend.as_mut()?.take_audio()?;
        Some(EmulatorAudio {
            sample_rate: batch.sample_rate,
            channels: batch.channels,
            samples: batch.samples,
        })
    }
}

impl<S: EmulatorSystem> ArcadeGame for EmulatorGame<S> {
    type Record = Record;
    const INFO: GameInfo = S::INFO;
    const ASSET_MANIFEST_DIR: Option<&'static str> = S::ASSET_MANIFEST_DIR;

    fn load_with(ctx: &GameLoadContext) -> Result<Self, String> {
        let mut game = Self {
            roms: ctx.roms.clone(),
            save_dir: ctx.save_dir.clone(),
            core_dir: ctx.core_dir.clone(),
            save_enabled: ctx.save_enabled,
            ..Self::default()
        };
        if let Some(initial) = &ctx.initial_rom {
            let index = game
                .roms
                .iter()
                .position(|rom| rom.path == initial.path)
                .unwrap_or_else(|| {
                    game.roms.push(initial.clone());
                    game.roms.len() - 1
                });
            game.picker.selected = index;
            game.start(index)?;
        }
        game.picker.clamp(game.roms.len());
        Ok(game)
    }

    fn paused(&self) -> bool {
        self.paused
    }

    fn back_to_picker(&mut self) -> bool {
        if !self.running {
            return false;
        }
        self.leave_running();
        // Every cartridge has its own catalog card, so Back returns straight
        // to the catalog instead of introducing a system submenu.
        false
    }

    fn take_audio(&mut self) -> Option<EmulatorAudio> {
        self.audio.take()
    }

    fn save(&mut self) {
        if self.running {
            self.save_sram();
        }
    }

    fn tick(&mut self, input: Input, sounds: &mut Vec<Sound>) {
        self.audio = None;
        if !self.running {
            self.ticks += 1;
            self.picker.clamp(self.roms.len());
            if input.menu_up {
                if !self.roms.is_empty() {
                    self.picker.selected =
                        (self.picker.selected + self.roms.len() - 1) % self.roms.len();
                }
                sounds.push(Sound::MenuMove);
            }
            if input.menu_down {
                if !self.roms.is_empty() {
                    self.picker.selected = (self.picker.selected + 1) % self.roms.len();
                }
                sounds.push(Sound::MenuMove);
            }
            self.picker.clamp(self.roms.len());
            if input.confirm {
                let index = self.picker.selected;
                if let Err(error) = self.start(index) {
                    self.picker.error = Some(error);
                } else {
                    sounds.push(Sound::MenuSelect);
                }
            }
            if input.restart {
                self.picker.error = None;
            }
            return;
        }
        if input.pause {
            self.paused = !self.paused;
            if self.paused {
                self.save_sram();
            }
            return;
        }
        if input.restart {
            if let Some(backend) = self.backend.as_mut() {
                backend.reset();
            }
            self.ticks = 0;
            return;
        }
        if self.paused {
            return;
        }
        self.ensure_backend();
        if self.backend.is_none() {
            return;
        }
        let mut next = input;
        next.pause = false;
        next.restart = false;
        self.pending.latch(next);
        let frame_hz = self
            .backend
            .as_ref()
            .map_or(S::FRAME_HZ, |backend| backend.frame_hz());
        if !self.clock.advance(frame_hz) {
            return;
        }
        let frame_input = std::mem::take(&mut self.pending);
        let buttons = S::buttons(&frame_input);
        if let Some(backend) = self.backend.as_mut() {
            backend.set_buttons(0, buttons[0]);
            backend.set_buttons(1, buttons[1]);
            backend.run_frame();
        }
        self.frame_cache.invalidate();
        self.audio = self.current_audio();
        self.ticks += 1;
        self.save_counter += 1;
        if self.save_counter >= (frame_hz * 60.0) as u64 {
            self.save_sram();
            self.save_counter = 0;
        }
    }

    fn autoplay(&self) -> Input {
        if self.running {
            Input {
                fire: true,
                jump: self.ticks % 90 < 45,
                move_x: if self.ticks % 240 < 120 { 1 } else { -1 },
                confirm: S::autoplay_confirm(self.ticks),
                ..Default::default()
            }
        } else {
            Input {
                confirm: self.ticks.is_multiple_of(90),
                ..Default::default()
            }
        }
    }

    fn scene(&self, hints: &ControlHints) -> Canvas {
        if self.running
            && let Some(frame) = self.pixel_frame(hints)
        {
            return frame.reference_canvas();
        }
        self.picker
            .scene(&S::INFO, &self.roms, hints, &S::controls_help(hints))
    }

    fn pixel_frame(&self, hints: &ControlHints) -> Option<PixelFrame> {
        if !self.running {
            return None;
        }
        let frame = self.backend.as_ref()?.frame()?;
        if frame.pixels.is_empty() {
            return None;
        }
        let image = self.frame_cache.image(|| {
            (
                frame.pixels.clone(),
                frame.width.max(1),
                frame.height.max(1),
            )
        });
        let mut help = S::controls_help(hints);
        if self.paused {
            help.push_str("   PAUSED");
        }
        Some(PixelFrame {
            image,
            aspect: S::ASPECT,
            help,
        })
    }

    fn record(&mut self, record: &mut Record) {
        record.rom_count = self.roms.len() as u32;
        record.play_seconds = self.ticks / S::FRAME_HZ.max(1.0).round() as u64;
        if let Some(rom) = &self.rom {
            record.last_rom.clone_from(&rom.name);
        }
    }

    fn summary(&self) -> String {
        format!(
            "system={} running={} rom={} paused={} frame={} fps={:.3} audio={}",
            S::INFO.aliases[0],
            self.running,
            self.rom.as_ref().map_or("-", |rom| rom.name.as_str()),
            self.paused,
            self.ticks,
            self.backend
                .as_ref()
                .map_or(S::FRAME_HZ, |backend| backend.frame_hz()),
            self.audio.as_ref().map_or(0, |audio| audio.samples.len())
        )
    }

    fn catalog_record(record: &Record) -> (&'static str, String) {
        ("ROM LIBRARY", format!("{} GAMES", record.rom_count))
    }

    fn motif(paint: &mut Paint, cx: f32, cy: f32, accent: Color) {
        S::motif(paint, cx, cy, accent);
    }
}

fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| value.subsec_nanos());
    let temporary = path.with_extension(format!("tmp-{}-{nanos}", std::process::id()));
    {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(data)?;
        file.sync_all()?;
    }
    std::fs::rename(temporary, path)
}
