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

pub(crate) fn assets() -> std::sync::Arc<crate::assets::Assets> {
    std::sync::Arc::new(
        crate::assets::Assets::load(&crate::assets::default_root())
            .expect("Micro Mages fixture assets"),
    )
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub engine: crate::game::Game,
    clock: VideoClock,
    pending: Input,
    pub paused: bool,
    #[serde(skip)]
    frame_cache: FrameCache,
    #[serde(skip)]
    draw_cache: classicclassics_core::tiled::DrawCache<crate::video::Video>,
}
impl Default for Game {
    fn default() -> Self {
        Self {
            engine: crate::game::Game::new(assets()),
            clock: VideoClock::default(),
            pending: Input::default(),
            paused: false,
            frame_cache: FrameCache::default(),
            draw_cache: Default::default(),
        }
    }
}
impl Game {
    fn with_assets(assets: std::sync::Arc<crate::assets::Assets>) -> Self {
        Self {
            engine: crate::game::Game::new(assets),
            clock: Default::default(),
            pending: Default::default(),
            paused: false,
            frame_cache: Default::default(),
            draw_cache: Default::default(),
        }
    }
}
impl ArcadeGame for Game {
    type Record = Record;
    const ASSET_MANIFEST_DIR: Option<&'static str> = Some(env!("CARGO_MANIFEST_DIR"));
    fn load(root: Option<&classicclassics_assets::Directory>) -> Result<Self, String> {
        let root = root.ok_or("Micro Mages asset mount missing")?;
        Ok(Self::with_assets(std::sync::Arc::new(
            crate::assets::Assets::load(root)?,
        )))
    }
    const COVER: Option<&'static [u8]> = Some(include_bytes!("../assets/basalt/cover.png"));
    fn paused(&self) -> bool {
        self.paused
    }
    const INFO: GameInfo = GameInfo {
        name: "Micro Mages",
        aliases: &["micromages", "micro-mages"],
        tagline: "CLIMB THE TOWERS. TOGETHER.",
        accent: [181, 138, 255, 255],
    };
    fn tick(&mut self, input: Input, sounds: &mut Vec<Sound>) {
        if input.restart {
            *self = Self::with_assets(self.engine.assets().clone());
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
        if !self.clock.advance(60.0) {
            return;
        }
        let input = std::mem::take(&mut self.pending);
        self.frame_cache.invalidate();
        self.draw_cache.invalidate();
        let in_game = matches!(
            self.engine.screen,
            crate::game::Screen::Playing | crate::game::Screen::StageClear
        );
        let primary = classicclassics_core::input::PlayerControls {
            x: input.move_x,
            y: input.move_y,
            fire: input.fire || input.fire_pressed,
            jump: input.jump || input.jump_pressed || (!in_game && input.confirm),
            secondary: input.secondary || input.secondary_pressed,
        };
        let players = [
            primary,
            input.other_players[0],
            input.other_players[1],
            input.other_players[2],
        ];
        let buttons = players.map(|p| {
            u8::from(p.jump || (!in_game && p.fire))
                | (u8::from(if in_game { p.fire } else { p.secondary }) << 1)
                | (u8::from(p.y < 0) << 4)
                | (u8::from(p.y > 0) << 5)
                | (u8::from(p.x < 0) << 6)
                | (u8::from(p.x > 0) << 7)
        });
        for event in self.engine.update(buttons) {
            if let Some(sound) = crate::audio_cues::cue(event) {
                sounds.push(sound);
            }
        }
    }
    fn autoplay(&self) -> Input {
        Input {
            confirm: self.engine.frame.is_multiple_of(2),
            jump: self.engine.frame % 60 < 30,
            jump_pressed: self.engine.frame.is_multiple_of(60),
            fire: matches!(
                self.engine.screen,
                crate::game::Screen::Playing | crate::game::Screen::StageClear
            ),
            move_x: if self.engine.frame % 240 < 120 { 1 } else { -1 },
            ..Default::default()
        }
    }
    fn draw_frame(&self, hints: &ControlHints) -> Option<classicclassics_core::tiled::DrawFrame> {
        if !matches!(
            self.engine.screen,
            crate::game::Screen::Playing | crate::game::Screen::StageClear
        ) || self.engine.world.is_none()
        {
            return None;
        }
        let mut frame = self
            .draw_cache
            .render(|renderer| renderer.render_draw(&self.engine, self.engine.assets()));
        frame.help = format!(
            "{} {} MOVE   {} JUMP   {} FIRE   {} SETTINGS{}",
            hints.move_x,
            hints.move_y,
            hints.jump,
            hints.fire,
            hints.settings,
            if self.paused { "   PAUSED" } else { "" }
        );
        Some(frame)
    }
    fn scene(&self, hints: &ControlHints) -> Canvas {
        self.pixel_frame(hints).unwrap().reference_canvas()
    }
    fn pixel_frame(&self, hints: &ControlHints) -> Option<PixelFrame> {
        let mut help = if matches!(
            self.engine.screen,
            crate::game::Screen::Playing | crate::game::Screen::StageClear
        ) {
            format!(
                "{} {} MOVE   {} JUMP   {} FIRE   {} SETTINGS",
                hints.move_x, hints.move_y, hints.jump, hints.fire, hints.settings
            )
        } else {
            format!(
                "{} {} SELECT   {} NEXT   {} HARD MODE   {} SETTINGS",
                hints.move_x, hints.move_y, hints.confirm, hints.hyperspace, hints.settings
            )
        };
        if self.paused {
            help.push_str("   PAUSED");
        }
        let image = self.frame_cache.image(|| {
            let mut renderer = crate::video::Video::new();
            (
                renderer.render(&self.engine, self.engine.assets()).to_vec(),
                256,
                240,
            )
        });
        Some(PixelFrame {
            image,
            aspect: 4.0 / 3.0,
            help,
        })
    }
    fn record(&mut self, record: &mut Record) {
        if let Some(world) = &self.engine.world {
            record.best_score = record.best_score.max(u64::from(world.score) * 100);
            record.best_level = record.best_level.max(world.level.id as u32 + 1);
        }
    }
    fn summary(&self) -> String {
        format!(
            "screen={:?} players={} frame={} score={}",
            self.engine.screen,
            self.engine.player_count,
            self.engine.frame,
            self.engine.world.as_ref().map_or(0, |w| w.score * 100)
        )
    }
    fn catalog_record(record: &Record) -> (&'static str, String) {
        ("BEST SCORE", record.best_score.to_string())
    }
    fn motif(paint: &mut Paint, cx: f32, cy: f32, accent: Color) {
        // Tower, battlements and two cooperative mages.
        paint.rect(cx - 72.0, cy - 55.0, 144.0, 110.0, [31, 27, 48, 255]);
        for x in [-72.0, -24.0, 24.0] {
            paint.rect(cx + x, cy - 65.0, 24.0, 18.0, accent);
        }
        for x in [-34.0, 34.0] {
            paint.polyline(
                &[
                    [cx + x - 15.0, cy + 16.0],
                    [cx + x, cy - 20.0],
                    [cx + x + 15.0, cy + 16.0],
                ],
                3.0,
                accent,
            );
            paint.circle([cx + x, cy + 25.0], 11.0, 3.0, accent);
        }
    }
}
