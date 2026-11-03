//! Small renderer-independent animations driven by `quasar-animation`.

use quasar_animation::{Clip, Interpolation, Keyframe, LoopMode, Player, Track, Value};
use std::sync::Arc;

/// A gentle two-second breathing scale used by the menu logo.
pub struct LogoPulse {
    player: Player,
}

impl LogoPulse {
    pub fn new() -> Self {
        let track = Track::new(
            "scale",
            Interpolation::Smooth,
            vec![
                Keyframe::new(0.0, Value::Float(1.0)),
                Keyframe::new(1.0, Value::Float(1.028)),
                Keyframe::new(2.0, Value::Float(1.0)),
            ],
        )
        .expect("valid pulse track");
        let clip = Arc::new(
            Clip::new("logo-pulse", 2.0, vec![track], Vec::new()).expect("valid pulse clip"),
        );
        Self {
            player: Player::new(clip, LoopMode::Loop),
        }
    }

    pub fn advance(&mut self, seconds: f32) {
        let _ = self.player.advance(seconds.max(0.0));
    }

    pub fn scale(&self) -> f32 {
        self.player.sample().f32("scale").unwrap_or(1.0)
    }
}

impl Default for LogoPulse {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logo_pulse_stays_in_range_and_loops() {
        let mut pulse = LogoPulse::new();
        let mut minimum = f32::MAX;
        let mut maximum = f32::MIN;
        for _ in 0..240 {
            pulse.advance(1.0 / 60.0);
            let scale = pulse.scale();
            assert!(scale.is_finite());
            minimum = minimum.min(scale);
            maximum = maximum.max(scale);
        }
        assert!((0.99..=1.01).contains(&minimum));
        assert!((1.02..1.04).contains(&maximum));
    }
}
