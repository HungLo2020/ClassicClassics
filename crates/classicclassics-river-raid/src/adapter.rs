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

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub engine: crate::game::Game,
    clock: VideoClock,
    pending: Input,
    pub paused: bool,
    #[serde(skip)]
    frame_cache: FrameCache,
    high_score: u64,
}
impl ArcadeGame for Game {
    type Record = Record;
    const COVER: Option<&'static [u8]> = Some(include_bytes!("../assets/cover.png"));
    fn paused(&self) -> bool {
        self.paused
    }
    const INFO: GameInfo = GameInfo {
        name: "River Raid",
        aliases: &["river-raid", "riverraid", "river"],
        tagline: "FLY LOW. WATCH YOUR FUEL.",
        accent: [102, 177, 255, 255],
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
        if !self.clock.advance(crate::game::TICK_RATE) {
            return;
        }
        let input = std::mem::take(&mut self.pending);
        self.frame_cache.invalidate();
        for event in self.engine.tick(crate::game::Input {
            left: input.move_x < 0,
            right: input.move_x > 0,
            up: input.move_y < 0,
            down: input.move_y > 0,
            fire: input.fire || input.fire_pressed,
            start: input.confirm,
            pause: false,
        }) {
            use crate::game::Event::*;
            sounds.push(match event {
                Shot => Sound::Laser,
                Explosion | Bridge => Sound::Explosion,
                ExtraLife => Sound::ExtraLife,
                GameOver => Sound::GameOver,
                LowFuel => Sound::Wall,
                Refuel | FullTank => Sound::Score,
            });
        }
    }
    fn autoplay(&self) -> Input {
        Input {
            confirm: true,
            fire: true,
            move_x: if self.engine.player_x < 72.0 {
                1
            } else if self.engine.player_x > 80.0 {
                -1
            } else {
                0
            },
            ..Default::default()
        }
    }
    fn scene(&self, hints: &ControlHints) -> Canvas {
        self.pixel_frame(hints).unwrap().reference_canvas()
    }
    fn pixel_frame(&self, hints: &ControlHints) -> Option<PixelFrame> {
        let help = format!(
            "{} STEER   {} SPEED   {} FIRE   {} SETTINGS{}",
            hints.move_x,
            hints.move_y,
            hints.fire,
            hints.settings,
            if self.paused { "   PAUSED" } else { "" }
        );
        let image = self.frame_cache.image(|| {
            let mut renderer = crate::render::Renderer::new();
            (
                renderer.render(&self.engine, self.high_score).to_vec(),
                160,
                210,
            )
        });
        Some(PixelFrame {
            image,
            aspect: 4.0 / 3.0,
            help,
        })
    }
    fn record(&mut self, record: &mut Record) {
        record.best_score = record.best_score.max(u64::from(self.engine.score));
        record.best_level = record.best_level.max(u32::from(self.engine.level));
        if self.high_score != record.best_score {
            self.frame_cache.invalidate();
        }
        self.high_score = record.best_score;
    }
    fn summary(&self) -> String {
        format!(
            "mode={:?} level={} score={} fuel={} frame={}",
            self.engine.mode,
            self.engine.level,
            self.engine.score,
            self.engine.fuel,
            self.engine.frame
        )
    }
    fn catalog_record(record: &Record) -> (&'static str, String) {
        ("BEST SCORE", record.best_score.to_string())
    }
    fn motif(paint: &mut Paint, cx: f32, cy: f32, accent: Color) {
        // River banks and banking jet.
        for side in [-1.0, 1.0] {
            paint.polyline(
                &[
                    [cx + side * 110.0, cy - 60.0],
                    [cx + side * 75.0, cy - 15.0],
                    [cx + side * 115.0, cy + 20.0],
                    [cx + side * 85.0, cy + 60.0],
                ],
                3.0,
                accent,
            );
        }
        paint.polyline(
            &[
                [cx, cy - 35.0],
                [cx - 7.0, cy],
                [cx - 33.0, cy + 16.0],
                [cx - 33.0, cy + 25.0],
                [cx - 5.0, cy + 14.0],
                [cx, cy + 35.0],
                [cx + 5.0, cy + 14.0],
                [cx + 33.0, cy + 25.0],
                [cx + 33.0, cy + 16.0],
                [cx + 7.0, cy],
            ],
            2.5,
            accent,
        );
    }
}
