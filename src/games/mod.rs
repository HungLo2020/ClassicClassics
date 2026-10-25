//! Pure fixed-step simulations and their Quasar draw lists.

pub mod asteroids;
pub mod lunar;
pub mod pong;

use serde::{Deserialize, Serialize};

/// A small deterministic pseudo-random generator owned by each simulation.
/// Keeping the state in the game makes replays and state snapshots exact.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1),
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        (x >> 32) as u32
    }

    pub fn unit(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }

    pub fn index(&mut self, length: usize) -> usize {
        if length == 0 {
            0
        } else {
            self.next_u32() as usize % length
        }
    }
}

/// A short explosion/impact particle shared by the vector games.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Particle {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub life: f32,
    pub max_life: f32,
    pub tone: u8,
}

impl Particle {
    pub fn new(position: [f32; 2], velocity: [f32; 2], life: f32, tone: u8) -> Self {
        Self {
            position,
            velocity,
            life,
            max_life: life,
            tone,
        }
    }
}

pub fn update_particles(particles: &mut Vec<Particle>, dt: f32) {
    for particle in particles.iter_mut() {
        particle.position[0] += particle.velocity[0] * dt;
        particle.position[1] += particle.velocity[1] * dt;
        particle.velocity[0] *= 0.985;
        particle.velocity[1] *= 0.985;
        particle.life -= dt;
    }
    particles.retain(|particle| particle.life > 0.0);
}

/// Wraps a coordinate into `[0, size)`.
pub fn wrap(value: f32, size: f32) -> f32 {
    ((value % size) + size) % size
}
