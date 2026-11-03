//! Deterministic FruitSlicer simulation, independent of windowing, GPU and audio.
//!
//! The game is a fixed-step, seeded state machine. Rendering consumes the
//! public state, and audio consumes [`GameEvent`] values. This keeps headless
//! replays, tests and the desktop application on exactly the same rules.

use quasar_sim::{Simulation, StateHash, StateHasher, Tick};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;

/// Logical canvas width in pixels.
pub const WIDTH: f32 = 1280.0;
/// Logical canvas height in pixels.
pub const HEIGHT: f32 = 720.0;
/// Fixed simulation rate. Slicing is checked every tick.
pub const TICK_HZ: f64 = 120.0;
/// Seconds per fixed tick.
pub const TICK_DT: f32 = 1.0 / 120.0;
/// Gravity in logical pixels per second squared at normal difficulty.
pub const GRAVITY: f32 = 1200.0;
/// Sprite cell size in the generated fruit atlas.
pub const ART_CELL: u32 = 256;
/// Drawn fruit cell size in logical pixels before per-fruit scale.
pub const FRUIT_DRAW: f32 = 188.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }
    pub fn normalized(self) -> Self {
        let length = self.length();
        if length > 1e-6 {
            Self::new(self.x / length, self.y / length)
        } else {
            Self::ZERO
        }
    }
    pub fn rotated(self, radians: f32) -> Self {
        let (sin, cos) = radians.sin_cos();
        Self::new(self.x * cos - self.y * sin, self.x * sin + self.y * cos)
    }
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self::new(
            self.x + (other.x - self.x) * t,
            self.y + (other.y - self.y) * t,
        )
    }
}

impl std::ops::Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}
impl std::ops::AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}
impl std::ops::Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}
impl std::ops::Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

/// Game selector shown on the main menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Classic,
    Arcade,
    Zen,
}

impl Mode {
    pub const ALL: [Self; 3] = [Self::Classic, Self::Arcade, Self::Zen];
    pub fn label(self) -> &'static str {
        match self {
            Self::Classic => "CLASSIC",
            Self::Arcade => "ARCADE",
            Self::Zen => "ZEN",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Classic => "THREE LIVES. ONE BOMB ENDS THE RUN.",
            Self::Arcade => "SIXTY SECONDS. BOMBS COST POINTS.",
            Self::Zen => "NINETY RELAXED SECONDS. NO BOMBS.",
        }
    }
    pub fn duration(self) -> Option<f32> {
        match self {
            Self::Classic => None,
            Self::Arcade => Some(60.0),
            Self::Zen => Some(90.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    pub const ALL: [Self; 3] = [Self::Easy, Self::Normal, Self::Hard];
    pub fn label(self) -> &'static str {
        match self {
            Self::Easy => "EASY",
            Self::Normal => "NORMAL",
            Self::Hard => "HARD",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Easy => "easy",
            Self::Normal => "normal",
            Self::Hard => "hard",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "easy" => Some(Self::Easy),
            "normal" | "medium" => Some(Self::Normal),
            "hard" => Some(Self::Hard),
            _ => None,
        }
    }
    fn gravity(self) -> f32 {
        match self {
            Self::Easy => 1050.0,
            Self::Normal => GRAVITY,
            Self::Hard => 1340.0,
        }
    }
    fn speed(self) -> f32 {
        match self {
            Self::Easy => 0.94,
            Self::Normal => 1.0,
            Self::Hard => 1.07,
        }
    }
    fn bomb_chance(self) -> f32 {
        match self {
            Self::Easy => 0.07,
            Self::Normal => 0.12,
            Self::Hard => 0.18,
        }
    }
    fn spawn_pressure(self) -> f32 {
        match self {
            Self::Easy => 0.7,
            Self::Normal => 1.0,
            Self::Hard => 1.35,
        }
    }
}

/// Every sliceable object type. `art_row` indexes the generated sprite atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Fruit(FruitKind),
    Bomb,
    Ice,
    Frenzy,
    Bonus,
    Heart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FruitKind {
    Watermelon,
    Apple,
    Orange,
    Banana,
    Pineapple,
    Strawberry,
    Kiwi,
    Grapes,
    Dragonfruit,
    Lemon,
    Coconut,
    Pomegranate,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KindInfo {
    pub art_row: u32,
    pub radius: f32,
    pub value: i64,
    pub juice: [u8; 4],
    pub weight: u32,
}

impl FruitKind {
    pub const ALL: [Self; 12] = [
        Self::Watermelon,
        Self::Apple,
        Self::Orange,
        Self::Banana,
        Self::Pineapple,
        Self::Strawberry,
        Self::Kiwi,
        Self::Grapes,
        Self::Dragonfruit,
        Self::Lemon,
        Self::Coconut,
        Self::Pomegranate,
    ];
    pub fn info(self) -> KindInfo {
        match self {
            Self::Watermelon => KindInfo {
                art_row: 0,
                radius: 0.40,
                value: 15,
                juice: [226, 58, 74, 255],
                weight: 8,
            },
            Self::Apple => KindInfo {
                art_row: 1,
                radius: 0.35,
                value: 10,
                juice: [236, 84, 72, 255],
                weight: 12,
            },
            Self::Orange => KindInfo {
                art_row: 2,
                radius: 0.36,
                value: 10,
                juice: [246, 154, 48, 255],
                weight: 11,
            },
            Self::Banana => KindInfo {
                art_row: 3,
                radius: 0.35,
                value: 10,
                juice: [250, 222, 112, 255],
                weight: 9,
            },
            Self::Pineapple => KindInfo {
                art_row: 4,
                radius: 0.38,
                value: 15,
                juice: [250, 202, 74, 255],
                weight: 7,
            },
            Self::Strawberry => KindInfo {
                art_row: 5,
                radius: 0.31,
                value: 8,
                juice: [238, 66, 96, 255],
                weight: 10,
            },
            Self::Kiwi => KindInfo {
                art_row: 6,
                radius: 0.32,
                value: 8,
                juice: [142, 188, 84, 255],
                weight: 9,
            },
            Self::Grapes => KindInfo {
                art_row: 7,
                radius: 0.35,
                value: 8,
                juice: [142, 96, 190, 255],
                weight: 8,
            },
            Self::Dragonfruit => KindInfo {
                art_row: 8,
                radius: 0.37,
                value: 12,
                juice: [236, 106, 174, 255],
                weight: 5,
            },
            Self::Lemon => KindInfo {
                art_row: 9,
                radius: 0.33,
                value: 8,
                juice: [248, 226, 94, 255],
                weight: 9,
            },
            Self::Coconut => KindInfo {
                art_row: 10,
                radius: 0.37,
                value: 10,
                juice: [196, 150, 106, 255],
                weight: 6,
            },
            Self::Pomegranate => KindInfo {
                art_row: 11,
                radius: 0.35,
                value: 12,
                juice: [198, 44, 72, 255],
                weight: 6,
            },
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Watermelon => "WATERMELON",
            Self::Apple => "APPLE",
            Self::Orange => "ORANGE",
            Self::Banana => "BANANA",
            Self::Pineapple => "PINEAPPLE",
            Self::Strawberry => "STRAWBERRY",
            Self::Kiwi => "KIWI",
            Self::Grapes => "GRAPES",
            Self::Dragonfruit => "DRAGONFRUIT",
            Self::Lemon => "LEMON",
            Self::Coconut => "COCONUT",
            Self::Pomegranate => "POMEGRANATE",
        }
    }
}

impl Kind {
    pub fn info(self) -> KindInfo {
        match self {
            Self::Fruit(fruit) => fruit.info(),
            Self::Bomb => KindInfo {
                art_row: 15,
                radius: 0.33,
                value: 0,
                juice: [70, 62, 58, 255],
                weight: 0,
            },
            Self::Ice => KindInfo {
                art_row: 12,
                radius: 0.36,
                value: 20,
                juice: [142, 224, 255, 255],
                weight: 0,
            },
            Self::Frenzy => KindInfo {
                art_row: 13,
                radius: 0.36,
                value: 20,
                juice: [255, 214, 110, 255],
                weight: 0,
            },
            Self::Bonus => KindInfo {
                art_row: 11,
                radius: 0.37,
                value: 0,
                juice: [214, 48, 78, 255],
                weight: 0,
            },
            Self::Heart => KindInfo {
                art_row: 14,
                radius: 0.34,
                value: 15,
                juice: [255, 126, 168, 255],
                weight: 0,
            },
        }
    }
    pub fn is_fruit(self) -> bool {
        matches!(self, Self::Fruit(_))
    }
    pub fn is_bomb(self) -> bool {
        matches!(self, Self::Bomb)
    }
    pub fn is_power(self) -> bool {
        matches!(self, Self::Ice | Self::Frenzy | Self::Bonus | Self::Heart)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Fruit(fruit) => fruit.label(),
            Self::Bomb => "BOMB",
            Self::Ice => "FROST FRUIT",
            Self::Frenzy => "FRENZY FRUIT",
            Self::Bonus => "BONUS POMEGRANATE",
            Self::Heart => "HEART FRUIT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Playing,
    Paused,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverReason {
    Bomb,
    Lives,
    Time,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fruit {
    pub id: u64,
    pub kind: Kind,
    pub position: Vec2,
    pub velocity: Vec2,
    pub rotation: f32,
    pub spin: f32,
    pub scale: f32,
    pub radius: f32,
    pub age: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Half {
    pub kind: Kind,
    /// 0 = left half, 1 = right half.
    pub side: u8,
    pub position: Vec2,
    pub velocity: Vec2,
    pub rotation: f32,
    pub spin: f32,
    pub scale: f32,
    pub life: f32,
    pub max_life: f32,
    pub age: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticleKind {
    Juice,
    Spark,
    Smoke,
    Seed,
    Glow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Particle {
    pub kind: ParticleKind,
    pub position: Vec2,
    pub velocity: Vec2,
    pub color: [u8; 4],
    pub size: f32,
    pub rotation: f32,
    pub spin: f32,
    pub drag: f32,
    pub gravity: f32,
    pub life: f32,
    pub max_life: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Splat {
    pub position: Vec2,
    pub color: [u8; 4],
    pub size: f32,
    pub rotation: f32,
    pub life: f32,
    pub max_life: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Floater {
    pub text: String,
    pub position: Vec2,
    pub velocity: Vec2,
    pub color: [u8; 4],
    pub size: f32,
    pub life: f32,
    pub max_life: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailNode {
    pub position: Vec2,
    pub age: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Stats {
    pub sliced: u32,
    pub bombs: u32,
    pub misses: u32,
    pub best_combo: u32,
    pub powers: u32,
}

/// Observable effects consumed by the audio layer. Rendering reads state
/// directly, so these are intentionally small and drained every frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameEvent {
    Slice { kind: Kind, pan: f32 },
    Bomb,
    Freeze,
    Frenzy,
    Bonus,
    Heart,
    Combo { count: u32 },
    Miss,
    GameOver(OverReason),
    BonusLife,
}

/// One fixed-step input frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct TickInput {
    pub pointer: Option<Vec2>,
    pub pointer_down: bool,
    pub start: bool,
    pub pause: bool,
    pub restart: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub mode: Mode,
    pub difficulty: Difficulty,
    pub phase: Phase,
    pub reason: Option<OverReason>,
    pub score: i64,
    pub lives: u8,
    pub time_left: f32,
    pub elapsed: f32,
    pub ticks: u64,
    pub combo: u32,
    pub combo_timer: f32,
    pub freeze_left: f32,
    pub frenzy_left: f32,
    pub over_timer: f32,
    pub shake: f32,
    pub flash: f32,
    pub flash_color: [u8; 4],
    pub time_scale: f32,
    pub fruits: Vec<Fruit>,
    pub halves: Vec<Half>,
    pub particles: Vec<Particle>,
    pub splats: Vec<Splat>,
    pub floaters: Vec<Floater>,
    pub trail: Vec<TrailNode>,
    pub stats: Stats,
    pub events: Vec<GameEvent>,
    seed: u64,
    rng: u64,
    next_id: u64,
    spawn_timer: f32,
    special_timer: f32,
    last_pointer: Option<Vec2>,
    pointer_down: bool,
    swipe_points: u32,
}

impl Game {
    pub fn new(mode: Mode, difficulty: Difficulty, seed: u64) -> Self {
        let mut game = Self {
            mode,
            difficulty,
            phase: Phase::Playing,
            reason: None,
            score: 0,
            lives: if mode == Mode::Classic { 3 } else { 0 },
            time_left: mode.duration().unwrap_or(0.0),
            elapsed: 0.0,
            ticks: 0,
            combo: 0,
            combo_timer: 0.0,
            freeze_left: 0.0,
            frenzy_left: 0.0,
            over_timer: 0.0,
            shake: 0.0,
            flash: 0.0,
            flash_color: [255, 255, 255, 255],
            time_scale: 1.0,
            fruits: Vec::new(),
            halves: Vec::new(),
            particles: Vec::new(),
            splats: Vec::new(),
            floaters: Vec::new(),
            trail: Vec::new(),
            stats: Stats::default(),
            events: Vec::new(),
            seed: if seed == 0 {
                0x9e37_79b9_7f4a_7c15
            } else {
                seed
            },
            rng: if seed == 0 {
                0x9e37_79b9_7f4a_7c15
            } else {
                seed
            },
            next_id: 1,
            spawn_timer: 0.9,
            special_timer: 9.0,
            last_pointer: None,
            pointer_down: false,
            swipe_points: 0,
        };
        game.spawn_wave(3);
        game
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Replaces the run while keeping the selected mode and difficulty.
    pub fn restart(&mut self) {
        let mode = self.mode;
        let difficulty = self.difficulty;
        let seed = self.seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        *self = Self::new(mode, difficulty, seed);
    }

    pub fn pause(&mut self) {
        if self.phase == Phase::Playing {
            self.phase = Phase::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.phase == Phase::Paused {
            self.phase = Phase::Playing;
        }
    }

    pub fn toggle_pause(&mut self) {
        match self.phase {
            Phase::Playing => self.phase = Phase::Paused,
            Phase::Paused => self.phase = Phase::Playing,
            Phase::GameOver => {}
        }
    }

    pub fn multiplier(&self) -> i64 {
        if self.frenzy_left > 0.0 { 2 } else { 1 }
    }

    pub fn in_frenzy(&self) -> bool {
        self.frenzy_left > 0.0
    }

    pub fn is_frozen(&self) -> bool {
        self.freeze_left > 0.0
    }

    pub fn combo_fraction(&self) -> f32 {
        (self.combo_timer / 0.62).clamp(0.0, 1.0)
    }

    pub fn take_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    pub fn fruit_positions(&self) -> impl Iterator<Item = (Vec2, Kind, f32)> + '_ {
        self.fruits
            .iter()
            .map(|fruit| (fruit.position, fruit.kind, fruit.radius))
    }

    /// Advances exactly one fixed step.
    pub fn tick(&mut self, input: TickInput) {
        self.ticks += 1;
        if self.events.len() > 256 {
            let excess = self.events.len() - 256;
            self.events.drain(0..excess);
        }
        if input.restart {
            self.restart();
            return;
        }
        if input.pause {
            self.toggle_pause();
            return;
        }
        self.shake = (self.shake - TICK_DT * 3.4).max(0.0);
        self.flash = (self.flash - TICK_DT * 2.8).max(0.0);
        match self.phase {
            Phase::Paused => {
                self.age_trail(TICK_DT);
            }
            Phase::GameOver => {
                self.over_timer += TICK_DT;
                self.age_effects(TICK_DT);
                if input.start && self.over_timer > 0.65 {
                    self.restart();
                }
            }
            Phase::Playing => {
                self.time_scale = if self.freeze_left > 0.0 { 0.34 } else { 1.0 };
                let dt = TICK_DT * self.time_scale;
                self.elapsed += dt;
                if let Some(_duration) = self.mode.duration() {
                    self.time_left = (self.time_left - dt).max(0.0);
                    if self.time_left <= 0.0 {
                        self.end(OverReason::Time);
                        return;
                    }
                }
                self.freeze_left = (self.freeze_left - TICK_DT).max(0.0);
                self.frenzy_left = (self.frenzy_left - dt).max(0.0);
                self.combo_timer = (self.combo_timer - TICK_DT).max(0.0);
                if self.combo_timer <= 0.0 {
                    self.combo = 0;
                }
                self.spawn_timer -= dt;
                self.special_timer -= dt;
                if self.spawn_timer <= 0.0 {
                    self.schedule_wave();
                }
                if self.special_timer <= 0.0 {
                    self.spawn_special();
                    self.special_timer = self.range(9.0, 15.0);
                }
                self.update_fruits(dt);
                self.process_pointer(input, dt);
                self.age_effects(dt);
                self.update_trail(dt, input.pointer, input.pointer_down);
            }
        }
    }

    fn schedule_wave(&mut self) {
        let pressure = self.difficulty.spawn_pressure()
            + (self.elapsed / 42.0).min(1.6)
            + if self.in_frenzy() { 1.4 } else { 0.0 };
        let base = 1 + (self.next_u64() % 3) as u32;
        let count = if self.elapsed < 5.0 {
            base.min(2)
        } else {
            (base + (pressure * 1.8) as u32).min(7)
        };
        self.spawn_wave(count);
        let interval = (1.30 - (self.elapsed / 65.0).min(0.55))
            / self.difficulty.spawn_pressure()
            / if self.in_frenzy() { 1.35 } else { 1.0 };
        self.spawn_timer = (interval * self.range(0.82, 1.16)).max(0.34);
    }

    fn spawn_wave(&mut self, count: u32) {
        let count = count.min(8);
        let mut bomb_slot = None;
        if self.mode != Mode::Zen && self.elapsed > 3.0 {
            let chance =
                self.difficulty.bomb_chance() + if self.elapsed > 30.0 { 0.03 } else { 0.0 };
            if self.chance(chance) {
                bomb_slot = Some(self.index(count as usize));
            }
        }
        for slot in 0..count {
            let kind = if bomb_slot == Some(slot as usize) {
                Kind::Bomb
            } else {
                self.pick_fruit()
            };
            let x = 132.0
                + (slot as f32 + 0.5) * (WIDTH - 264.0) / count as f32
                + self.range(-34.0, 34.0);
            let scale = self.range(0.82, 1.12);
            self.launch(kind, x, scale);
        }
    }

    fn spawn_special(&mut self) {
        let mut choices = Vec::new();
        if self.freeze_left <= 0.5 {
            choices.push(Kind::Ice);
        }
        if self.frenzy_left <= 0.5 {
            choices.push(Kind::Frenzy);
        }
        choices.push(Kind::Bonus);
        if self.mode == Mode::Classic && self.lives < 4 {
            choices.push(Kind::Heart);
            choices.push(Kind::Heart);
        }
        let kind = choices[self.index(choices.len())];
        let x = self.range(340.0, WIDTH - 340.0);
        let scale = self.range(1.0, 1.14);
        self.launch(kind, x, scale);
        // A special always travels with a small escort so it is worth chasing.
        for offset in [-150.0, 150.0] {
            let fruit = self.pick_fruit();
            self.launch(fruit, (x + offset).clamp(140.0, WIDTH - 140.0), 0.95);
        }
    }

    fn launch(&mut self, kind: Kind, x: f32, scale: f32) {
        let info = kind.info();
        let draw = FRUIT_DRAW * scale;
        let toward = (WIDTH * 0.5 - x) * 0.30;
        let velocity = Vec2::new(
            toward + self.range(-120.0, 120.0),
            -self.range(1080.0, 1430.0) * self.difficulty.speed(),
        );
        let id = self.next_id();
        let rotation = self.range(-0.5, 0.5);
        let spin = self.range(-3.4, 3.4);
        let fruit = Fruit {
            id,
            kind,
            position: Vec2::new(x, HEIGHT + 92.0),
            velocity,
            rotation,
            spin,
            scale: draw / FRUIT_DRAW,
            radius: FRUIT_DRAW * info.radius * (draw / FRUIT_DRAW),
            age: 0.0,
        };
        self.fruits.push(fruit);
    }

    fn update_fruits(&mut self, dt: f32) {
        let gravity = self.difficulty.gravity();
        let mut index = 0;
        while index < self.fruits.len() {
            let mut missed_kind = None;
            let mut remove = false;
            {
                let fruit = &mut self.fruits[index];
                fruit.velocity.y += gravity * dt;
                fruit.position += fruit.velocity * dt;
                fruit.rotation += fruit.spin * dt;
                fruit.age += dt;
                let offscreen = fruit.position.y > HEIGHT + 230.0
                    || fruit.position.x < -260.0
                    || fruit.position.x > WIDTH + 260.0;
                let was_visible = fruit.position.y < HEIGHT + 120.0;
                if was_visible && fruit.velocity.y > 0.0 && fruit.position.y > HEIGHT + 96.0 {
                    missed_kind = Some(fruit.kind);
                    remove = true;
                }
                if offscreen {
                    remove = true;
                }
            }
            if remove {
                self.fruits.remove(index);
            } else {
                index += 1;
            }
            if let Some(kind) = missed_kind
                && kind.is_fruit()
                && self.mode == Mode::Classic
            {
                self.stats.misses += 1;
                self.combo = 0;
                self.combo_timer = 0.0;
                self.lives = self.lives.saturating_sub(1);
                self.flash = 0.55;
                self.flash_color = [255, 58, 58, 255];
                self.shake = self.shake.max(0.32);
                self.events.push(GameEvent::Miss);
                if self.lives == 0 {
                    self.end(OverReason::Lives);
                    return;
                }
            }
        }
    }

    fn process_pointer(&mut self, input: TickInput, dt: f32) {
        let Some(pointer) = input.pointer else {
            if self.pointer_down != input.pointer_down {
                self.end_swipe();
            }
            self.pointer_down = input.pointer_down;
            self.last_pointer = None;
            return;
        };
        if input.pointer_down && !self.pointer_down {
            self.pointer_down = true;
            self.last_pointer = Some(pointer);
            self.swipe_points = 0;
            self.trail.clear();
            self.trail.push(TrailNode {
                position: pointer,
                age: 0.0,
            });
            return;
        }
        if !input.pointer_down {
            if self.pointer_down {
                self.end_swipe();
            }
            self.pointer_down = false;
            self.last_pointer = Some(pointer);
            return;
        }
        let Some(previous) = self.last_pointer else {
            self.last_pointer = Some(pointer);
            self.pointer_down = true;
            return;
        };
        self.pointer_down = true;
        let segment = pointer - previous;
        if segment.length() >= 3.0 {
            self.slice_segment(previous, pointer, dt);
            self.swipe_points += 1;
            self.trail.push(TrailNode {
                position: pointer,
                age: 0.0,
            });
            if self.trail.len() > 96 {
                self.trail.remove(0);
            }
        }
        self.last_pointer = Some(pointer);
    }

    fn end_swipe(&mut self) {
        self.swipe_points = 0;
    }

    fn slice_segment(&mut self, from: Vec2, to: Vec2, dt: f32) {
        let mut index = 0;
        let mut consumed_combo = false;
        while index < self.fruits.len() {
            let fruit = &self.fruits[index];
            if segment_hits_circle(from, to, fruit.position, fruit.radius * 1.06) {
                let fruit = self.fruits.remove(index);
                self.slice_fruit(fruit, from, to, dt);
                consumed_combo = true;
                if self.phase == Phase::GameOver {
                    break;
                }
            } else {
                index += 1;
            }
        }
        if consumed_combo && self.combo == 0 {
            self.combo = 1;
            self.combo_timer = 0.62;
        }
    }

    fn slice_fruit(&mut self, fruit: Fruit, from: Vec2, to: Vec2, _dt: f32) {
        let info = fruit.kind.info();
        let direction = (to - from).normalized();
        let slice_angle = direction.y.atan2(direction.x);
        let normal = Vec2::new(-direction.y, direction.x);
        let midpoint = fruit.position;

        if fruit.kind.is_bomb() {
            self.stats.bombs += 1;
            self.explode(midpoint);
            self.events.push(GameEvent::Bomb);
            match self.mode {
                Mode::Classic => {
                    self.end(OverReason::Bomb);
                }
                Mode::Arcade => {
                    self.score = (self.score - 20).max(0);
                    self.add_floater(midpoint, "-20", [255, 118, 96, 255], 40.0);
                    self.flash = 0.9;
                    self.flash_color = [255, 96, 66, 255];
                    self.shake = 1.0;
                }
                Mode::Zen => {}
            }
            return;
        }

        self.stats.sliced += 1;
        if fruit.kind.is_power() {
            self.stats.powers += 1;
        }
        self.combo += 1;
        self.combo_timer = 0.62;
        self.stats.best_combo = self.stats.best_combo.max(self.combo);

        let multiplier = self.multiplier();
        let combo_multiplier = 1.0 + (self.combo.saturating_sub(1).min(9)) as f32 * 0.15;
        let gained = (info.value as f32 * multiplier as f32 * combo_multiplier).round() as i64;
        if self.combo == 3 || self.combo == 5 || self.combo == 8 || self.combo == 10 {
            self.events.push(GameEvent::Combo { count: self.combo });
        }
        self.score += gained;

        let points_color = if multiplier > 1 {
            [255, 224, 128, 255]
        } else {
            [255, 244, 218, 255]
        };
        if gained > 0 {
            self.add_floater(
                midpoint + Vec2::new(0.0, -34.0),
                &format!("+{gained}"),
                points_color,
                34.0,
            );
        }

        self.spawn_halves(&fruit, slice_angle, normal);
        self.spawn_juice(midpoint, direction, info.juice, fruit.kind);
        self.add_splat(midpoint, info.juice, fruit.scale);

        let pan = ((midpoint.x / WIDTH) * 2.0 - 1.0).clamp(-1.0, 1.0);
        self.events.push(GameEvent::Slice {
            kind: fruit.kind,
            pan,
        });

        match fruit.kind {
            Kind::Ice => {
                self.freeze_left = 6.0;
                self.flash = 0.7;
                self.flash_color = [126, 218, 255, 255];
                self.events.push(GameEvent::Freeze);
                self.add_floater(
                    midpoint + Vec2::new(0.0, -86.0),
                    "FREEZE!",
                    [156, 230, 255, 255],
                    52.0,
                );
                self.spark_burst(midpoint, [176, 236, 255, 255], 34);
            }
            Kind::Frenzy => {
                self.frenzy_left = 8.0;
                self.flash = 0.6;
                self.flash_color = [255, 206, 96, 255];
                self.events.push(GameEvent::Frenzy);
                self.add_floater(
                    midpoint + Vec2::new(0.0, -86.0),
                    "FRENZY! 2X",
                    [255, 220, 120, 255],
                    52.0,
                );
                self.spark_burst(midpoint, [255, 226, 140, 255], 34);
            }
            Kind::Bonus => {
                self.score += 50;
                self.events.push(GameEvent::Bonus);
                self.add_floater(
                    midpoint + Vec2::new(0.0, -86.0),
                    "BONUS +50",
                    [255, 214, 128, 255],
                    48.0,
                );
                self.spark_burst(midpoint, [255, 206, 110, 255], 40);
                for offset in [-220.0, 0.0, 220.0] {
                    let kind = self.pick_fruit();
                    self.launch(
                        kind,
                        (midpoint.x + offset).clamp(150.0, WIDTH - 150.0),
                        0.92,
                    );
                }
            }
            Kind::Heart => {
                if self.lives < 5 {
                    self.lives += 1;
                    self.events.push(GameEvent::BonusLife);
                    self.add_floater(
                        midpoint + Vec2::new(0.0, -86.0),
                        "+1 LIFE",
                        [255, 150, 184, 255],
                        48.0,
                    );
                } else {
                    self.score += 15;
                    self.add_floater(
                        midpoint + Vec2::new(0.0, -86.0),
                        "+30",
                        [255, 150, 184, 255],
                        42.0,
                    );
                }
                self.events.push(GameEvent::Heart);
                self.spark_burst(midpoint, [255, 158, 190, 255], 30);
            }
            Kind::Fruit(_) | Kind::Bomb => {}
        }
    }

    fn spawn_halves(&mut self, fruit: &Fruit, slice_angle: f32, normal: Vec2) {
        let sprite_angle = slice_angle + std::f32::consts::FRAC_PI_2;
        for side in 0..2 {
            let sign = if side == 0 { -1.0 } else { 1.0 };
            let half = Half {
                kind: fruit.kind,
                side,
                position: fruit.position,
                velocity: fruit.velocity
                    + normal * sign * self.range(55.0, 130.0)
                    + Vec2::new(self.range(-30.0, 30.0), self.range(-40.0, 10.0)),
                rotation: sprite_angle,
                spin: fruit.spin * 0.6 + sign * self.range(0.8, 3.2),
                scale: fruit.scale,
                life: 1.7,
                max_life: 1.7,
                age: 0.0,
            };
            self.halves.push(half);
        }
    }

    fn spawn_juice(&mut self, at: Vec2, direction: Vec2, color: [u8; 4], kind: Kind) {
        let count = match kind {
            Kind::Fruit(fruit) => match fruit {
                FruitKind::Watermelon | FruitKind::Pineapple | FruitKind::Coconut => 34,
                _ => 24,
            },
            Kind::Bomb => 0,
            _ => 30,
        };
        for i in 0..count {
            let spread = self.range(-0.9, 0.9);
            let speed = self.range(85.0, 540.0);
            let dir = direction.rotated(spread);
            let mut tint = color;
            if i % 5 == 0 {
                tint = [255, 250, 238, 255];
            }
            let particle = Particle {
                kind: ParticleKind::Juice,
                position: at + Vec2::new(self.range(-14.0, 14.0), self.range(-14.0, 14.0)),
                velocity: dir * speed
                    + Vec2::new(self.range(-70.0, 70.0), self.range(-170.0, 40.0)),
                color: tint,
                size: self.range(4.0, 16.0),
                rotation: self.range(0.0, std::f32::consts::TAU),
                spin: self.range(-6.0, 6.0),
                drag: self.range(0.8, 2.2),
                gravity: self.range(700.0, 1250.0),
                life: self.range(0.45, 1.15),
                max_life: 1.15,
            };
            self.particles.push(particle);
        }
        let seeds = match kind {
            Kind::Fruit(FruitKind::Watermelon | FruitKind::Strawberry | FruitKind::Kiwi) => 7,
            Kind::Fruit(FruitKind::Pomegranate | FruitKind::Dragonfruit) => 10,
            _ => 0,
        };
        for _ in 0..seeds {
            let velocity = direction.rotated(self.range(-1.1, 1.1)) * self.range(120.0, 430.0);
            let particle = Particle {
                kind: ParticleKind::Seed,
                position: at,
                velocity,
                color: [44, 32, 24, 255],
                size: self.range(3.0, 6.0),
                rotation: self.range(0.0, std::f32::consts::TAU),
                spin: self.range(-8.0, 8.0),
                drag: 0.5,
                gravity: 1350.0,
                life: self.range(0.7, 1.4),
                max_life: 1.4,
            };
            self.particles.push(particle);
        }
    }

    fn explode(&mut self, at: Vec2) {
        for _ in 0..70 {
            let angle = self.range(0.0, std::f32::consts::TAU);
            let speed = self.range(60.0, 760.0);
            let dir = Vec2::new(angle.cos(), angle.sin());
            let kind = if self.chance(0.6) {
                ParticleKind::Spark
            } else {
                ParticleKind::Smoke
            };
            let color = if self.chance(0.5) {
                [255, 182, 82, 255]
            } else {
                [255, 238, 206, 255]
            };
            let particle = Particle {
                kind,
                position: at,
                velocity: dir * speed,
                color,
                size: self.range(7.0, 30.0),
                rotation: angle,
                spin: self.range(-5.0, 5.0),
                drag: 1.8,
                gravity: 240.0,
                life: self.range(0.35, 1.35),
                max_life: 1.35,
            };
            self.particles.push(particle);
        }
        for _ in 0..14 {
            let angle = self.range(0.0, std::f32::consts::TAU);
            let radius = self.range(0.0, 40.0);
            let speed = self.range(30.0, 130.0);
            let particle = Particle {
                kind: ParticleKind::Smoke,
                position: at + Vec2::new(angle.cos(), angle.sin()) * radius,
                velocity: Vec2::new(angle.cos(), angle.sin()) * speed,
                color: [82, 74, 70, 210],
                size: self.range(34.0, 72.0),
                rotation: angle,
                spin: self.range(-1.0, 1.0),
                drag: 1.4,
                gravity: -40.0,
                life: self.range(0.8, 1.8),
                max_life: 1.8,
            };
            self.particles.push(particle);
        }
    }

    fn spark_burst(&mut self, at: Vec2, color: [u8; 4], count: u32) {
        for _ in 0..count {
            let angle = self.range(0.0, std::f32::consts::TAU);
            let speed = self.range(90.0, 460.0);
            let particle = Particle {
                kind: ParticleKind::Spark,
                position: at,
                velocity: Vec2::new(angle.cos(), angle.sin()) * speed,
                color,
                size: self.range(8.0, 22.0),
                rotation: angle,
                spin: self.range(-7.0, 7.0),
                drag: 1.4,
                gravity: 120.0,
                life: self.range(0.35, 0.95),
                max_life: 0.95,
            };
            self.particles.push(particle);
        }
        for _ in 0..5 {
            let particle = Particle {
                kind: ParticleKind::Glow,
                position: at,
                velocity: Vec2::ZERO,
                color,
                size: self.range(60.0, 130.0),
                rotation: 0.0,
                spin: 0.0,
                drag: 0.0,
                gravity: 0.0,
                life: self.range(0.25, 0.6),
                max_life: 0.6,
            };
            self.particles.push(particle);
        }
    }

    fn add_splat(&mut self, at: Vec2, color: [u8; 4], scale: f32) {
        if self.splats.len() >= 34 {
            self.splats.remove(0);
        }
        let position = at + Vec2::new(self.range(-24.0, 24.0), self.range(-10.0, 40.0));
        let rotation = self.range(0.0, std::f32::consts::TAU);
        let size = self.range(58.0, 138.0) * scale;
        self.splats.push(Splat {
            position,
            color,
            size,
            rotation,
            life: 6.0,
            max_life: 6.0,
        });
    }

    fn add_floater(&mut self, at: Vec2, text: &str, color: [u8; 4], size: f32) {
        self.floaters.push(Floater {
            text: text.to_string(),
            position: at,
            velocity: Vec2::new(0.0, -46.0),
            color,
            size,
            life: 0.9,
            max_life: 0.9,
        });
        if self.floaters.len() > 24 {
            self.floaters.remove(0);
        }
    }

    fn update_trail(&mut self, dt: f32, pointer: Option<Vec2>, down: bool) {
        self.age_trail(dt);
        if down
            && let Some(pointer) = pointer
            && self
                .trail
                .last()
                .is_none_or(|node| (node.position - pointer).length() > 4.0)
        {
            self.trail.push(TrailNode {
                position: pointer,
                age: 0.0,
            });
            if self.trail.len() > 96 {
                self.trail.remove(0);
            }
        }
    }

    fn age_trail(&mut self, dt: f32) {
        for node in &mut self.trail {
            node.age += dt;
        }
        self.trail.retain(|node| node.age < 0.20);
    }

    fn age_effects(&mut self, dt: f32) {
        let gravity = self.difficulty.gravity();
        let mut index = 0;
        while index < self.halves.len() {
            let half = &mut self.halves[index];
            half.velocity.y += gravity * dt;
            half.position += half.velocity * dt;
            half.rotation += half.spin * dt;
            half.age += dt;
            half.life -= dt;
            if half.life <= 0.0
                || half.position.y > HEIGHT + 260.0
                || half.position.x < -300.0
                || half.position.x > WIDTH + 300.0
            {
                self.halves.remove(index);
            } else {
                index += 1;
            }
        }
        index = 0;
        while index < self.particles.len() {
            let particle = &mut self.particles[index];
            particle.velocity.y += particle.gravity * dt;
            let damping = (1.0 - particle.drag * dt).max(0.0);
            particle.velocity.x *= damping;
            particle.velocity.y *= damping;
            particle.position += particle.velocity * dt;
            particle.rotation += particle.spin * dt;
            particle.life -= dt;
            if particle.life <= 0.0 {
                self.particles.remove(index);
            } else {
                index += 1;
            }
        }
        for splat in &mut self.splats {
            splat.life -= dt;
        }
        self.splats.retain(|splat| splat.life > 0.0);
        for floater in &mut self.floaters {
            floater.position += floater.velocity * dt;
            floater.velocity.y *= (1.0 - 2.6 * dt).max(0.0);
            floater.life -= dt;
        }
        self.floaters.retain(|floater| floater.life > 0.0);
        if self.particles.len() > 900 {
            let excess = self.particles.len() - 900;
            self.particles.drain(0..excess);
        }
    }

    fn end(&mut self, reason: OverReason) {
        if self.phase == Phase::GameOver {
            return;
        }
        self.phase = Phase::GameOver;
        self.reason = Some(reason);
        self.over_timer = 0.0;
        self.events.push(GameEvent::GameOver(reason));
    }

    fn pick_fruit(&mut self) -> Kind {
        let total: u32 = FruitKind::ALL.iter().map(|kind| kind.info().weight).sum();
        let mut roll = (self.next_u64() % u64::from(total)) as u32;
        for kind in FruitKind::ALL {
            let weight = kind.info().weight;
            if roll < weight {
                return Kind::Fruit(kind);
            }
            roll -= weight;
        }
        Kind::Fruit(FruitKind::Apple)
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn next_u64(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u32 << 24) as f32
    }

    fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }

    fn chance(&mut self, probability: f32) -> bool {
        self.unit() < probability
    }

    fn index(&mut self, length: usize) -> usize {
        if length == 0 {
            0
        } else {
            (self.next_u64() % length as u64) as usize
        }
    }
}

impl Simulation for Game {
    type Input = TickInput;
    type Snapshot = Game;
    type Error = Infallible;

    fn update(&mut self, _tick: Tick, input: &Self::Input) -> Result<(), Self::Error> {
        self.tick(*input);
        Ok(())
    }

    fn state_hash(&self) -> StateHash {
        let mut hash = StateHasher::new();
        hash.write_u8(match self.phase {
            Phase::Playing => 0,
            Phase::Paused => 1,
            Phase::GameOver => 2,
        });
        hash.write_i64(self.score);
        hash.write_u8(self.lives);
        hash.write_u32(self.ticks as u32);
        hash.write_u64(self.rng);
        hash.write_u32(self.fruits.len() as u32);
        for fruit in &self.fruits {
            hash.write_u64(fruit.id);
            hash.write_f32_bits(fruit.position.x);
            hash.write_f32_bits(fruit.position.y);
            hash.write_f32_bits(fruit.velocity.x);
            hash.write_f32_bits(fruit.velocity.y);
        }
        hash.write_u32(self.halves.len() as u32);
        hash.write_u32(self.particles.len() as u32);
        hash.write_u64(self.combo as u64);
        hash.finish()
    }

    fn snapshot(&self) -> Self::Snapshot {
        self.clone()
    }

    fn restore(&mut self, snapshot: &Self::Snapshot) -> Result<(), Self::Error> {
        *self = snapshot.clone();
        Ok(())
    }
}

/// Closest-point test between a line segment and a circle.
pub fn segment_hits_circle(from: Vec2, to: Vec2, center: Vec2, radius: f32) -> bool {
    let segment = to - from;
    let length_squared = segment.x * segment.x + segment.y * segment.y;
    let t = if length_squared <= 1e-6 {
        0.0
    } else {
        (((center.x - from.x) * segment.x + (center.y - from.y) * segment.y) / length_squared)
            .clamp(0.0, 1.0)
    };
    let closest = Vec2::new(from.x + segment.x * t, from.y + segment.y * t);
    (center - closest).length() <= radius
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playing() -> Game {
        Game::new(Mode::Classic, Difficulty::Normal, 7)
    }

    #[test]
    fn segment_circle_geometry_hits_and_misses() {
        assert!(segment_hits_circle(
            Vec2::new(0.0, 10.0),
            Vec2::new(20.0, 10.0),
            Vec2::new(10.0, 10.0),
            2.0
        ));
        assert!(!segment_hits_circle(
            Vec2::new(0.0, 10.0),
            Vec2::new(20.0, 10.0),
            Vec2::new(10.0, 20.0),
            2.0
        ));
        assert!(segment_hits_circle(
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            1.5
        ));
    }

    #[test]
    fn slicing_a_fruit_scores_and_spawns_halves() {
        let mut game = playing();
        game.fruits.clear();
        game.fruits.push(Fruit {
            id: 1,
            kind: Kind::Fruit(FruitKind::Watermelon),
            position: Vec2::new(400.0, 300.0),
            velocity: Vec2::ZERO,
            rotation: 0.0,
            spin: 0.0,
            scale: 1.0,
            radius: 70.0,
            age: 0.0,
        });
        let before = game.score;
        game.slice_segment(Vec2::new(320.0, 300.0), Vec2::new(480.0, 300.0), TICK_DT);
        assert!(game.score > before);
        assert_eq!(game.halves.len(), 2);
        assert!(!game.particles.is_empty());
        assert_eq!(game.stats.sliced, 1);
    }

    #[test]
    fn bombs_end_classic_but_only_cost_points_in_arcade() {
        let mut classic = playing();
        classic.fruits.clear();
        classic.fruits.push(Fruit {
            id: 1,
            kind: Kind::Bomb,
            position: Vec2::new(400.0, 300.0),
            velocity: Vec2::ZERO,
            rotation: 0.0,
            spin: 0.0,
            scale: 1.0,
            radius: 55.0,
            age: 0.0,
        });
        classic.slice_segment(Vec2::new(350.0, 300.0), Vec2::new(450.0, 300.0), TICK_DT);
        assert_eq!(classic.phase, Phase::GameOver);
        assert_eq!(classic.reason, Some(OverReason::Bomb));

        let mut arcade = Game::new(Mode::Arcade, Difficulty::Normal, 7);
        arcade.score = 100;
        arcade.fruits.clear();
        arcade.fruits.push(Fruit {
            id: 1,
            kind: Kind::Bomb,
            position: Vec2::new(400.0, 300.0),
            velocity: Vec2::ZERO,
            rotation: 0.0,
            spin: 0.0,
            scale: 1.0,
            radius: 55.0,
            age: 0.0,
        });
        arcade.slice_segment(Vec2::new(350.0, 300.0), Vec2::new(450.0, 300.0), TICK_DT);
        assert_eq!(arcade.phase, Phase::Playing);
        assert_eq!(arcade.score, 80);
    }

    #[test]
    fn combo_scales_and_awards_bonus() {
        let mut game = playing();
        game.fruits.clear();
        for i in 0..4 {
            game.fruits.push(Fruit {
                id: i,
                kind: Kind::Fruit(FruitKind::Apple),
                position: Vec2::new(300.0 + i as f32 * 20.0, 300.0),
                velocity: Vec2::ZERO,
                rotation: 0.0,
                spin: 0.0,
                scale: 1.0,
                radius: 40.0,
                age: 0.0,
            });
        }
        game.slice_segment(Vec2::new(260.0, 300.0), Vec2::new(420.0, 300.0), TICK_DT);
        assert_eq!(game.combo, 4);
        assert_eq!(game.stats.best_combo, 4);
        assert!(game.score > 4 * 10);
    }

    #[test]
    fn freeze_and_frenzy_change_timing_and_multiplier() {
        let mut game = playing();
        game.freeze_left = 3.0;
        game.frenzy_left = 3.0;
        let before = game.time_scale;
        game.tick(TickInput::default());
        assert!((game.time_scale - 0.34).abs() < 0.001);
        assert_ne!(before, game.time_scale);
        assert_eq!(game.multiplier(), 2);
    }

    #[test]
    fn a_missed_fruit_costs_exactly_one_life() {
        let mut game = playing();
        game.fruits.clear();
        game.lives = 3;
        game.fruits.push(Fruit {
            id: 1,
            kind: Kind::Fruit(FruitKind::Apple),
            position: Vec2::new(400.0, HEIGHT + 100.0),
            velocity: Vec2::new(0.0, 120.0),
            rotation: 0.0,
            spin: 0.0,
            scale: 1.0,
            radius: 40.0,
            age: 0.0,
        });
        game.update_fruits(TICK_DT);
        assert_eq!(game.lives, 2);
        assert!(game.fruits.is_empty());
        assert_eq!(game.stats.misses, 1);
    }

    #[test]
    fn deterministic_runs_hash_identically() {
        use quasar_sim::{Recorder, Replay, SimulationConfig};
        let config = SimulationConfig::new(1, 42, "fruitslicer-test");
        let mut recorder = Recorder::new(playing(), config);
        for _ in 0..600 {
            recorder.tick(TickInput::default()).unwrap();
        }
        let journal = recorder.journal().clone();
        let mut replay = Replay::new(playing(), journal).unwrap();
        let summary = replay.run_to_end().unwrap();
        assert_eq!(summary.ticks, 600);
    }
}
