//! Replay-capable game session and the deterministic headless autoplayer.

use crate::game::{Difficulty, Game, Kind, Mode, OverReason, Phase, TickInput, Vec2, WIDTH};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replay {
    pub version: u32,
    pub mode: Mode,
    pub difficulty: Difficulty,
    pub seed: u64,
    pub inputs: Vec<TickInput>,
}

impl Replay {
    pub const VERSION: u32 = 1;
    pub fn new(game: &Game) -> Self {
        Self {
            version: Self::VERSION,
            mode: game.mode,
            difficulty: game.difficulty,
            seed: game.seed(),
            inputs: Vec::new(),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != Self::VERSION {
            return Err(format!(
                "unsupported replay version {} (expected {})",
                self.version,
                Self::VERSION
            ));
        }
        if self.inputs.is_empty() {
            return Err("replay contains no inputs".into());
        }
        Ok(())
    }
}

/// Small scripted player used by `--autoplay` and verification runs. It only
/// emits normal `TickInput` values, so autoplay never bypasses game rules.
#[derive(Debug, Clone, Default)]
pub struct Autoplay {
    anchor: Option<Vec2>,
    target: Option<(u64, Vec2)>,
    direction: Vec2,
    cooldown: f32,
}

impl Autoplay {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn input(&mut self, game: &Game) -> TickInput {
        if game.phase == Phase::GameOver {
            self.anchor = None;
            self.target = None;
            self.cooldown = 0.0;
            return TickInput {
                start: game.over_timer > 0.7,
                ..TickInput::default()
            };
        }
        self.cooldown = (self.cooldown - 1.0 / 120.0).max(0.0);
        if self.cooldown > 0.0 {
            return TickInput::default();
        }
        if let Some(anchor) = self.anchor {
            let target = self.target.map(|(_, p)| p).unwrap_or(anchor);
            self.anchor = None;
            self.target = None;
            self.cooldown = 0.05;
            return TickInput {
                pointer: Some(target + self.direction * 70.0),
                pointer_down: true,
                ..TickInput::default()
            };
        }
        let Some((id, position)) = self.choose_target(game) else {
            return TickInput::default();
        };
        let direction = self.choose_direction(game, position);
        self.target = Some((id, position));
        self.direction = direction;
        self.anchor = Some(position - direction * 70.0);
        TickInput {
            pointer: self.anchor,
            pointer_down: true,
            ..TickInput::default()
        }
    }

    fn choose_target(&self, game: &Game) -> Option<(u64, Vec2)> {
        game.fruits
            .iter()
            .filter(|fruit| !fruit.kind.is_bomb())
            .filter(|fruit| fruit.position.y > 100.0 && fruit.position.y < 640.0)
            .filter(|fruit| fruit.position.x > 80.0 && fruit.position.x < WIDTH - 80.0)
            .min_by(|a, b| {
                let da = (a.position.y - 380.0).abs() + (a.position.x - WIDTH * 0.5).abs() * 0.2;
                let db = (b.position.y - 380.0).abs() + (b.position.x - WIDTH * 0.5).abs() * 0.2;
                da.total_cmp(&db)
            })
            .map(|fruit| (fruit.id, fruit.position))
    }

    fn choose_direction(&self, game: &Game, target: Vec2) -> Vec2 {
        for offset in [0.0_f32, 0.55, -0.55, 1.1, -1.1, 1.75, -1.75, 2.6, -2.6] {
            let direction = Vec2::new(offset.cos(), offset.sin());
            let from = target - direction * 70.0;
            let to = target + direction * 70.0;
            let hits_bomb = game.fruits.iter().any(|fruit| {
                fruit.kind.is_bomb()
                    && crate::game::segment_hits_circle(
                        from,
                        to,
                        fruit.position,
                        fruit.radius + 34.0,
                    )
            });
            if !hits_bomb {
                return direction;
            }
        }
        Vec2::new(1.0, 0.0)
    }
}

/// Owns a game, its optional replay/recording, and the autoplayer.
pub struct Session {
    pub game: Game,
    pub autoplay: Autoplay,
    pub ticks: usize,
    pub replay: Option<Replay>,
    pub recording: Option<Replay>,
    pub best: i64,
    pub finished: bool,
}

impl Session {
    pub fn new(mode: Mode, difficulty: Difficulty, seed: u64) -> Self {
        Self {
            game: Game::new(mode, difficulty, seed),
            autoplay: Autoplay::new(),
            ticks: 0,
            replay: None,
            recording: None,
            best: 0,
            finished: false,
        }
    }

    pub fn with_replay(replay: Replay) -> Result<Self, String> {
        replay.validate()?;
        let mut session = Self::new(replay.mode, replay.difficulty, replay.seed);
        session.replay = Some(replay);
        Ok(session)
    }

    pub fn start_recording(&mut self) {
        self.recording = Some(Replay::new(&self.game));
    }

    pub fn restart(&mut self) {
        let seed = self.game.seed().wrapping_add(0x9e37_79b9_7f4a_7c15);
        self.game = Game::new(self.game.mode, self.game.difficulty, seed);
        self.ticks = 0;
        self.finished = false;
        self.autoplay = Autoplay::new();
        if self.recording.is_some() {
            self.recording = Some(Replay::new(&self.game));
        }
    }

    /// Advances one tick. Returns false once a replay or tick limit is exhausted.
    pub fn step(&mut self, live: TickInput, autoplay: bool) -> bool {
        if self.finished {
            return false;
        }
        let input = if let Some(replay) = &self.replay {
            let Some(input) = replay.inputs.get(self.ticks).cloned() else {
                self.finished = true;
                return false;
            };
            input
        } else if autoplay {
            self.autoplay.input(&self.game)
        } else {
            live
        };
        if let Some(recording) = &mut self.recording {
            recording.inputs.push(input);
        }
        self.game.tick(input);
        self.ticks += 1;
        self.best = self.best.max(self.game.score);
        true
    }

    pub fn step_count(&self) -> usize {
        self.ticks
    }

    pub fn best_score(&self) -> i64 {
        self.best.max(self.game.score)
    }

    pub fn over_reason(&self) -> Option<OverReason> {
        self.game.reason
    }

    pub fn mode(&self) -> Mode {
        self.game.mode
    }

    pub fn fruit_count(&self) -> usize {
        self.game.fruits.len()
    }

    pub fn kinds(&self) -> Vec<Kind> {
        self.game.fruits.iter().map(|fruit| fruit.kind).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autoplay_slices_fruit_and_avoids_restart_loops() {
        let mut session = Session::new(Mode::Zen, Difficulty::Normal, 11);
        let mut sliced = 0;
        for _ in 0..2400 {
            session.step(TickInput::default(), true);
            sliced = session.game.stats.sliced;
            if sliced >= 6 {
                break;
            }
        }
        assert!(sliced >= 6, "autoplay only sliced {sliced} fruit");
        assert!(session.game.score > 0);
    }

    #[test]
    fn replay_reproduces_the_same_final_state() {
        let mut live = Session::new(Mode::Arcade, Difficulty::Hard, 99);
        live.start_recording();
        for _ in 0..1200 {
            live.step(TickInput::default(), true);
        }
        let replay = live.recording.clone().unwrap();
        let expected = serde_json::to_vec(&live.game).unwrap();
        let mut played = Session::with_replay(replay).unwrap();
        let mut steps = 0;
        while played.step(TickInput::default(), false) {
            steps += 1;
        }
        assert_eq!(steps, 1200);
        assert_eq!(serde_json::to_vec(&played.game).unwrap(), expected);
    }

    #[test]
    fn replay_validation_rejects_bad_input() {
        let mut replay = Replay::new(&Game::new(Mode::Classic, Difficulty::Normal, 1));
        assert!(replay.validate().is_err());
        replay.inputs.push(TickInput::default());
        replay.version = 99;
        assert!(replay.validate().is_err());
    }
}
