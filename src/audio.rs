//! Deterministic generated sound effects and the optional Quasar audio host.
//!
//! Every effect is a PCM `quasar_audio::Clip` created at startup, so the
//! release needs no audio assets and the same synthesis runs in tests. The
//! native output device is opened through `quasar_audio::AudioEngine` when the
//! `audio` feature is enabled; absence of a device is never fatal.

use quasar_audio::Clip;

pub const SAMPLE_RATE: u32 = 48_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    MenuMove,
    MenuSelect,
    Paddle,
    Wall,
    Score,
    Win,
    Thrust,
    Explosion,
    Laser,
    Landing,
    Crash,
    ExtraLife,
    GameOver,
    Hyperspace,
}

pub struct SoundBank {
    menu_move: Clip,
    menu_select: Clip,
    paddle: Clip,
    wall: Clip,
    score: Clip,
    win: Clip,
    thrust: Clip,
    explosion: Clip,
    laser: Clip,
    landing: Clip,
    crash: Clip,
    extra_life: Clip,
    game_over: Clip,
    hyperspace: Clip,
}

impl SoundBank {
    pub fn generate() -> Self {
        Self {
            menu_move: tone(430.0, 430.0, 0.045, 0.16),
            menu_select: tone(620.0, 880.0, 0.09, 0.20),
            paddle: tone(280.0, 340.0, 0.055, 0.22),
            wall: tone(170.0, 150.0, 0.035, 0.14),
            score: tone(420.0, 640.0, 0.18, 0.22),
            win: tone(660.0, 1320.0, 0.42, 0.24),
            thrust: noise(0.055, 0.16, 11),
            explosion: noise(0.34, 0.28, 29),
            laser: tone(980.0, 220.0, 0.085, 0.17),
            landing: tone(300.0, 720.0, 0.32, 0.25),
            crash: noise(0.55, 0.34, 71),
            extra_life: tone(780.0, 1560.0, 0.24, 0.20),
            game_over: tone(180.0, 70.0, 0.70, 0.24),
            hyperspace: tone(180.0, 1400.0, 0.28, 0.16),
        }
    }

    pub fn clip(&self, sound: Sound) -> &Clip {
        match sound {
            Sound::MenuMove => &self.menu_move,
            Sound::MenuSelect => &self.menu_select,
            Sound::Paddle => &self.paddle,
            Sound::Wall => &self.wall,
            Sound::Score => &self.score,
            Sound::Win => &self.win,
            Sound::Thrust => &self.thrust,
            Sound::Explosion => &self.explosion,
            Sound::Laser => &self.laser,
            Sound::Landing => &self.landing,
            Sound::Crash => &self.crash,
            Sound::ExtraLife => &self.extra_life,
            Sound::GameOver => &self.game_over,
            Sound::Hyperspace => &self.hyperspace,
        }
    }

    pub fn gain(sound: Sound) -> f32 {
        match sound {
            Sound::Thrust => 0.42,
            Sound::Wall => 0.55,
            Sound::Explosion | Sound::Crash => 0.70,
            _ => 0.80,
        }
    }
}

fn envelope(frame: usize, frames: usize) -> f32 {
    let position = frame as f32 / frames.max(1) as f32;
    // Short attack, smooth square-ish decay.
    let attack = (position / 0.04).min(1.0);
    attack * (1.0 - position).powf(1.8)
}

fn tone(start_hz: f32, end_hz: f32, seconds: f32, gain: f32) -> Clip {
    let frames = (SAMPLE_RATE as f32 * seconds).round().max(1.0) as usize;
    let mut samples = Vec::with_capacity(frames * 2);
    let mut phase = 0.0f32;
    for frame in 0..frames {
        let position = frame as f32 / frames as f32;
        let frequency = start_hz + (end_hz - start_hz) * position;
        phase += frequency * std::f32::consts::TAU / SAMPLE_RATE as f32;
        let value = (phase.sin() + 0.18 * (phase * 2.0).sin()) * gain * envelope(frame, frames);
        samples.push(value);
        samples.push(value);
    }
    Clip::from_samples(samples, 2, SAMPLE_RATE, "classicclassics-tone")
        .expect("generated tone is valid")
}

fn noise(seconds: f32, gain: f32, seed: u32) -> Clip {
    let frames = (SAMPLE_RATE as f32 * seconds).round().max(1.0) as usize;
    let mut samples = Vec::with_capacity(frames * 2);
    let mut state = seed.wrapping_mul(2_654_435_761) | 1;
    let mut low = 0.0f32;
    for frame in 0..frames {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        let white = (state as f32 / u32::MAX as f32) * 2.0 - 1.0;
        low = low * 0.72 + white * 0.28;
        let value = (low * 0.8 + white * 0.2) * gain * envelope(frame, frames);
        samples.push(value);
        samples.push(value);
    }
    Clip::from_samples(samples, 2, SAMPLE_RATE, "classicclassics-noise")
        .expect("generated noise is valid")
}

#[cfg(feature = "audio")]
pub struct Audio {
    engine: quasar_audio::AudioEngine,
    bus: quasar_audio::BusId,
    muted: bool,
}

#[cfg(feature = "audio")]
impl Audio {
    pub fn open(enabled: bool, volume: f32, device: Option<&str>) -> Option<Self> {
        if !enabled {
            return None;
        }
        let config = quasar_audio::EngineConfig {
            sample_rate: Some(SAMPLE_RATE),
            channels: Some(2),
            max_voices: 32,
            ..Default::default()
        };
        let engine = match device {
            Some(name) => quasar_audio::AudioEngine::open(Some(name), config)
                .map_err(|error| {
                    eprintln!("Cannot open audio device '{name}' ({error}); continuing silently")
                })
                .ok()?,
            None => quasar_audio::AudioEngine::try_open(config)?,
        };
        let bus = engine.create_bus("effects", 0.9, 24).ok()?;
        let _ = engine.set_master_volume(volume.clamp(0.0, 1.0));
        Some(Self {
            engine,
            bus,
            muted: false,
        })
    }

    pub fn play(&self, sound: Sound, bank: &SoundBank, pan: f32) {
        if self.muted {
            return;
        }
        let _ = self.engine.play(
            bank.clip(sound).clone(),
            quasar_audio::PlaybackConfig {
                bus: self.bus,
                volume: SoundBank::gain(sound),
                pan: pan.clamp(-1.0, 1.0),
                ..Default::default()
            },
        );
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    pub fn muted(&self) -> bool {
        self.muted
    }

    pub fn set_volume(&self, volume: f32) {
        let _ = self.engine.set_master_volume(volume.clamp(0.0, 1.0));
    }
}

#[cfg(not(feature = "audio"))]
pub struct Audio;

#[cfg(not(feature = "audio"))]
impl Audio {
    pub fn open(_: bool, _: f32, _: Option<&str>) -> Option<Self> {
        None
    }
    pub fn play(&self, _: Sound, _: &SoundBank, _: f32) {}
    pub fn set_muted(&mut self, _: bool) {}
    pub fn muted(&self) -> bool {
        true
    }
    pub fn set_volume(&self, _: f32) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_effect_is_short_and_finite() {
        let bank = SoundBank::generate();
        for sound in [
            Sound::MenuMove,
            Sound::MenuSelect,
            Sound::Paddle,
            Sound::Wall,
            Sound::Score,
            Sound::Win,
            Sound::Thrust,
            Sound::Explosion,
            Sound::Laser,
            Sound::Landing,
            Sound::Crash,
            Sound::ExtraLife,
            Sound::GameOver,
            Sound::Hyperspace,
        ] {
            let clip = bank.clip(sound);
            assert!(clip.frames() > 0);
            assert!(clip.duration_seconds() < 1.0);
        }
    }
}
