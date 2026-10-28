//! Asteroids: inertia, wrap-around space, splitting rocks and a vector ship.

use classicclassics_core::ControlHints;
use classicclassics_core::paint::{
    AMBER, CORAL, CYAN, DIM, GREEN, GRID, MUTED, Paint, WHITE, world,
};
use classicclassics_core::simulation::{Particle, Rng, update_particles, wrap};
use classicclassics_core::{ArcadeGame, GameInfo, Input, Sound};
use classicclassics_core::{HEIGHT, TICK_HZ, WIDTH};
use quasar_render_2d::Canvas;
use serde::{Deserialize, Serialize};

const SHIP_RADIUS: f32 = 12.0;
const BULLET_SPEED: f32 = 620.0;
const BULLET_LIFE: f32 = 0.9;
const FIRE_COOLDOWN: f32 = 0.16;
const THRUST_ACCEL: f32 = 320.0;
const TURN_SPEED: f32 = 3.5;
const MAX_SHIP_SPEED: f32 = 420.0;
const MAX_BULLETS: usize = 5;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asteroid {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub size: u8,
    pub radius: f32,
    pub angle: f32,
    pub spin: f32,
    pub shape: Vec<[f32; 2]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bullet {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Playing,
    Paused,
    GameOver,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub ship: [f32; 2],
    pub velocity: [f32; 2],
    pub angle: f32,
    pub lives: u32,
    pub score: u32,
    pub level: u32,
    pub phase: Phase,
    pub invulnerable: f32,
    pub cooldown: f32,
    pub hyperspace_cooldown: f32,
    pub bullets: Vec<Bullet>,
    pub asteroids: Vec<Asteroid>,
    pub particles: Vec<Particle>,
    pub ticks: u64,
    pub rng: Rng,
    #[serde(default)]
    thrusting: bool,
}

impl Game {
    pub fn new() -> Self {
        let mut game = Self {
            ship: [WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5],
            velocity: [0.0; 2],
            angle: 0.0,
            lives: 3,
            score: 0,
            level: 1,
            phase: Phase::Playing,
            invulnerable: 2.5,
            cooldown: 0.0,
            hyperspace_cooldown: 0.0,
            bullets: Vec::new(),
            asteroids: Vec::new(),
            particles: Vec::new(),
            ticks: 0,
            rng: Rng::new(0xA57E_2019),
            thrusting: false,
        };
        game.spawn_level();
        game
    }

    fn spawn_level(&mut self) {
        self.asteroids.clear();
        let count = (3 + self.level.min(6)) as usize;
        for _ in 0..count {
            self.spawn_wave_asteroid();
        }
    }

    fn spawn_wave_asteroid(&mut self) {
        let (x, y) = loop {
            let edge = self.rng.index(4);
            let (x, y) = match edge {
                0 => (self.rng.range(60.0, WIDTH as f32 - 60.0), 30.0),
                1 => (
                    self.rng.range(60.0, WIDTH as f32 - 60.0),
                    HEIGHT as f32 - 30.0,
                ),
                2 => (30.0, self.rng.range(60.0, HEIGHT as f32 - 60.0)),
                _ => (
                    WIDTH as f32 - 30.0,
                    self.rng.range(60.0, HEIGHT as f32 - 60.0),
                ),
            };
            let dx = wrapped_delta(x, self.ship[0], WIDTH as f32);
            let dy = wrapped_delta(y, self.ship[1], HEIGHT as f32);
            if dx.hypot(dy) > 220.0 {
                break (x, y);
            }
        };
        let toward_center = [
            wrapped_delta(WIDTH as f32 * 0.5, x, WIDTH as f32),
            wrapped_delta(HEIGHT as f32 * 0.5, y, HEIGHT as f32),
        ];
        let length = toward_center[0].hypot(toward_center[1]).max(0.01);
        let speed = self.rng.range(26.0, 56.0) + self.level.min(8) as f32 * 3.0;
        let jitter = self.rng.range(-0.65, 0.65);
        let (sin, cos) = jitter.sin_cos();
        let vx = (toward_center[0] / length * cos - toward_center[1] / length * sin) * speed;
        let vy = (toward_center[0] / length * sin + toward_center[1] / length * cos) * speed;
        let asteroid = self.make_asteroid(3, [x, y], [vx, vy]);
        self.asteroids.push(asteroid);
    }

    fn make_asteroid(&mut self, size: u8, position: [f32; 2], velocity: [f32; 2]) -> Asteroid {
        let radius = match size {
            3 => 56.0,
            2 => 33.0,
            _ => 19.0,
        };
        let count = 9 + size as usize;
        let mut shape = Vec::with_capacity(count);
        for index in 0..count {
            let angle = index as f32 / count as f32 * std::f32::consts::TAU;
            let wobble = self.rng.range(0.76, 1.04);
            shape.push([angle.cos() * wobble, angle.sin() * wobble]);
        }
        Asteroid {
            x: position[0],
            y: position[1],
            vx: velocity[0],
            vy: velocity[1],
            size,
            radius,
            angle: self.rng.range(0.0, std::f32::consts::TAU),
            spin: self.rng.range(-0.9, 0.9),
            shape,
        }
    }

    fn collide_bullets(&mut self, sounds: &mut Vec<Sound>) {
        let mut bullet_index = 0;
        while bullet_index < self.bullets.len() {
            let bullet = self.bullets[bullet_index].clone();
            let mut hit = None;
            for (index, asteroid) in self.asteroids.iter().enumerate() {
                let dx = wrapped_delta(bullet.x, asteroid.x, WIDTH as f32);
                let dy = wrapped_delta(bullet.y, asteroid.y, HEIGHT as f32);
                if dx.hypot(dy) <= asteroid.radius * 0.9 {
                    hit = Some(index);
                    break;
                }
            }
            if let Some(index) = hit {
                self.bullets.remove(bullet_index);
                let asteroid = self.asteroids.remove(index);
                self.score = self.score.saturating_add(match asteroid.size {
                    3 => 100,
                    2 => 50,
                    _ => 20,
                });
                self.break_apart(&asteroid, sounds);
            } else {
                bullet_index += 1;
            }
        }
    }

    fn break_apart(&mut self, asteroid: &Asteroid, sounds: &mut Vec<Sound>) {
        sounds.push(Sound::Explosion);
        for index in 0..10 {
            let angle = index as f32 / 10.0 * std::f32::consts::TAU;
            let speed = 30.0 + (index % 4) as f32 * 18.0;
            self.particles.push(Particle::new(
                [asteroid.x, asteroid.y],
                [angle.cos() * speed, angle.sin() * speed],
                0.55,
                if index % 2 == 0 { 0 } else { 1 },
            ));
        }
        if asteroid.size <= 1 {
            return;
        }
        let child = asteroid.size - 1;
        let base = asteroid.vx.hypot(asteroid.vy).max(28.0) + 18.0;
        let direction = asteroid.vy.atan2(asteroid.vx);
        for offset in [-0.55f32, 0.55] {
            let angle = direction + offset;
            let velocity = [
                angle.cos() * base + self.rng.range(-8.0, 8.0),
                angle.sin() * base + self.rng.range(-8.0, 8.0),
            ];
            let position = [
                wrap(asteroid.x + offset.cos() * 12.0, WIDTH as f32),
                wrap(asteroid.y + offset.sin() * 12.0, HEIGHT as f32),
            ];
            let child = self.make_asteroid(child, position, velocity);
            self.asteroids.push(child);
        }
    }

    fn collide_ship(&mut self, sounds: &mut Vec<Sound>) {
        if self.invulnerable > 0.0 {
            return;
        }
        let hit = self.asteroids.iter().any(|asteroid| {
            let dx = wrapped_delta(self.ship[0], asteroid.x, WIDTH as f32);
            let dy = wrapped_delta(self.ship[1], asteroid.y, HEIGHT as f32);
            dx.hypot(dy) < asteroid.radius * 0.82 + SHIP_RADIUS * 0.6
        });
        if !hit {
            return;
        }
        self.lives = self.lives.saturating_sub(1);
        self.explode_ship(sounds);
        if self.lives == 0 {
            self.phase = Phase::GameOver;
            sounds.push(Sound::GameOver);
        } else {
            self.ship = [WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5];
            self.velocity = [0.0, 0.0];
            self.angle = 0.0;
            self.invulnerable = 2.6;
            self.bullets.clear();
        }
    }

    fn explode_ship(&mut self, sounds: &mut Vec<Sound>) {
        sounds.push(Sound::Explosion);
        for index in 0..28 {
            let angle = index as f32 / 28.0 * std::f32::consts::TAU;
            let speed = 40.0 + (index % 5) as f32 * 20.0;
            self.particles.push(Particle::new(
                self.ship,
                [angle.cos() * speed, angle.sin() * speed],
                0.95,
                if index % 3 == 0 { 0 } else { 1 },
            ));
        }
    }

    fn draw_asteroid(&self, paint: &mut Paint, asteroid: &Asteroid) {
        for offset_x in [-(WIDTH as f32), 0.0, WIDTH as f32] {
            for offset_y in [-(HEIGHT as f32), 0.0, HEIGHT as f32] {
                let x = asteroid.x + offset_x;
                let y = asteroid.y + offset_y;
                if x < -asteroid.radius * 2.0
                    || x > WIDTH as f32 + asteroid.radius * 2.0
                    || y < -asteroid.radius * 2.0
                    || y > HEIGHT as f32 + asteroid.radius * 2.0
                {
                    continue;
                }
                let points: Vec<[f32; 2]> = asteroid
                    .shape
                    .iter()
                    .map(|point| {
                        let rotated = classicclassics_core::paint::rotate(*point, asteroid.angle);
                        [
                            rotated[0] * asteroid.radius + x,
                            rotated[1] * asteroid.radius + y,
                        ]
                    })
                    .collect();
                let halo = if asteroid.size >= 3 {
                    [64, 39, 33, 255]
                } else {
                    [49, 34, 31, 255]
                };
                paint.glow_polyline(&points, 1.3, halo, [174, 121, 103, 255], true);
            }
        }
    }

    fn draw_ship(&self, paint: &mut Paint) {
        for offset_x in [-(WIDTH as f32), 0.0, WIDTH as f32] {
            for offset_y in [-(HEIGHT as f32), 0.0, HEIGHT as f32] {
                let x = self.ship[0] + offset_x;
                let y = self.ship[1] + offset_y;
                if x < -40.0 || x > WIDTH as f32 + 40.0 || y < -40.0 || y > HEIGHT as f32 + 40.0 {
                    continue;
                }
                let position = [x, y];
                let body = [
                    world(position, self.angle, [0.0, -17.0]),
                    world(position, self.angle, [11.0, 12.0]),
                    world(position, self.angle, [0.0, 6.0]),
                    world(position, self.angle, [-11.0, 12.0]),
                ];
                paint.glow_polyline(&body, 1.5, [40, 67, 55, 255], CYAN, true);
                if self.thrusting {
                    paint.line(
                        world(position, self.angle, [-5.0, 10.0]),
                        world(position, self.angle, [0.0, 25.0]),
                        1.5,
                        AMBER,
                    );
                    paint.line(
                        world(position, self.angle, [5.0, 10.0]),
                        world(position, self.angle, [0.0, 25.0]),
                        1.5,
                        AMBER,
                    );
                    paint.line(
                        world(position, self.angle, [0.0, 10.0]),
                        world(position, self.angle, [0.0, 31.0]),
                        1.0,
                        [255, 226, 158, 255],
                    );
                }
            }
        }
    }
}

fn normalize_angle(angle: f32) -> f32 {
    let mut angle = angle;
    while angle > std::f32::consts::PI {
        angle -= std::f32::consts::TAU;
    }
    while angle < -std::f32::consts::PI {
        angle += std::f32::consts::TAU;
    }
    angle
}

fn wrapped_delta(a: f32, b: f32, size: f32) -> f32 {
    let mut delta = a - b;
    if delta > size * 0.5 {
        delta -= size;
    } else if delta < -size * 0.5 {
        delta += size;
    }
    delta
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Record {
    pub best_score: u32,
    pub best_level: u32,
}

impl ArcadeGame for Game {
    type Record = Record;
    const INFO: GameInfo = GameInfo {
        name: "ASTEROIDS",
        aliases: &["asteroids"],
        tagline: "EVERY ROCK SPLITS.",
        accent: classicclassics_core::paint::CORAL,
    };

    fn tick(&mut self, input: Input, sounds: &mut Vec<Sound>) {
        self.ticks += 1;
        if input.restart {
            *self = Self::new();
            sounds.push(Sound::MenuSelect);
            return;
        }
        if input.pause && self.phase != Phase::GameOver {
            self.phase = match self.phase {
                Phase::Playing => Phase::Paused,
                Phase::Paused => Phase::Playing,
                other => other,
            };
            return;
        }
        if self.phase == Phase::GameOver {
            if input.fire_pressed || input.confirm {
                *self = Self::new();
                sounds.push(Sound::MenuSelect);
            }
            return;
        }
        if self.phase == Phase::Paused {
            return;
        }

        let dt = 1.0 / TICK_HZ as f32;
        self.invulnerable = (self.invulnerable - dt).max(0.0);
        self.cooldown = (self.cooldown - dt).max(0.0);
        self.hyperspace_cooldown = (self.hyperspace_cooldown - dt).max(0.0);
        update_particles(&mut self.particles, dt);

        self.angle += input.rotate.clamp(-1, 1) as f32 * TURN_SPEED * dt;
        if self.angle > std::f32::consts::PI {
            self.angle -= std::f32::consts::TAU;
        } else if self.angle < -std::f32::consts::PI {
            self.angle += std::f32::consts::TAU;
        }
        let heading = [self.angle.sin(), -self.angle.cos()];
        self.thrusting = input.thrust;
        if input.thrust {
            self.velocity[0] += heading[0] * THRUST_ACCEL * dt;
            self.velocity[1] += heading[1] * THRUST_ACCEL * dt;
            if (self.ticks % 11).is_multiple_of(11) {
                sounds.push(Sound::Thrust);
            }
        }
        let drag = (-0.45 * dt).exp();
        self.velocity[0] *= drag;
        self.velocity[1] *= drag;
        let speed = self.velocity[0].hypot(self.velocity[1]);
        if speed > MAX_SHIP_SPEED {
            let scale = MAX_SHIP_SPEED / speed;
            self.velocity[0] *= scale;
            self.velocity[1] *= scale;
        }
        self.ship[0] = wrap(self.ship[0] + self.velocity[0] * dt, WIDTH as f32);
        self.ship[1] = wrap(self.ship[1] + self.velocity[1] * dt, HEIGHT as f32);

        if input.hyperspace && self.hyperspace_cooldown <= 0.0 {
            self.ship = [
                self.rng.range(120.0, WIDTH as f32 - 120.0),
                self.rng.range(100.0, HEIGHT as f32 - 100.0),
            ];
            self.velocity = [0.0, 0.0];
            self.invulnerable = 1.6;
            self.hyperspace_cooldown = 3.0;
            sounds.push(Sound::Hyperspace);
        }

        let firing = input.fire || input.fire_pressed;
        if firing && self.cooldown <= 0.0 && self.bullets.len() < MAX_BULLETS {
            self.bullets.push(Bullet {
                x: self.ship[0] + heading[0] * 16.0,
                y: self.ship[1] + heading[1] * 16.0,
                vx: heading[0] * BULLET_SPEED + self.velocity[0] * 0.35,
                vy: heading[1] * BULLET_SPEED + self.velocity[1] * 0.35,
                life: BULLET_LIFE,
            });
            self.cooldown = FIRE_COOLDOWN;
            sounds.push(Sound::Laser);
        }

        for bullet in &mut self.bullets {
            bullet.x = wrap(bullet.x + bullet.vx * dt, WIDTH as f32);
            bullet.y = wrap(bullet.y + bullet.vy * dt, HEIGHT as f32);
            bullet.life -= dt;
        }
        self.bullets.retain(|bullet| bullet.life > 0.0);

        for asteroid in &mut self.asteroids {
            asteroid.x = wrap(asteroid.x + asteroid.vx * dt, WIDTH as f32);
            asteroid.y = wrap(asteroid.y + asteroid.vy * dt, HEIGHT as f32);
            asteroid.angle += asteroid.spin * dt;
        }

        self.collide_bullets(sounds);
        self.collide_ship(sounds);

        if self.asteroids.is_empty() {
            self.level = self.level.saturating_add(1);
            if self.level.is_multiple_of(2) {
                self.lives = self.lives.saturating_add(1);
                sounds.push(Sound::ExtraLife);
            } else {
                sounds.push(Sound::Score);
            }
            self.spawn_level();
        }
    }

    fn record(&mut self, records: &mut Record) {
        records.best_score = records.best_score.max(self.score);
        records.best_level = records.best_level.max(self.level);
    }

    fn autoplay(&self) -> Input {
        if self.phase == Phase::GameOver {
            return Input {
                fire_pressed: true,
                ..Default::default()
            };
        }
        let target = self.asteroids.iter().min_by(|a, b| {
            let da = wrapped_delta(self.ship[0], a.x, WIDTH as f32).hypot(wrapped_delta(
                self.ship[1],
                a.y,
                HEIGHT as f32,
            ));
            let db = wrapped_delta(self.ship[0], b.x, WIDTH as f32).hypot(wrapped_delta(
                self.ship[1],
                b.y,
                HEIGHT as f32,
            ));
            da.total_cmp(&db)
        });
        let Some(target) = target else {
            return Input::default();
        };
        let dx = wrapped_delta(target.x, self.ship[0], WIDTH as f32);
        let dy = wrapped_delta(target.y, self.ship[1], HEIGHT as f32);
        let desired = dx.atan2(-dy);
        let difference = normalize_angle(desired - self.angle);
        let rotate = if difference > 0.08 {
            1
        } else if difference < -0.08 {
            -1
        } else {
            0
        };
        let distance = dx.hypot(dy);
        Input {
            rotate,
            thrust: distance > 280.0,
            fire: difference.abs() < 0.20,
            ..Default::default()
        }
    }

    fn summary(&self) -> String {
        format!(
            "phase={:?} level={} score={} lives={} asteroids={} bullets={}",
            self.phase,
            self.level,
            self.score,
            self.lives,
            self.asteroids.len(),
            self.bullets.len()
        )
    }

    fn scene(&self, hints: &ControlHints) -> Canvas {
        let mut paint = Paint::new(WIDTH, HEIGHT);
        paint.background();
        paint.starfield(0xA57E2019, 180, [43, 50, 62, 255]);

        paint.text(60.0, 26.0, "ASTEROIDS", 3.0, WHITE);
        paint.text(61.0, 58.0, "QUASAR ARCADE / VECTOR SPACE 03", 1.0, MUTED);
        paint.text(60.0, 88.0, &format!("SCORE {:06}", self.score), 2.0, CORAL);
        paint.text(60.0, 118.0, &format!("LEVEL {:02}", self.level), 1.0, MUTED);
        paint.right(896.0, 30.0, "LIVES", 1.0, DIM);
        for index in 0..self.lives.min(8) {
            let x = 896.0 - 26.0 - index as f32 * 25.0;
            let y = 48.0;
            let ship = [
                [x, y - 8.0],
                [x + 8.0, y + 7.0],
                [x, y + 3.0],
                [x - 8.0, y + 7.0],
            ];
            paint.glow_polyline(&ship, 1.0, [58, 35, 31, 255], CORAL, true);
        }
        paint.right(
            896.0,
            76.0,
            &format!("{}  HYPERSPACE", hints.hyperspace),
            1.0,
            if self.hyperspace_cooldown > 0.0 {
                DIM
            } else {
                CYAN
            },
        );

        for asteroid in &self.asteroids {
            self.draw_asteroid(&mut paint, asteroid);
        }
        for bullet in &self.bullets {
            paint.rect(
                bullet.x - 5.0,
                bullet.y - 5.0,
                10.0,
                10.0,
                [71, 104, 112, 255],
            );
            paint.rect(bullet.x - 2.0, bullet.y - 2.0, 4.0, 4.0, WHITE);
        }
        let visible = self.invulnerable <= 0.0 || (self.ticks / 5).is_multiple_of(2);
        if visible && self.phase != Phase::GameOver {
            self.draw_ship(&mut paint);
        }
        for particle in &self.particles {
            let color = match particle.tone {
                0 => [255, 200, 105, 255],
                _ => [255, 118, 92, 255],
            };
            let alpha = (particle.life / particle.max_life).clamp(0.0, 1.0);
            let length = 2.0 + alpha * 8.0;
            let magnitude = particle.velocity[0].hypot(particle.velocity[1]).max(0.01);
            paint.line(
                particle.position,
                [
                    particle.position[0] - particle.velocity[0] / magnitude * length,
                    particle.position[1] - particle.velocity[1] / magnitude * length,
                ],
                1.5,
                color,
            );
        }

        let overlay = match self.phase {
            Phase::Paused => Some(("PAUSED", format!("{} TO RESUME", hints.pause))),
            Phase::GameOver => Some(("GAME OVER", format!("{} TO FLY AGAIN", hints.fire))),
            Phase::Playing => None,
        };
        if let Some((title, subtitle)) = overlay {
            paint.dim_rect(286.0, 232.0, 388.0, 120.0);
            paint.outline(286.0, 232.0, 388.0, 120.0, GRID);
            paint.rect(310.0, 250.0, 28.0, 2.0, CORAL);
            paint.centered(262.0, title, 3.0, WHITE);
            paint.centered(310.0, &subtitle, 1.0, MUTED);
        }

        paint.text(60.0, 526.0, &format!("{}  TURN", hints.rotate), 1.0, CORAL);
        paint.text(
            258.0,
            526.0,
            &format!("{}  THRUST", hints.thrust),
            1.0,
            AMBER,
        );
        paint.text(398.0, 526.0, &format!("{}  FIRE", hints.fire), 1.0, CYAN);
        paint.text(
            510.0,
            526.0,
            &format!("{}  HYPERSPACE", hints.hyperspace),
            1.0,
            GREEN,
        );
        paint.text(
            700.0,
            526.0,
            &format!("{}  RESTART", hints.restart),
            1.0,
            MUTED,
        );
        paint.text(
            60.0,
            550.0,
            &format!("{}  SETTINGS   {}  CATALOG", hints.settings, hints.back),
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
                "INERTIA ON   WRAP-AROUND SPACE   {} SHIPS IN RESERVE",
                self.lives
            ),
            1.0,
            [63, 82, 94, 255],
        );
        paint.into_canvas()
    }

    fn catalog_record(record: &Record) -> (&'static str, String) {
        ("BEST SCORE", record.best_score.to_string())
    }

    fn motif(paint: &mut Paint, cx: f32, cy: f32, accent: [u8; 4]) {
        use classicclassics_core::paint::{WHITE, world};
        let rocks = [
            (cx - 55.0, cy - 24.0, 20.0, 0.4),
            (cx + 45.0, cy + 18.0, 29.0, 2.1),
            (cx + 14.0, cy - 44.0, 12.0, 1.2),
        ];
        for (x, y, radius, angle) in rocks {
            let points: Vec<[f32; 2]> = (0..9)
                .map(|index| {
                    let a = index as f32 / 9.0 * std::f32::consts::TAU;
                    let wobble = 0.72 + ((index * 37) % 11) as f32 * 0.035;
                    [
                        x + (a + angle).cos() * radius * wobble,
                        y + (a + angle).sin() * radius * wobble,
                    ]
                })
                .collect();
            paint.glow_polyline(&points, 1.2, [65, 38, 32, 255], [166, 111, 94, 255], true);
        }
        let ship = [cx - 8.0, cy + 28.0];
        let body = [
            world(ship, -1.57, [0.0, -15.0]),
            world(ship, -1.57, [10.0, 10.0]),
            world(ship, -1.57, [0.0, 5.0]),
            world(ship, -1.57, [-10.0, 10.0]),
        ];
        paint.glow_polyline(&body, 1.4, [42, 64, 48, 255], accent, true);
        paint.line(
            world(ship, -1.57, [0.0, 5.0]),
            world(ship, -1.57, [-4.0, 18.0]),
            1.0,
            [255, 210, 120, 255],
        );
        paint.line(
            world(ship, -1.57, [0.0, 5.0]),
            world(ship, -1.57, [4.0, 18.0]),
            1.0,
            [255, 210, 120, 255],
        );
        paint.rect(cx - 30.0, cy - 8.0, 3.0, 3.0, WHITE);
        paint.rect(cx - 56.0, cy - 12.0, 2.0, 2.0, WHITE);
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
    fn level_starts_with_asteroids_and_lives() {
        let game = Game::new();
        assert!(!game.asteroids.is_empty());
        assert_eq!(game.lives, 3);
        assert_eq!(game.phase, Phase::Playing);
    }

    #[test]
    fn a_bullet_splits_a_large_asteroid() {
        let mut game = Game::new();
        game.asteroids.clear();
        let asteroid = game.make_asteroid(3, [300.0, 300.0], [0.0, 0.0]);
        game.asteroids.push(asteroid);
        game.bullets.push(Bullet {
            x: 300.0,
            y: 300.0,
            vx: 0.0,
            vy: 0.0,
            life: 1.0,
        });
        let mut sounds = Vec::new();
        game.collide_bullets(&mut sounds);
        assert_eq!(game.score, 100);
        assert_eq!(game.asteroids.len(), 2);
        assert!(game.asteroids.iter().all(|asteroid| asteroid.size == 2));
    }

    #[test]
    fn ship_collision_costs_a_life_and_grants_invulnerability() {
        let mut game = Game::new();
        game.asteroids.clear();
        let asteroid = game.make_asteroid(3, game.ship, [0.0, 0.0]);
        game.asteroids.push(asteroid);
        game.invulnerable = 0.0;
        let mut sounds = Vec::new();
        game.collide_ship(&mut sounds);
        assert_eq!(game.lives, 2);
        assert!(game.invulnerable > 0.0);
    }

    #[test]
    fn wrapping_delta_uses_the_shortest_path() {
        assert_eq!(wrapped_delta(10.0, 950.0, 960.0), 20.0);
        assert_eq!(wrapped_delta(950.0, 10.0, 960.0), -20.0);
        assert_eq!(wrapped_delta(300.0, 200.0, 960.0), 100.0);
    }

    #[test]
    fn autoplay_is_finite_and_fires() {
        let mut game = Game::new();
        let mut sounds = Vec::new();
        let mut shots = 0;
        for _ in 0..6_000 {
            let input = game.autoplay();
            if input.fire {
                shots += 1;
            }
            game.tick(input, &mut sounds);
            assert!(game.ship[0].is_finite() && game.ship[1].is_finite());
        }
        assert!(shots > 0);
        assert!(game.asteroids.len() <= 80);
    }

    #[test]
    fn clearing_the_field_advances_the_level() {
        let mut game = Game::new();
        game.asteroids.clear();
        let mut sounds = Vec::new();
        game.tick(Input::default(), &mut sounds);
        assert_eq!(game.level, 2);
        assert!(!game.asteroids.is_empty());
    }
}
