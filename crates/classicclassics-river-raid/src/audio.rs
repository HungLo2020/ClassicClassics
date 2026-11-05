//! River Raid synthesis preserved from the standalone backup; playback is hosted by Quasar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    Shot,
    Explosion,
    Refuel,
    FullTank,
    ExtraLife,
    LowFuel,
    GameOver,
}

struct Voice {
    sound: Sound,
    age: f32,
    duration: f32,
    phase: f32,
}

pub struct Synth {
    voices: Vec<Voice>,
    noise: u16,
    noise_phase: f32,
    noise_value: f32,
    engine: Option<u8>,
}

impl Default for Synth {
    fn default() -> Self {
        Self {
            voices: Vec::with_capacity(16),
            noise: 0,
            noise_phase: 0.0,
            noise_value: 0.0,
            engine: None,
        }
    }
}

impl Synth {
    pub fn trigger(&mut self, sound: Sound) {
        if self.voices.len() == 16 {
            self.voices.remove(0);
        }
        let duration = match sound {
            Sound::Shot => 15.0 / 60.0,
            Sound::Explosion => 31.0 / 60.0,
            Sound::Refuel | Sound::FullTank => 8.0 / 60.0,
            Sound::ExtraLife => 0.18,
            Sound::LowFuel => 0.12,
            Sound::GameOver => 35.0 / 60.0,
        };
        self.voices.push(Voice {
            sound,
            age: 0.0,
            duration,
            phase: 0.0,
        });
    }
    pub fn clear(&mut self) {
        self.voices.clear();
    }
    pub fn set_engine(&mut self, speed: Option<u8>) {
        self.engine = speed;
    }
    pub fn sample(&mut self, sample_rate: f32) -> f32 {
        let dt = 1.0 / sample_rate;
        let divider = self.engine.map_or(9, |speed| 31 - (speed >> 4)).max(1);
        self.noise_phase += 31_400.0 / f32::from(divider) * dt;
        if self.noise_phase >= 1.0 {
            self.noise_phase -= 1.0;
            if self.noise == 0 {
                self.noise = 0x1ff;
            }
            let feedback = (self.noise ^ (self.noise >> 4)) & 1;
            self.noise = (self.noise >> 1) | (feedback << 8);
            self.noise_value = if self.noise & 1 == 0 { -1.0 } else { 1.0 };
        }
        let mut mixed = if self.engine.is_some() {
            self.noise_value * 0.055
        } else {
            0.0
        };
        for voice in &mut self.voices {
            let fraction = voice.age / voice.duration;
            let envelope = (1.0 - fraction).max(0.0);
            let (frequency, noise_mix, gain) = match voice.sound {
                Sound::Shot => (31_400.0 / (6.0 * (14.0 + 15.0 * fraction)), 0.0, 0.13),
                Sound::Explosion => (31_400.0 / 29.0, 1.0, 0.25),
                Sound::Refuel => (31_400.0 / 64.0, 0.0, 0.13),
                Sound::FullTank => (31_400.0 / 32.0, 0.0, 0.13),
                Sound::ExtraLife => (1100.0 + 500.0 * fraction, 0.0, 0.10),
                Sound::LowFuel => (31_400.0 / (6.0 * (15.0 + 15.0 * fraction)), 0.0, 0.14),
                Sound::GameOver => (31_400.0 / 9.0, 1.0, 0.20),
            };
            voice.phase = (voice.phase + frequency * dt).fract();
            let square = if voice.phase < 0.5 { -1.0 } else { 1.0 };
            mixed += (square * (1.0 - noise_mix) + self.noise_value * noise_mix) * envelope * gain;
            voice.age += dt;
        }
        self.voices.retain(|v| v.age < v.duration);
        mixed.clamp(-0.85, 0.85)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restored_synth_has_speed_dependent_engine_and_finite_effects() {
        let mut slow = Synth::default();
        let mut fast = Synth::default();
        slow.set_engine(Some(64));
        fast.set_engine(Some(254));
        let a: Vec<_> = (0..1000).map(|_| slow.sample(48000.0)).collect();
        let b: Vec<_> = (0..1000).map(|_| fast.sample(48000.0)).collect();
        assert_ne!(a, b);
        assert!(a.iter().any(|v| v.abs() > 0.01));
        let mut synth = Synth::default();
        synth.trigger(Sound::Explosion);
        assert!((0..48000).all(|_| synth.sample(48000.0).abs() <= 0.85));
        assert_eq!(synth.sample(48000.0), 0.0);
    }
}
