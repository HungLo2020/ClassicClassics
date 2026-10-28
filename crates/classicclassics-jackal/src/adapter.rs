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

fn assets() -> &'static crate::assets::Assets {
    static ASSETS: std::sync::OnceLock<crate::assets::Assets> = std::sync::OnceLock::new();
    ASSETS.get_or_init(|| {
        crate::assets::Assets::load(&crate::assets::Assets::default_directory())
            .expect("packaged Jackal assets")
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub engine: crate::game::Game,
    clock: VideoClock,
    pending: Input,
    pub paused: bool,
    #[serde(skip)]
    frame_cache: FrameCache,
}
impl Default for Game {
    fn default() -> Self {
        // Prepare static assets during launch, never on the first gameplay draw.
        assets();
        Self {
            engine: Default::default(),
            clock: Default::default(),
            pending: Default::default(),
            paused: false,
            frame_cache: Default::default(),
        }
    }
}
impl ArcadeGame for Game {
    type Record = Record;
    const COVER: Option<&'static [u8]> = Some(include_bytes!("../assets/basalt/cover.png"));
    fn paused(&self) -> bool {
        self.paused
    }
    const INFO: GameInfo = GameInfo {
        name: "Jackal",
        aliases: &["jackal"],
        tagline: "RESCUE. REARM. RETURN.",
        accent: [137, 201, 106, 255],
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
        if !self.clock.advance(crate::TICKS_PER_SECOND) {
            return;
        }
        let input = std::mem::take(&mut self.pending);
        self.frame_cache.invalidate();
        let second = input.other_players[0];
        let player = |x: i8, y: i8, fire: bool, grenade: bool| crate::input::PlayerInput {
            left: x < 0,
            right: x > 0,
            up: y < 0,
            down: y > 0,
            gun: fire,
            grenade,
        };
        self.engine.update(
            crate::input::Input {
                players: [
                    player(
                        input.move_x,
                        input.move_y,
                        input.fire || input.fire_pressed,
                        input.secondary || input.secondary_pressed,
                    ),
                    player(second.x, second.y, second.fire, second.secondary),
                ],
                start: input.confirm,
                select: input.mode,
                pause: false,
            },
            assets(),
        );
        for event in self.engine.drain_events() {
            if let Some(sound) = crate::sound::cue(event) {
                sounds.push(sound);
            }
        }
    }
    fn autoplay(&self) -> Input {
        Input {
            confirm: self.engine.tick.is_multiple_of(120),
            fire: true,
            secondary: true,
            move_y: -1,
            move_x: if self.engine.tick % 240 < 120 { -1 } else { 1 },
            ..Default::default()
        }
    }
    fn scene(&self, hints: &ControlHints) -> Canvas {
        self.pixel_frame(hints).unwrap().reference_canvas()
    }
    fn pixel_frame(&self, hints: &ControlHints) -> Option<PixelFrame> {
        let help = format!(
            "{} {} MOVE   {} GUN   {} GRENADE   {} START{}",
            hints.move_x,
            hints.move_y,
            hints.fire,
            hints.hyperspace,
            hints.confirm,
            if self.paused { "   PAUSED" } else { "" }
        );
        let image = self.frame_cache.image(|| {
            let mut renderer = crate::renderer::Renderer::new();
            (renderer.render(&self.engine, assets()).to_vec(), 256, 240)
        });
        Some(PixelFrame {
            image,
            aspect: 4.0 / 3.0,
            help,
        })
    }
    fn record(&mut self, record: &mut Record) {
        for player in &self.engine.players {
            record.best_score = record.best_score.max(u64::from(player.score));
        }
        record.best_level = record.best_level.max(self.engine.stage as u32 + 1);
    }
    fn summary(&self) -> String {
        format!(
            "state={:?} stage={} players={} tick={} score={}",
            self.engine.state,
            self.engine.stage + 1,
            self.engine.selected_players,
            self.engine.tick,
            self.engine.players[0].score
        )
    }
    fn catalog_record(record: &Record) -> (&'static str, String) {
        ("BEST SCORE", record.best_score.to_string())
    }
    fn motif(paint: &mut Paint, cx: f32, cy: f32, accent: Color) {
        // Jeep, wheels, forward gun and rescue cross.
        paint.outline(cx - 55.0, cy - 30.0, 110.0, 70.0, accent);
        for x in [-67.0, 55.0] {
            for y in [-26.0, 24.0] {
                paint.rect(cx + x, cy + y, 12.0, 24.0, accent);
            }
        }
        paint.line([cx, cy - 12.0], [cx, cy - 64.0], 5.0, accent);
        paint.line([cx - 18.0, cy + 4.0], [cx + 18.0, cy + 4.0], 5.0, accent);
        paint.line([cx, cy - 14.0], [cx, cy + 22.0], 5.0, accent);
    }
}
