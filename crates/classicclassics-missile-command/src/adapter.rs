use classicclassics_core::{
    ArcadeGame, ControlHints, GameInfo, Input, Sound,
    imported::{FrameCache, PixelFrame, VideoClock},
    paint::{Color, Paint},
};
use quasar_render_2d::Canvas;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Record {
    pub best_score: u64,
    pub best_level: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub engine: crate::game::Game,
    clock: VideoClock,
    pending: Input,
    pub paused: bool,
    #[serde(skip)]
    frame_cache: FrameCache,
    high_score: u64,
}
impl Default for Game {
    fn default() -> Self {
        Self {
            engine: crate::game::Game::new(0x4d495353494c45),
            clock: VideoClock::default(),
            pending: Input::default(),
            paused: false,
            frame_cache: FrameCache::default(),
            high_score: 0,
        }
    }
}
impl ArcadeGame for Game {
    type Record = Record;
    const COVER: Option<&'static [u8]> = Some(include_bytes!("../assets/cover.png"));
    fn paused(&self) -> bool {
        self.paused
    }
    const INFO: GameInfo = GameInfo {
        name: "Missile Command",
        aliases: &["missile-command", "missilecommand", "missile"],
        tagline: "DEFEND THE LAST SIX CITIES.",
        accent: [255, 199, 98, 255],
    };
    fn tick(&mut self, input: Input, sounds: &mut Vec<Sound>) {
        if input.restart {
            *self = Self::default();
            sounds.push(Sound::Asset(
                classicclassics_core::audio::AssetSound::StopMusic,
            ));
        }
        if input.pause {
            self.paused = !self.paused;
        }
        if self.paused {
            return;
        }
        let mut input = input;
        input.pause = false;
        input.restart = false;
        self.pending.latch(input);
        if !self.clock.advance(crate::game::TICK_HZ) {
            return;
        }
        let input = std::mem::take(&mut self.pending);
        self.frame_cache.invalidate();
        let mut aim = self.engine.aim;
        if let Some([x, y]) = input.pointer {
            // The original 256x231 view fills 560 logical pixels vertically.
            let width = 560.0 * 4.0 / 3.0;
            aim.x = ((x - (960.0 - width) * 0.5) / width * 256.0).clamp(0.0, 255.0);
            aim.y = (y / 560.0 * 231.0).clamp(0.0, 230.0);
        } else {
            aim.x = (aim.x + f32::from(input.move_x) * 2.5).clamp(0.0, 255.0);
            aim.y = (aim.y + f32::from(input.move_y) * 2.5).clamp(0.0, 230.0);
        }
        self.engine.update(crate::game::Input {
            aim,
            fire: [
                input.secondary_pressed,
                input.fire_pressed,
                input.jump_pressed,
            ],
            start: input.confirm,
        });
        for event in &self.engine.events {
            use crate::game::GameEvent::*;
            sounds.push(match event {
                Launch { .. } => Sound::Laser,
                Explosion | BaseDestroyed { .. } | CityDestroyed { .. } => Sound::Explosion,
                BonusCity => Sound::ExtraLife,
                GameOver => Sound::GameOver,
                _ => Sound::Score,
            });
        }
    }
    fn autoplay(&self) -> Input {
        Input {
            confirm: true,
            fire_pressed: self.engine.tick.is_multiple_of(12),
            secondary_pressed: self.engine.tick.is_multiple_of(17),
            jump_pressed: self.engine.tick.is_multiple_of(23),
            move_x: if self.engine.tick % 120 < 60 { 1 } else { -1 },
            ..Default::default()
        }
    }
    fn scene(&self, hints: &ControlHints) -> Canvas {
        self.pixel_frame(hints).unwrap().reference_canvas()
    }
    fn pixel_frame(&self, hints: &ControlHints) -> Option<PixelFrame> {
        let help = format!(
            "AIM MOUSE / {} {}   BATTERIES {}  {}  {}   {} START{}",
            hints.move_x,
            hints.move_y,
            hints.hyperspace,
            hints.fire,
            hints.jump,
            hints.confirm,
            if self.paused { "   PAUSED" } else { "" }
        );
        let image = self.frame_cache.image(|| {
            let mut renderer = crate::render::Renderer::new();
            (
                renderer
                    .render(&self.engine, self.engine.aim, self.high_score)
                    .to_vec(),
                256,
                231,
            )
        });
        Some(PixelFrame {
            image,
            aspect: 4.0 / 3.0,
            help,
        })
    }
    fn record(&mut self, record: &mut Record) {
        record.best_score = record.best_score.max(self.engine.score);
        record.best_level = record.best_level.max(self.engine.wave);
        if self.high_score != record.best_score {
            self.frame_cache.invalidate();
        }
        self.high_score = record.best_score;
    }
    fn summary(&self) -> String {
        format!(
            "phase={:?} wave={} score={} cities={} tick={}",
            self.engine.phase,
            self.engine.wave,
            self.engine.score,
            self.engine.alive_cities(),
            self.engine.tick
        )
    }
    fn catalog_record(record: &Record) -> (&'static str, String) {
        ("BEST SCORE", record.best_score.to_string())
    }
    fn motif(paint: &mut Paint, cx: f32, cy: f32, accent: Color) {
        // Three defensive batteries firing toward a chain explosion.
        for x in [-130.0, 0.0, 130.0] {
            paint.polyline(
                &[
                    [cx + x - 20.0, cy + 50.0],
                    [cx + x, cy + 30.0],
                    [cx + x + 20.0, cy + 50.0],
                ],
                3.0,
                accent,
            );
            paint.line([cx + x, cy + 30.0], [cx + 30.0, cy - 27.0], 2.0, accent);
        }
        paint.circle([cx + 30.0, cy - 27.0], 26.0, 3.0, accent);
        paint.circle([cx + 30.0, cy - 27.0], 14.0, 2.0, accent);
    }
}
