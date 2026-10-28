//! Pong: a deterministic table-tennis simulation with a solo computer opponent
//! and a local two-player mode.

use classicclassics_core::ControlHints;
use classicclassics_core::paint::{CORAL, CYAN, CYAN_DIM, DIM, GRID, MUTED, Paint, RED, WHITE};
use classicclassics_core::{ArcadeGame, GameInfo, Input, Sound};
use classicclassics_core::{TICK_HZ, WIDTH};
use quasar_render_2d::Canvas;
use serde::{Deserialize, Serialize};

pub const WIN_SCORE: u32 = 7;
const TOP: f32 = 104.0;
const BOTTOM: f32 = 500.0;
const LEFT_FACE: f32 = 78.0;
const RIGHT_FACE: f32 = 882.0;
const CENTER_X: f32 = 480.0;
const PADDLE_HEIGHT: f32 = 78.0;
const PADDLE_HALF: f32 = PADDLE_HEIGHT * 0.5;
const PADDLE_SPEED: f32 = 390.0;
const BALL_RADIUS: f32 = 7.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Solo,
    Local,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Ready,
    Playing,
    Paused,
    Finished,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub mode: Mode,
    pub phase: Phase,
    pub paddles: [f32; 2],
    pub ball: [f32; 2],
    pub velocity: [f32; 2],
    pub score: [u32; 2],
    pub rally: u32,
    pub longest_rally: u32,
    pub ticks: u64,
    serve_direction: f32,
    #[serde(default)]
    finish_recorded: bool,
}

impl Game {
    pub fn new() -> Self {
        Self {
            mode: Mode::Solo,
            phase: Phase::Ready,
            paddles: [(TOP + BOTTOM) * 0.5; 2],
            ball: [CENTER_X, (TOP + BOTTOM) * 0.5],
            velocity: [0.0; 2],
            score: [0; 2],
            rally: 0,
            longest_rally: 0,
            ticks: 0,
            serve_direction: 1.0,
            finish_recorded: false,
        }
    }

    fn reset_match(&mut self) {
        let mode = self.mode;
        let ticks = self.ticks;
        *self = Self::new();
        self.mode = mode;
        self.ticks = ticks;
    }

    fn point(&mut self, player: usize, sounds: &mut Vec<Sound>) {
        self.score[player] += 1;
        self.rally = 0;
        self.ball = [CENTER_X, (TOP + BOTTOM) * 0.5];
        self.velocity = [0.0; 2];
        self.serve_direction = if player == 0 { -1.0 } else { 1.0 };
        if self.score[player] >= WIN_SCORE {
            self.phase = Phase::Finished;
            self.finish_recorded = true;
            sounds.push(Sound::Win);
        } else {
            self.phase = Phase::Ready;
            sounds.push(Sound::Score);
        }
    }
}

fn direction(delta: f32, deadzone: f32) -> i8 {
    if delta > deadzone {
        1
    } else if delta < -deadzone {
        -1
    } else {
        0
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Record {
    pub best_rally: u32,
    pub wins: u32,
    pub matches: u32,
}

impl ArcadeGame for Game {
    type Record = Record;
    const INFO: GameInfo = GameInfo {
        name: "PONG",
        aliases: &["pong"],
        tagline: "TWO PADDLES. ONE BALL.",
        accent: classicclassics_core::paint::CYAN,
    };

    fn tick(&mut self, input: Input, sounds: &mut Vec<Sound>) {
        self.ticks += 1;
        if input.restart {
            self.reset_match();
            sounds.push(Sound::MenuSelect);
            return;
        }
        if input.mode {
            self.mode = match self.mode {
                Mode::Solo => Mode::Local,
                Mode::Local => Mode::Solo,
            };
            self.reset_match();
            sounds.push(Sound::MenuSelect);
            return;
        }
        if input.pause {
            self.phase = match self.phase {
                Phase::Playing => Phase::Paused,
                Phase::Paused => Phase::Playing,
                other => other,
            };
            return;
        }
        if self.phase == Phase::Finished {
            if input.fire_pressed || input.confirm {
                self.reset_match();
                sounds.push(Sound::MenuSelect);
            }
            return;
        }
        if self.phase == Phase::Paused {
            return;
        }

        let dt = 1.0 / TICK_HZ as f32;
        let right = if self.mode == Mode::Solo {
            let target = if self.velocity[0] > 0.0 {
                self.ball[1]
            } else {
                (TOP + BOTTOM) * 0.5
            };
            direction(target - self.paddles[1], 8.0)
        } else {
            input.p2.clamp(-1, 1)
        };
        for (index, axis) in [input.p1.clamp(-1, 1), right].into_iter().enumerate() {
            let speed = if index == 1 && self.mode == Mode::Solo {
                300.0
            } else {
                PADDLE_SPEED
            };
            self.paddles[index] = (self.paddles[index] + axis as f32 * speed * dt)
                .clamp(TOP + PADDLE_HALF, BOTTOM - PADDLE_HALF);
        }

        if self.phase == Phase::Ready {
            if input.fire_pressed || input.confirm {
                self.velocity = [
                    self.serve_direction * 330.0,
                    if self.score.iter().sum::<u32>().is_multiple_of(2) {
                        120.0
                    } else {
                        -120.0
                    },
                ];
                self.phase = Phase::Playing;
                sounds.push(Sound::Paddle);
            }
            return;
        }

        let previous = self.ball;
        self.ball[0] += self.velocity[0] * dt;
        self.ball[1] += self.velocity[1] * dt;
        if self.ball[1] < TOP + BALL_RADIUS {
            self.ball[1] = 2.0 * (TOP + BALL_RADIUS) - self.ball[1];
            self.velocity[1] = self.velocity[1].abs();
            sounds.push(Sound::Wall);
        } else if self.ball[1] > BOTTOM - BALL_RADIUS {
            self.ball[1] = 2.0 * (BOTTOM - BALL_RADIUS) - self.ball[1];
            self.velocity[1] = -self.velocity[1].abs();
            sounds.push(Sound::Wall);
        }

        for (index, face, sign) in [(0, LEFT_FACE, 1.0), (1, RIGHT_FACE, -1.0)] {
            let crossing = if index == 0 {
                self.velocity[0] < 0.0 && previous[0] >= face && self.ball[0] <= face
            } else {
                self.velocity[0] > 0.0 && previous[0] <= face && self.ball[0] >= face
            };
            if !crossing {
                continue;
            }
            let fraction = (face - previous[0]) / (self.ball[0] - previous[0]);
            let y = previous[1] + fraction * (self.ball[1] - previous[1]);
            let offset = y - self.paddles[index];
            if offset.abs() <= PADDLE_HALF + BALL_RADIUS {
                let angle = (offset / PADDLE_HALF).clamp(-1.0, 1.0) * 0.92;
                let speed = (self.velocity[0].hypot(self.velocity[1]) + 24.0).min(620.0);
                self.velocity = [sign * speed * angle.cos(), speed * angle.sin()];
                self.ball[0] = face + sign * (self.ball[0] - face).abs();
                self.rally += 1;
                self.longest_rally = self.longest_rally.max(self.rally);
                sounds.push(Sound::Paddle);
            }
        }

        if self.ball[0] < -BALL_RADIUS {
            self.point(1, sounds);
        } else if self.ball[0] > WIDTH as f32 + BALL_RADIUS {
            self.point(0, sounds);
        }
    }

    fn record(&mut self, records: &mut Record) {
        records.best_rally = records.best_rally.max(self.longest_rally);
        if self.finish_recorded {
            records.matches = records.matches.saturating_add(1);
            if self.score[0] >= WIN_SCORE {
                records.wins = records.wins.saturating_add(1);
            }
            self.finish_recorded = false;
        }
    }

    fn autoplay(&self) -> Input {
        Input {
            p1: direction(self.ball[1] - self.paddles[0], 4.0),
            p2: direction(self.ball[1] - self.paddles[1], 4.0),
            fire_pressed: self.phase == Phase::Ready,
            restart: self.phase == Phase::Finished,
            confirm: false,
            ..Default::default()
        }
    }

    fn summary(&self) -> String {
        format!(
            "mode={:?} phase={:?} score={}:{} rally={} best_rally={}",
            self.mode, self.phase, self.score[0], self.score[1], self.rally, self.longest_rally
        )
    }

    fn scene(&self, hints: &ControlHints) -> Canvas {
        let mut paint = Paint::new(WIDTH, classicclassics_core::HEIGHT);
        paint.background();
        paint.starfield(0x50A6, 80, [22, 33, 45, 255]);
        paint.text(64.0, 28.0, "PONG", 3.0, WHITE);
        paint.text(65.0, 58.0, "QUASAR ARCADE / TABLE 01", 1.0, MUTED);
        paint.text(430.0, 24.0, &self.score[0].to_string(), 4.0, CYAN);
        paint.text(470.0, 32.0, ":", 3.0, DIM);
        paint.text(494.0, 24.0, &self.score[1].to_string(), 4.0, CORAL);
        paint.right(
            896.0,
            30.0,
            match self.mode {
                Mode::Solo => "SINGLE PLAYER / VS CPU",
                Mode::Local => "LOCAL / TWO PLAYER",
            },
            1.0,
            WHITE,
        );
        paint.right(896.0, 48.0, "FIRST TO SEVEN", 1.0, MUTED);

        paint.rect(64.0, TOP - 1.0, 832.0, BOTTOM - TOP + 2.0, GRID);
        paint.rect(65.0, TOP, 830.0, BOTTOM - TOP, [9, 14, 22, 255]);
        paint.dashed_line(
            [CENTER_X, TOP + 8.0],
            [CENTER_X, BOTTOM - 8.0],
            10.0,
            11.0,
            2.0,
            [34, 55, 68, 255],
        );
        paint.glow_circle(
            [CENTER_X, (TOP + BOTTOM) * 0.5],
            54.0,
            1.0,
            [24, 42, 53, 255],
        );

        for (index, x) in [LEFT_FACE, RIGHT_FACE].into_iter().enumerate() {
            let y = self.paddles[index] - PADDLE_HALF;
            let color = if index == 0 { CYAN } else { CORAL };
            let halo = if index == 0 {
                CYAN_DIM
            } else {
                [85, 43, 36, 255]
            };
            paint.rect(
                x - 10.0,
                y - 4.0,
                20.0,
                PADDLE_HEIGHT + 8.0,
                [10, 18, 27, 255],
            );
            paint.rect(x - 5.0, y - 4.0, 10.0, PADDLE_HEIGHT + 8.0, halo);
            paint.rect(x - 3.0, y, 6.0, PADDLE_HEIGHT, color);
            paint.rect(x - 1.0, y + 6.0, 1.0, PADDLE_HEIGHT - 12.0, WHITE);
        }

        let ball = self.ball;
        paint.glow_circle(ball, BALL_RADIUS + 4.0, 8.0, [30, 74, 68, 255]);
        paint.circle(ball, BALL_RADIUS, 10.0, [95, 235, 214, 255]);
        paint.circle(ball, BALL_RADIUS - 1.0, 7.0, WHITE);

        let overlay = match self.phase {
            Phase::Ready => Some(("READY", format!("{} TO SERVE", hints.fire))),
            Phase::Paused => Some(("PAUSED", format!("{} TO RESUME", hints.pause))),
            Phase::Finished => Some((
                if self.score[0] >= WIN_SCORE {
                    "PLAYER ONE WINS"
                } else {
                    "PLAYER TWO WINS"
                },
                format!("{} TO PLAY AGAIN", hints.fire),
            )),
            Phase::Playing => None,
        };
        if let Some((title, subtitle)) = overlay {
            paint.dim_rect(294.0, 236.0, 372.0, 122.0);
            paint.outline(294.0, 236.0, 372.0, 122.0, GRID);
            paint.rect(318.0, 254.0, 28.0, 2.0, CYAN);
            paint.centered(266.0, title, 3.0, WHITE);
            paint.centered(314.0, &subtitle, 1.0, MUTED);
        }

        paint.text(64.0, 526.0, &format!("{}  MOVE", hints.p1), 1.0, CYAN);
        paint.text(
            246.0,
            526.0,
            &format!(
                "{}  {}",
                hints.p2,
                if self.mode == Mode::Solo {
                    "CPU"
                } else {
                    "MOVE"
                }
            ),
            1.0,
            CORAL,
        );
        paint.text(
            430.0,
            526.0,
            &format!(
                "{}  SERVE   {}  PAUSE   {}  MODE",
                hints.fire, hints.pause, hints.mode
            ),
            1.0,
            MUTED,
        );
        paint.text(
            64.0,
            550.0,
            &format!(
                "{}  RESET   RALLY {:02}   LONGEST {:02}",
                hints.restart, self.rally, self.longest_rally
            ),
            1.0,
            DIM,
        );
        paint.right(
            896.0,
            550.0,
            &format!(
                "{} FULLSCREEN   {} FILTER   {} SCALE",
                hints.fullscreen, hints.filter, hints.scale
            ),
            1.0,
            DIM,
        );
        paint.text(
            64.0,
            574.0,
            &format!("{}  SETTINGS    {}  CATALOG", hints.settings, hints.back),
            1.0,
            DIM,
        );
        if self.phase == Phase::Playing && self.ball[0] > CENTER_X {
            paint.text(820.0, 574.0, "IN PLAY", 1.0, RED);
        }
        paint.into_canvas()
    }

    fn catalog_record(record: &Record) -> (&'static str, String) {
        ("BEST RALLY", record.best_rally.to_string())
    }

    fn motif(paint: &mut Paint, cx: f32, cy: f32, accent: [u8; 4]) {
        use classicclassics_core::paint::{CORAL, CYAN, WHITE};
        let left = cx - 82.0;
        let top = cy - 58.0;
        paint.outline(left, top, 164.0, 116.0, [30, 51, 65, 255]);
        paint.dashed_line(
            [cx, top + 6.0],
            [cx, top + 110.0],
            6.0,
            7.0,
            1.0,
            [39, 66, 77, 255],
        );
        paint.rect(left + 12.0, cy - 21.0, 5.0, 42.0, CYAN);
        paint.rect(left + 147.0, cy - 14.0, 5.0, 36.0, CORAL);
        paint.rect(cx + 22.0, cy - 8.0, 7.0, 7.0, WHITE);
        paint.line([cx - 18.0, cy + 8.0], [cx + 12.0, cy - 13.0], 1.0, accent);
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serve_wall_paddle_and_score_progress() {
        let mut game = Game::new();
        let mut sounds = Vec::new();
        game.tick(
            Input {
                fire_pressed: true,
                ..Default::default()
            },
            &mut sounds,
        );
        assert_eq!(game.phase, Phase::Playing);
        assert!(game.velocity[0] > 0.0);

        game.ball = [100.0, TOP + BALL_RADIUS + 0.5];
        game.velocity = [-330.0, -120.0];
        game.tick(Input::default(), &mut sounds);
        assert!(game.velocity[1] > 0.0);

        game.ball = [LEFT_FACE + 0.2, game.paddles[0]];
        game.velocity = [-380.0, 0.0];
        game.tick(Input::default(), &mut sounds);
        assert!(game.velocity[0] > 0.0);
        assert_eq!(game.rally, 1);

        game.score[0] = WIN_SCORE - 1;
        game.ball = [WIDTH as f32 + BALL_RADIUS - 0.1, 200.0];
        game.velocity = [380.0, 0.0];
        game.tick(Input::default(), &mut sounds);
        assert_eq!(game.phase, Phase::Finished);
    }

    #[test]
    fn pause_and_mode_switch_are_stable() {
        let mut game = Game::new();
        let mut sounds = Vec::new();
        game.tick(
            Input {
                fire_pressed: true,
                ..Default::default()
            },
            &mut sounds,
        );
        game.tick(
            Input {
                pause: true,
                ..Default::default()
            },
            &mut sounds,
        );
        let ball = game.ball;
        game.tick(
            Input {
                p1: 1,
                ..Default::default()
            },
            &mut sounds,
        );
        assert_eq!(game.ball, ball);
        game.tick(
            Input {
                pause: true,
                ..Default::default()
            },
            &mut sounds,
        );
        game.tick(
            Input {
                mode: true,
                ..Default::default()
            },
            &mut sounds,
        );
        assert_eq!(game.mode, Mode::Local);
        assert_eq!(game.phase, Phase::Ready);
    }

    #[test]
    fn autoplay_rallies_and_records_are_bounded() {
        let mut game = Game::new();
        let mut records = Record::default();
        let mut sounds = Vec::new();
        for _ in 0..14_000 {
            let input = game.autoplay();
            game.tick(input, &mut sounds);
            game.record(&mut records);
        }
        assert!(game.longest_rally > 0);
        assert!(records.best_rally >= game.longest_rally);
    }
}
