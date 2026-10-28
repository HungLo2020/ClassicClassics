#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    Asset(AssetSound),
    MenuMove,
    MenuSelect,
    Paddle,
    Wall,
    Score,
    Win,
    Thrust,
    Explosion,
    Laser,
    Landing,
    Crash,
    ExtraLife,
    GameOver,
    Hyperspace,
}

/// Original asset cues are data; only the collection opens an audio device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetSound {
    Effect {
        game: &'static str,
        file: &'static str,
    },
    Music {
        game: &'static str,
        file: &'static str,
        region: MusicLoop,
    },
    StopMusic,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicLoop {
    Once,
    Whole,
    FromFrame(u64),
    Milliseconds(u64, u64),
}
