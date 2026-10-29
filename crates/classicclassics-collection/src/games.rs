//! Register a game once here to connect catalog, CLI, dispatch and records.
use classicclassics_core::{ArcadeGame, ControlHints, GameInfo, Input, Sound, paint::Paint};
use quasar_render_2d::Canvas;
use serde::{Deserialize, Serialize};

macro_rules! games {
    ($($variant:ident => $game:ident, $field:ident, $number:literal;)+) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub enum GameKind { $($variant,)+ }

        impl GameKind {
            pub const ALL: [Self; [$(stringify!($variant)),+].len()] = [$(Self::$variant),+];

            pub fn info(self) -> &'static GameInfo {
                match self { $(Self::$variant => &$game::Game::INFO,)+ }
            }
            pub fn asset_manifest_dir(self) -> Option<&'static str> {
                match self { $(Self::$variant => $game::Game::ASSET_MANIFEST_DIR,)+ }
            }
            pub fn cover(self) -> Option<&'static [u8]> {
                match self { $(Self::$variant => $game::Game::COVER,)+ }
            }
            pub fn name(self) -> &'static str { self.info().name }
            pub fn tagline(self) -> &'static str { self.info().tagline }
            pub fn accent(self) -> [u8; 4] { self.info().accent }
            pub fn number(self) -> &'static str {
                match self { $(Self::$variant => $number,)+ }
            }
            pub fn from_alias(alias: &str) -> Option<Self> {
                Self::ALL.into_iter().find(|kind| kind.info().aliases.contains(&alias))
            }
            pub fn motif(self, paint: &mut Paint, cx: f32, cy: f32) {
                match self { $(Self::$variant => $game::Game::motif(paint, cx, cy, self.accent()),)+ }
            }
            pub fn catalog_record(self, records: &Records) -> (&'static str, String) {
                match self { $(Self::$variant => $game::Game::catalog_record(&records.$field),)+ }
            }
        }

        #[derive(Clone, Debug, Serialize, Deserialize)]
        #[serde(tag = "kind", content = "state", rename_all = "snake_case")]
        pub enum Game { $($variant(Box<$game::Game>),)+ }

        impl Game {
            pub fn new(kind: GameKind) -> Self {
                match kind { $(GameKind::$variant => Self::$variant(Box::default()),)+ }
            }
            pub fn load(kind: GameKind, root: Option<&classicclassics_assets::Directory>) -> Result<Self, String> {
                match kind { $(GameKind::$variant => $game::Game::load(root).map(|game| Self::$variant(Box::new(game))),)+ }
            }
            pub fn kind(&self) -> GameKind {
                match self { $(Self::$variant(_) => GameKind::$variant,)+ }
            }
            pub fn tick(&mut self, input: Input, sounds: &mut Vec<Sound>) {
                match self { $(Self::$variant(game) => game.tick(input, sounds),)+ }
            }
            pub fn paused(&self) -> bool {
                match self { $(Self::$variant(game) => game.paused(),)+ }
            }
            pub fn autoplay(&self) -> Input {
                match self { $(Self::$variant(game) => game.autoplay(),)+ }
            }
            pub fn pixel_frame(&self, hints: &ControlHints) -> Option<classicclassics_core::imported::PixelFrame> {
                match self { $(Self::$variant(game) => game.pixel_frame(hints),)+ }
            }
            pub fn scene(&self, hints: &ControlHints) -> Canvas {
                match self { $(Self::$variant(game) => game.scene(hints),)+ }
            }
            pub fn record(&mut self, records: &mut Records) {
                match self { $(Self::$variant(game) => game.record(&mut records.$field),)+ }
            }
            pub fn summary(&self) -> String {
                match self { $(Self::$variant(game) => game.summary(),)+ }
            }
        }

        /// Version-one JSON layout remains compatible with existing saves.
        #[derive(Clone, Debug, Serialize, Deserialize)]
        #[serde(default)]
        pub struct Records {
            pub version: u32,
            $(pub $field: $game::Record,)+
        }
        impl Default for Records {
            fn default() -> Self {
                Self { version: 1, $($field: Default::default(),)+ }
            }
        }
    }
}

use classicclassics_asteroids as asteroids;
use classicclassics_lunar as lunar;
use classicclassics_pong as pong;

use classicclassics_jackal as jackal;
use classicclassics_micromages as micromages;
use classicclassics_missile_command as missile;
use classicclassics_river_raid as river;

games! {
    Pong => pong, pong, "01";
    LunarLander => lunar, lunar, "02";
    Asteroids => asteroids, asteroids, "03";
    MicroMages => micromages, micromages, "04";
    Jackal => jackal, jackal, "05";
    MissileCommand => missile, missile, "06";
    RiverRaid => river, river, "07";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_games_roundtrip_and_continue_deterministically() {
        let mut aliases = std::collections::HashSet::new();
        for kind in GameKind::ALL {
            assert!(!kind.info().aliases.is_empty());
            for alias in kind.info().aliases {
                assert!(aliases.insert(*alias), "duplicate game alias: {alias}");
                assert_eq!(GameKind::from_alias(alias), Some(kind));
            }
            let mut game = Game::new(kind);
            let mut sounds = Vec::new();
            for _ in 0..1200 {
                game.tick(game.autoplay(), &mut sounds);
            }
            let snapshot = serde_json::to_value(&game).unwrap();
            let mut restored: Game = serde_json::from_value(snapshot).unwrap();
            assert_eq!(restored.kind(), kind);
            let mut records = Records::default();
            let mut restored_records = Records::default();
            for _ in 0..120 {
                let input = game.autoplay();
                let mut expected_sounds = Vec::new();
                let mut actual_sounds = Vec::new();
                game.tick(input, &mut expected_sounds);
                restored.tick(input, &mut actual_sounds);
                game.record(&mut records);
                restored.record(&mut restored_records);
                assert_eq!(expected_sounds, actual_sounds);
            }
            assert_eq!(
                serde_json::to_value(&game).unwrap(),
                serde_json::to_value(&restored).unwrap()
            );
            assert_eq!(
                serde_json::to_value(records).unwrap(),
                serde_json::to_value(restored_records).unwrap()
            );
            game.scene(&ControlHints::default())
                .validate(&Default::default())
                .unwrap();
        }
    }

    #[test]
    fn version_one_record_fields_still_load_and_missing_games_default() {
        let old = serde_json::json!({
            "version": 1,
            "pong": { "best_rally": 17, "wins": 2, "matches": 4 },
            "lunar": { "best_score": 2500, "landings": 5, "crashes": 3 },
            "asteroids": { "best_score": 9000, "best_level": 6 }
        });
        let records: Records = serde_json::from_value(old.clone()).unwrap();
        let current = serde_json::to_value(records).unwrap();
        for key in ["version", "pong", "lunar", "asteroids"] {
            assert_eq!(current[key], old[key]);
        }
        let partial: Records =
            serde_json::from_str(r#"{"version":1,"pong":{"best_rally":17}}"#).unwrap();
        assert_eq!(partial.pong.best_rally, 17);
        assert_eq!(partial.asteroids.best_score, 0);
    }
}
