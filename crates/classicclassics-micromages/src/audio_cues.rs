pub const MUSIC: [(&str, bool, f64, f64); 9] = [
    ("music/mysterious.ogg", true, 0.0, 0.0),
    ("music/heroic.ogg", true, 0.0, 0.0),
    ("music/spooky.ogg", true, 0.0, 0.0),
    ("music/dreamy.ogg", true, 1.596, 39.934),
    ("music/evil.ogg", true, 0.0, 0.0),
    ("music/intro.ogg", true, 15.174, 24.759),
    ("music/fanfare.ogg", false, 0.0, 0.0),
    ("music/boss.ogg", true, 0.0, 0.0),
    ("music/march.ogg", true, 0.0, 0.0),
];
pub const SFX: [(&str, u32); 34] = [
    ("sfx/emerald.ogg", 8),
    ("sfx/spiderweb.ogg", 8),
    ("sfx/destroy block.ogg", 8),
    ("sfx/hero jump.ogg", 6),
    ("sfx/spell.ogg", 5),
    ("sfx/spell2.ogg", 7),
    ("sfx/damage block.ogg", 6),
    ("sfx/damage enemy.ogg", 6),
    ("sfx/destroy enemy.ogg", 8),
    ("sfx/hero damage.ogg", 9),
    ("sfx/hero head.ogg", 7),
    ("sfx/hero bounce.ogg", 8),
    ("sfx/fairy.ogg", 9),
    ("sfx/extinguish.ogg", 8),
    ("sfx/boss dash.ogg", 10),
    ("sfx/boss inhale.ogg", 10),
    ("sfx/open chest.ogg", 8),
    ("sfx/duck flap.ogg", 7),
    ("sfx/goat bubble.ogg", 7),
    ("sfx/troll spear.ogg", 7),
    ("sfx/wstar.ogg", 9),
    ("sfx/checkpoint.ogg", 10),
    ("sfx/spell charge.ogg", 8),
    ("sfx/pause.ogg", 15),
    ("sfx/unpause.ogg", 15),
    ("sfx/level exit.ogg", 10),
    ("sfx/level exit2.ogg", 10),
    ("sfx/hero ghost.ogg", 9),
    ("sfx/boss damage.ogg", 9),
    ("sfx/portal.ogg", 8),
    ("sfx/spark.ogg", 8),
    ("sfx/snake.ogg", 9),
    ("sfx/boss dash charge.ogg", 10),
    ("sfx/willowisp scream.ogg", 9),
];

pub fn cue(event: crate::events::GameEvent) -> Option<classicclassics_core::Sound> {
    use crate::events::GameEvent;
    use classicclassics_core::audio::{AssetSound, MusicLoop};
    Some(classicclassics_core::Sound::Asset(match event {
        GameEvent::Sound(effect) => AssetSound::Effect {
            game: "micromages",
            file: SFX[effect.index()].0,
        },
        GameEvent::Music(music) => {
            let (file, looping, start, end) = MUSIC[music.index()];
            AssetSound::Music {
                game: "micromages",
                file,
                region: if !looping {
                    MusicLoop::Once
                } else if end > start {
                    MusicLoop::Milliseconds((start * 1000.0) as u64, (end * 1000.0) as u64)
                } else {
                    MusicLoop::Whole
                },
            }
        }
        GameEvent::StopMusic | GameEvent::FadeMusic => AssetSound::StopMusic,
        _ => return None,
    }))
}
