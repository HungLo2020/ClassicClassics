//! Lunar Lander: a vector-drawn gravity descent with fuel, rotating thrust and
//! multiplier landing pads.

use crate::audio::Sound;
use crate::controls::ControlHints;
use crate::games::{Particle, Rng, update_particles};
use crate::paint::{AMBER, CORAL, CYAN, DIM, GREEN, GRID, MUTED, Paint, RED, WHITE, world};
use crate::session::{Input, Records};
use crate::{HEIGHT, TICK_HZ, WIDTH};
use quasar_render_2d::Canvas;
use serde::{Deserialize, Serialize};

const GRAVITY: f32 = 23.0;
const THRUST: f32 = 54.0;
const ROTATION_SPEED: f32 = 2.6;
const FUEL_BURN: f32 = 5.5;
const MAX_FUEL: f32 = 100.0;
const TERRAIN_POINTS: usize = 41;
const TERRAIN_STEP: f32 = WIDTH as f32 / (TERRAIN_POINTS - 1) as f32;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pad {
    pub x0: f32,
    pub x1: f32,
    pub y: f32,
    pub multiplier: u32,
}

impl Pad {
    fn center(&self) -> f32 {
        (self.x0 + self.x1) * 0.5
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Terrain {
    pub points: Vec<[f32; 2]>,
    pub pads: Vec<Pad>,
}

impl Terrain {
    fn generate(level: u32) -> Self {
        let mut rng = Rng::new(0x1A_4D_3E + level as u64 * 977);
        let base = 438.0 + (level % 3) as f32 * 14.0;
        let mut heights = Vec::with_capacity(TERRAIN_POINTS);
        for index in 0..TERRAIN_POINTS {
            let wave = (index as f32 * 0.48 + level as f32 * 0.7).sin() * 18.0;
            let noise = rng.range(-13.0, 13.0);
            heights.push((base + wave + noise).clamp(372.0, 526.0));
        }

        let pad_starts = [6usize, 17, 29];
        let multipliers = [2u32, 3, 5];
        let mut pads = Vec::with_capacity(3);
        for (slot, start) in pad_starts.into_iter().enumerate() {
            let start = start.min(TERRAIN_POINTS - 4);
            let y = rng.range(438.0, 512.0);
            for height in heights.iter_mut().skip(start).take(3) {
                *height = y;
            }
            pads.push(Pad {
                x0: start as f32 * TERRAIN_STEP,
                x1: (start + 2) as f32 * TERRAIN_STEP,
                y,
                multiplier: multipliers[slot],
            });
        }

        let points = heights
            .into_iter()
            .enumerate()
            .map(|(index, y)| [index as f32 * TERRAIN_STEP, y])
            .collect();
        Self { points, pads }
    }

    fn y_at(&self, x: f32) -> f32 {
        let x = x.clamp(0.0, WIDTH as f32);
        let index = ((x / TERRAIN_STEP).floor() as usize).min(TERRAIN_POINTS - 2);
        let a = self.points[index];
        let b = self.points[index + 1];
        let span = (b[0] - a[0]).max(0.001);
        let t = ((x - a[0]) / span).clamp(0.0, 1.0);
        a[1] + (b[1] - a[1]) * t
    }

    fn pad_at(&self, x: f32) -> Option<&Pad> {
        self.pads
            .iter()
            .find(|pad| x >= pad.x0 - 3.0 && x <= pad.x1 + 3.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Flying,
    Landed,
    Crashed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub angle: f32,
    pub fuel: f32,
    pub score: u32,
    pub level: u32,
    pub landings: u32,
    pub crashes: u32,
    pub phase: Phase,
    pub paused: bool,
    pub timer: f32,
    pub ticks: u64,
    pub terrain: Terrain,
    pub particles: Vec<Particle>,
    pub last_bonus: u32,
    #[serde(default)]
    thrust_sound_timer: f32,
}

impl Game {
    pub fn new() -> Self {
        let level = 1;
        let terrain = Terrain::generate(level);
        Self {
            x: 160.0,
            y: 78.0,
            vx: 28.0,
            vy: 0.0,
            angle: 0.0,
            fuel: MAX_FUEL,
            score: 0,
            level,
            landings: 0,
            crashes: 0,
            phase: Phase::Flying,
            paused: false,
            timer: 0.0,
            ticks: 0,
            terrain,
            particles: Vec::new(),
            last_bonus: 0,
            thrust_sound_timer: 0.0,
        }
    }

    pub fn tick(&mut self, input: Input, sounds: &mut Vec<Sound>) {
        self.ticks += 1;
        if input.restart {
            self.respawn(false);
            sounds.push(Sound::MenuSelect);
            return;
        }
        if input.pause {
            self.paused = !self.paused;
            return;
        }
        if self.paused {
            return;
        }
        let dt = 1.0 / TICK_HZ as f32;
        update_particles(&mut self.particles, dt);

        if self.phase != Phase::Flying {
            self.timer -= dt;
            if self.timer <= 0.0 {
                let next_level = self.phase == Phase::Landed;
                self.respawn(next_level);
            }
            return;
        }

        self.angle += input.rotate.clamp(-1, 1) as f32 * ROTATION_SPEED * dt;
        if self.angle > std::f32::consts::PI {
            self.angle -= std::f32::consts::TAU;
        } else if self.angle < -std::f32::consts::PI {
            self.angle += std::f32::consts::TAU;
        }

        let thrusting = input.thrust && self.fuel > 0.0;
        if thrusting {
            let direction = [self.angle.sin(), -self.angle.cos()];
            self.vx += direction[0] * THRUST * dt;
            self.vy += direction[1] * THRUST * dt;
            self.fuel = (self.fuel - FUEL_BURN * dt).max(0.0);
            self.thrust_sound_timer -= dt;
            if self.thrust_sound_timer <= 0.0 {
                sounds.push(Sound::Thrust);
                self.thrust_sound_timer = 0.075;
            }
        } else {
            self.thrust_sound_timer = 0.0;
        }
        self.vy += GRAVITY * dt;
        self.vx = self.vx.clamp(-180.0, 180.0);
        self.vy = self.vy.clamp(-180.0, 220.0);
        self.x += self.vx * dt;
        self.y += self.vy * dt;

        if self.x < -30.0 || self.x > WIDTH as f32 + 30.0 || self.y > HEIGHT as f32 + 50.0 {
            self.crash(sounds);
            return;
        }

        let left_foot = world([self.x, self.y], self.angle, [-13.0, 24.0]);
        let right_foot = world([self.x, self.y], self.angle, [13.0, 24.0]);
        let left_ground = self.terrain.y_at(left_foot[0]);
        let right_ground = self.terrain.y_at(right_foot[0]);
        if left_foot[1] >= left_ground || right_foot[1] >= right_ground {
            let center = (left_foot[0] + right_foot[0]) * 0.5;
            let pad = self.terrain.pad_at(center).cloned();
            let aligned = self.angle.abs() < 0.22;
            let gentle = self.vy.abs() < 35.0 && self.vx.abs() < 28.0;
            let feet_on_pad = pad.as_ref().is_some_and(|pad| {
                left_foot[0] >= pad.x0 - 4.0
                    && right_foot[0] <= pad.x1 + 4.0
                    && (left_foot[1] - pad.y).abs() < 10.0
                    && (right_foot[1] - pad.y).abs() < 10.0
            });
            if let Some(pad) = pad {
                if feet_on_pad && aligned && gentle {
                    self.land(pad, sounds);
                } else {
                    self.crash(sounds);
                }
            } else {
                self.crash(sounds);
            }
        }
    }

    fn land(&mut self, pad: Pad, sounds: &mut Vec<Sound>) {
        let bonus = pad.multiplier * (120 + self.fuel.round() as u32);
        self.last_bonus = bonus;
        self.score = self.score.saturating_add(bonus);
        self.landings = self.landings.saturating_add(1);
        self.phase = Phase::Landed;
        self.timer = 1.9;
        for index in 0..18 {
            let angle = index as f32 / 18.0 * std::f32::consts::TAU;
            let speed = 35.0 + (index % 5) as f32 * 12.0;
            self.particles.push(Particle::new(
                [self.x, self.y + 16.0],
                [angle.cos() * speed, angle.sin().abs() * speed * 0.4],
                0.8,
                2,
            ));
        }
        sounds.push(Sound::Landing);
    }

    fn crash(&mut self, sounds: &mut Vec<Sound>) {
        if self.phase == Phase::Crashed {
            return;
        }
        self.phase = Phase::Crashed;
        self.timer = 1.55;
        self.crashes = self.crashes.saturating_add(1);
        for index in 0..34 {
            let angle = index as f32 / 34.0 * std::f32::consts::TAU;
            let speed = 35.0 + (index % 7) as f32 * 16.0;
            self.particles.push(Particle::new(
                [self.x, self.y],
                [angle.cos() * speed, angle.sin() * speed - 12.0],
                1.1,
                if index % 3 == 0 { 0 } else { 1 },
            ));
        }
        sounds.push(Sound::Crash);
    }

    fn respawn(&mut self, next_level: bool) {
        if next_level {
            self.level = self.level.saturating_add(1);
        }
        self.terrain = Terrain::generate(self.level);
        self.x = 130.0 + (self.level as f32 * 73.0) % 680.0;
        self.y = 74.0;
        self.vx = 24.0 + (self.level % 4) as f32 * 3.0;
        self.vy = 0.0;
        self.angle = 0.0;
        self.fuel = MAX_FUEL;
        self.phase = Phase::Flying;
        self.paused = false;
        self.timer = 0.0;
        self.last_bonus = 0;
        self.particles.clear();
        self.thrust_sound_timer = 0.0;
    }

    pub fn record(&self, records: &mut Records) {
        records.lunar.best_score = records.lunar.best_score.max(self.score);
        records.lunar.landings = records.lunar.landings.max(self.landings);
        records.lunar.crashes = records.lunar.crashes.max(self.crashes);
    }

    pub fn autoplay(&self) -> Input {
        if self.phase != Phase::Flying {
            return Input::default();
        }
        let pad = self
            .terrain
            .pads
            .iter()
            .min_by(|a, b| {
                let da = (a.center() - self.x).abs() - a.multiplier as f32 * 8.0;
                let db = (b.center() - self.x).abs() - b.multiplier as f32 * 8.0;
                da.total_cmp(&db)
            })
            .expect("terrain always has pads");
        let desired = ((pad.center() - self.x) * 0.011).clamp(-0.58, 0.58);
        let rotate = if self.angle < desired - 0.035 {
            1
        } else if self.angle > desired + 0.035 {
            -1
        } else {
            0
        };
        let altitude = self.terrain.y_at(self.x) - self.y;
        let thrust = self.vy > 42.0
            || (altitude < 175.0 && self.vy > 14.0)
            || (altitude < 110.0 && self.vy > 4.0);
        Input {
            rotate,
            thrust,
            ..Default::default()
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "phase={:?} level={} score={} fuel={:.1} v=({:.1},{:.1}) landings={} crashes={}",
            self.phase,
            self.level,
            self.score,
            self.fuel,
            self.vx,
            self.vy,
            self.landings,
            self.crashes
        )
    }

    pub fn scene(&self, hints: &ControlHints) -> Canvas {
        let mut paint = Paint::new(WIDTH, HEIGHT);
        paint.background();
        paint.starfield(0x1A4D3E, 150, [42, 49, 60, 255]);
        paint.glow_circle([780.0, 104.0], 42.0, 1.0, [50, 57, 68, 255]);
        paint.glow_circle([790.0, 96.0], 9.0, 1.0, [37, 42, 52, 255]);

        paint.text(60.0, 26.0, "LUNAR LANDER", 3.0, WHITE);
        paint.text(61.0, 58.0, "QUASAR ARCADE / VECTOR FLIGHT 02", 1.0, MUTED);
        paint.text(60.0, 88.0, &format!("SCORE {:06}", self.score), 2.0, AMBER);
        paint.text(60.0, 118.0, &format!("LEVEL {:02}", self.level), 1.0, MUTED);

        let fuel_width = 238.0 * (self.fuel / MAX_FUEL).clamp(0.0, 1.0);
        paint.text(658.0, 30.0, "FUEL", 1.0, DIM);
        paint.right(
            896.0,
            30.0,
            &format!("{:03.0}%", self.fuel),
            1.0,
            if self.fuel < 25.0 { RED } else { AMBER },
        );
        paint.rect(658.0, 46.0, 238.0, 12.0, [15, 23, 34, 255]);
        paint.outline(658.0, 46.0, 238.0, 12.0, GRID);
        paint.rect(
            660.0,
            48.0,
            fuel_width,
            8.0,
            if self.fuel < 25.0 { CORAL } else { AMBER },
        );
        paint.right(
            896.0,
            76.0,
            &format!(
                "ALT {:05.0}   VX {:+04.0}   VY {:+04.0}",
                self.altitude(),
                self.vx,
                self.vy
            ),
            1.0,
            MUTED,
        );
        paint.right(
            896.0,
            96.0,
            &format!("ANGLE {:+05.1} DEG", self.angle.to_degrees()),
            1.0,
            MUTED,
        );

        let terrain_points: Vec<[f32; 2]> = self
            .terrain
            .points
            .iter()
            .map(|point| [point[0], point[1].min(HEIGHT as f32 - 24.0)])
            .collect();
        paint.glow_polyline(
            &terrain_points,
            1.5,
            [38, 34, 26, 255],
            [124, 104, 58, 255],
            false,
        );
        paint.rect(0.0, 560.0, WIDTH as f32, 40.0, [8, 12, 18, 255]);
        for point in &self.terrain.points {
            paint.rect(
                point[0],
                point[1],
                2.0,
                (HEIGHT as f32 - point[1]).max(0.0),
                [9, 13, 20, 255],
            );
        }

        for pad in &self.terrain.pads {
            paint.line([pad.x0, pad.y], [pad.x1, pad.y], 6.0, [74, 62, 35, 255]);
            paint.line([pad.x0 + 2.0, pad.y], [pad.x1 - 2.0, pad.y], 2.0, AMBER);
            for x in [pad.x0, pad.x1] {
                paint.line([x, pad.y - 8.0], [x, pad.y + 2.0], 2.0, AMBER);
            }
            paint.text(
                pad.center() - 6.0,
                pad.y - 20.0,
                &format!("x{}", pad.multiplier),
                1.0,
                AMBER,
            );
        }

        if self.phase == Phase::Flying || self.phase == Phase::Landed {
            self.draw_lander(&mut paint, hints);
        }
        for particle in &self.particles {
            let color = match particle.tone {
                0 => [255, 196, 92, 255],
                1 => [255, 112, 74, 255],
                _ => [118, 238, 190, 255],
            };
            let alpha = (particle.life / particle.max_life).clamp(0.0, 1.0);
            let length = 2.0 + alpha * 7.0;
            let direction = [particle.velocity[0], particle.velocity[1]];
            let magnitude = direction[0].hypot(direction[1]).max(0.01);
            paint.line(
                particle.position,
                [
                    particle.position[0] - direction[0] / magnitude * length,
                    particle.position[1] - direction[1] / magnitude * length,
                ],
                1.5,
                color,
            );
        }

        let overlay = match self.phase {
            Phase::Landed => Some((
                "TOUCHDOWN",
                format!("+{} BONUS   {} FOR NEXT LEVEL", self.last_bonus, "AUTO"),
            )),
            Phase::Crashed => Some(("HULL LOST", "RESPAWNING ON THIS TERRAIN".into())),
            Phase::Flying if self.paused => Some(("PAUSED", format!("{} TO RESUME", hints.pause))),
            _ => None,
        };
        if let Some((title, subtitle)) = overlay {
            paint.dim_rect(290.0, 230.0, 380.0, 120.0);
            paint.outline(290.0, 230.0, 380.0, 120.0, GRID);
            paint.rect(314.0, 248.0, 28.0, 2.0, AMBER);
            paint.centered(260.0, title, 3.0, WHITE);
            paint.centered(308.0, &subtitle, 1.0, MUTED);
        }

        paint.text(
            60.0,
            526.0,
            &format!("{}  ROTATE", hints.rotate),
            1.0,
            AMBER,
        );
        paint.text(
            282.0,
            526.0,
            &format!("{}  THRUST", hints.thrust),
            1.0,
            CYAN,
        );
        paint.text(
            440.0,
            526.0,
            &format!("{}  RESTART   {}  PAUSE", hints.restart, hints.pause),
            1.0,
            MUTED,
        );
        paint.text(
            60.0,
            550.0,
            &format!(
                "{}  SETTINGS   {}  CATALOG   LAND SOFT / UPRIGHT / ON A PAD",
                hints.settings, hints.back
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
            60.0,
            574.0,
            &format!(
                "LANDINGS {:02}   CRASHES {:02}",
                self.landings, self.crashes
            ),
            1.0,
            GREEN,
        );
        paint.into_canvas()
    }

    fn draw_lander(&self, paint: &mut Paint, hints: &ControlHints) {
        let position = [self.x, self.y];
        let body = [
            world(position, self.angle, [-10.0, -13.0]),
            world(position, self.angle, [10.0, -13.0]),
            world(position, self.angle, [15.0, 5.0]),
            world(position, self.angle, [8.0, 15.0]),
            world(position, self.angle, [-8.0, 15.0]),
            world(position, self.angle, [-15.0, 5.0]),
        ];
        paint.glow_polyline(&body, 1.8, [75, 58, 26, 255], AMBER, true);
        paint.line(
            world(position, self.angle, [-8.0, 15.0]),
            world(position, self.angle, [-15.0, 24.0]),
            1.5,
            AMBER,
        );
        paint.line(
            world(position, self.angle, [8.0, 15.0]),
            world(position, self.angle, [15.0, 24.0]),
            1.5,
            AMBER,
        );
        paint.line(
            world(position, self.angle, [-15.0, 24.0]),
            world(position, self.angle, [-9.0, 24.0]),
            2.0,
            AMBER,
        );
        paint.line(
            world(position, self.angle, [15.0, 24.0]),
            world(position, self.angle, [9.0, 24.0]),
            2.0,
            AMBER,
        );
        paint.line(
            world(position, self.angle, [-7.0, -2.0]),
            world(position, self.angle, [-7.0, 9.0]),
            1.0,
            [125, 101, 48, 255],
        );
        paint.line(
            world(position, self.angle, [7.0, -2.0]),
            world(position, self.angle, [7.0, 9.0]),
            1.0,
            [125, 101, 48, 255],
        );
        paint.circle(world(position, self.angle, [0.0, 1.0]), 3.0, 1.0, CYAN);
        if self.fuel < 25.0 {
            paint.text(self.x - 20.0, self.y - 36.0, "LOW FUEL", 1.0, RED);
        }
        let _ = hints;
    }

    fn altitude(&self) -> f32 {
        (self.terrain.y_at(self.x) - self.y).max(0.0)
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
    use crate::games::wrap;

    #[test]
    fn every_level_has_three_pads_on_the_terrain() {
        for level in 1..=20 {
            let terrain = Terrain::generate(level);
            assert_eq!(terrain.pads.len(), 3);
            for pad in &terrain.pads {
                assert!((terrain.y_at(pad.x0) - pad.y).abs() < 0.01);
                assert!((terrain.y_at(pad.x1) - pad.y).abs() < 0.01);
            }
        }
    }

    #[test]
    fn thrust_spends_fuel_and_changes_velocity() {
        let mut game = Game::new();
        let mut sounds = Vec::new();
        let vy = game.vy;
        for _ in 0..30 {
            game.tick(
                Input {
                    thrust: true,
                    ..Default::default()
                },
                &mut sounds,
            );
        }
        assert!(game.fuel < MAX_FUEL);
        assert!(game.vy < vy);
    }

    #[test]
    fn an_upright_soft_touchdown_scores() {
        let mut game = Game::new();
        let pad = game.terrain.pads[0].clone();
        game.x = pad.center();
        game.y = pad.y - 24.2;
        game.vx = 0.0;
        game.vy = 10.0;
        game.angle = 0.0;
        game.fuel = 50.0;
        let mut sounds = Vec::new();
        for _ in 0..4 {
            game.tick(Input::default(), &mut sounds);
        }
        assert_eq!(game.phase, Phase::Landed);
        assert!(game.score > 0);
        assert_eq!(game.landings, 1);
    }

    #[test]
    fn a_fast_impact_crashes() {
        let mut game = Game::new();
        let pad = game.terrain.pads[0].clone();
        game.x = pad.center();
        game.y = pad.y - 24.2;
        game.vx = 80.0;
        game.vy = 60.0;
        let mut sounds = Vec::new();
        game.tick(Input::default(), &mut sounds);
        assert_eq!(game.phase, Phase::Crashed);
        assert_eq!(game.crashes, 1);
    }

    #[test]
    fn autoplay_stays_finite_over_a_long_run() {
        let mut game = Game::new();
        let mut sounds = Vec::new();
        for _ in 0..12_000 {
            let input = game.autoplay();
            game.tick(input, &mut sounds);
            assert!(game.x.is_finite() && game.y.is_finite());
            assert!(game.fuel >= 0.0);
        }
    }

    #[test]
    fn particle_wrap_helpers_are_stable() {
        assert_eq!(wrap(-1.0, 960.0), 959.0);
        assert_eq!(wrap(960.0, 960.0), 0.0);
    }
}
