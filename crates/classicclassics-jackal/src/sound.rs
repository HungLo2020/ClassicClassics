//! Device-independent audio event identities retained from Jackal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MusicTrack {
    Title,
    Briefing,
    Stage(u8),
    Boss,
    Victory,
    GameOver,
    Ending,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SoundEffect {
    MachineGun,
    Grenade,
    Rocket,
    Explosion,
    PlayerDeath,
    InfantryDeath,
    Rescue,
    Pickup,
    Delivery,
}
fn music_id(track: MusicTrack) -> u8 {
    match track {
        MusicTrack::Title => 0x30,
        MusicTrack::Briefing => 0x34,
        MusicTrack::Stage(stage) => [0x38, 0x3c, 0x40][stage as usize % 3],
        MusicTrack::Boss => 0x23,
        MusicTrack::Victory => 0x4c,
        MusicTrack::GameOver => 0x50,
        MusicTrack::Ending => 0x54,
    }
}
fn effect_id(effect: SoundEffect) -> u8 {
    match effect {
        SoundEffect::MachineGun => 0x09,
        SoundEffect::Grenade => 0x11,
        SoundEffect::Rocket => 0x13,
        SoundEffect::Explosion => 0x15,
        SoundEffect::PlayerDeath => 0x25,
        SoundEffect::InfantryDeath => 0x12,
        SoundEffect::Rescue => 0x1d,
        SoundEffect::Pickup => 0x21,
        SoundEffect::Delivery => 0x1f,
    }
}

pub fn cue(event: crate::game::GameEvent) -> Option<classicclassics_core::Sound> {
    use classicclassics_core::audio::{AssetSound, MusicLoop};
    use crate::game::GameEvent;
    let id = match event { GameEvent::Music(music) => music_id(music), GameEvent::Effect(effect) => effect_id(effect), GameEvent::AudioCommand(id) => id, GameEvent::Pause(_) => return None };
    if matches!(id, 0 | 0x59) { return Some(classicclassics_core::Sound::Asset(AssetSound::StopMusic)); }
    let (file, start) = match id {
        1 => ("audio/effect_01.wav", None),
        3 => ("audio/effect_03.wav", None),
        4 => ("audio/effect_04.wav", None),
        5 => ("audio/effect_05.wav", None),
        6 => ("audio/effect_06.wav", None),
        7 => ("audio/effect_07.wav", None),
        8 => ("audio/effect_08.wav", None),
        9 => ("audio/machine_gun.wav", None),
        10 => ("audio/effect_0a.wav", None),
        11 => ("audio/effect_0b.wav", None),
        12 => ("audio/effect_0c.wav", None),
        14 => ("audio/effect_0e.wav", None),
        16 => ("audio/effect_10.wav", None),
        17 => ("audio/grenade.wav", None),
        18 => ("audio/effect_12.wav", None),
        19 => ("audio/effect_13.wav", None),
        21 => ("audio/explosion.wav", None),
        22 => ("audio/effect_16.wav", None),
        23 => ("audio/effect_17.wav", None),
        25 => ("audio/effect_19.wav", None),
        27 => ("audio/effect_1b.wav", None),
        29 => ("audio/rescue.wav", None),
        31 => ("audio/delivery.wav", None),
        33 => ("audio/pickup.wav", None),
        35 => ("audio/boss.wav", Some(490906)),
        37 => ("audio/player_death.wav", None),
        38 => ("audio/effect_26.wav", None),
        39 => ("audio/effect_27.wav", None),
        40 => ("audio/effect_28.wav", Some(248021)),
        42 => ("audio/effect_2a.wav", None),
        45 => ("audio/effect_2d.wav", None),
        46 => ("audio/effect_2e.wav", None),
        48 => ("audio/title.wav", None),
        52 => ("audio/briefing.wav", Some(564285)),
        56 => ("audio/stage_1.wav", Some(245820)),
        60 => ("audio/stage_2.wav", Some(785890)),
        64 => ("audio/stage_3.wav", Some(23481)),
        68 => ("audio/alternate_stage.wav", Some(436606)),
        72 => ("audio/helicopter.wav", Some(661146)),
        76 => ("audio/victory.wav", None),
        80 => ("audio/game_over.wav", None),
        84 => ("audio/ending.wav", Some(1074271)),
        88 => ("audio/effect_58.wav", None),
        _ => return None,
    };
    let music = matches!(id, 0x23 | 0x30 | 0x34 | 0x38 | 0x3c | 0x40 | 0x44 | 0x48 | 0x4c | 0x50 | 0x54);
    Some(classicclassics_core::Sound::Asset(if music { AssetSound::Music { game: "jackal", file, region: start.map_or(MusicLoop::Once, MusicLoop::FromFrame) } } else { AssetSound::Effect { game: "jackal", file } }))
}
