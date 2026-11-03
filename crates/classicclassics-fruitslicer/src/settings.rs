//! Basalt-aware persistence: player settings in `config/shared`, display and
//! records in the machine-specific/save categories. All writes are atomic.

use crate::game::{Difficulty, Mode};
use quasar_storage::{GamePaths, read_json, write_json};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AudioSettings {
    pub master: f32,
    pub sfx: f32,
    pub music: f32,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master: 0.85,
            sfx: 0.9,
            music: 0.55,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct GameplaySettings {
    pub difficulty: String,
    pub show_fps: bool,
    pub reduced_flash: bool,
}

impl Default for GameplaySettings {
    fn default() -> Self {
        Self {
            difficulty: "normal".into(),
            show_fps: false,
            reduced_flash: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub audio: AudioSettings,
    pub gameplay: GameplaySettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            audio: AudioSettings::default(),
            gameplay: GameplaySettings::default(),
        }
    }
}

impl Settings {
    pub fn path(paths: &GamePaths) -> PathBuf {
        paths.shared_config.join("settings.toml")
    }

    pub fn load(paths: &GamePaths) -> Result<Self, String> {
        let path = Self::path(paths);
        if path.is_file() {
            Self::load_file(&path)
        } else {
            Ok(Self::default())
        }
    }

    pub fn load_file(path: &Path) -> Result<Self, String> {
        let mut settings = quasar_config::load_file_typed::<Settings>(path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if settings.version != 1 {
            return Err(format!(
                "{}: unsupported settings version {}",
                path.display(),
                settings.version
            ));
        }
        settings.normalize();
        Ok(settings)
    }

    pub fn save(&self, paths: &GamePaths) -> Result<(), String> {
        let text = toml::to_string_pretty(self).map_err(|error| error.to_string())?;
        let path = Self::path(paths);
        let parent = path.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let temporary = parent.join(".settings.toml.tmp");
        let mut file = std::fs::File::create(&temporary).map_err(|error| error.to_string())?;
        file.write_all(text.as_bytes())
            .map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        drop(file);
        std::fs::rename(&temporary, &path).map_err(|error| error.to_string())
    }

    pub fn difficulty(&self) -> Difficulty {
        Difficulty::parse(&self.gameplay.difficulty).unwrap_or(Difficulty::Normal)
    }

    pub fn normalize(&mut self) {
        self.audio.master = self.audio.master.clamp(0.0, 1.0);
        self.audio.sfx = self.audio.sfx.clamp(0.0, 1.0);
        self.audio.music = self.audio.music.clamp(0.0, 1.0);
        if Difficulty::parse(&self.gameplay.difficulty).is_none() {
            self.gameplay.difficulty = Difficulty::Normal.key().into();
        }
    }

    /// Applies `--set KEY=VALUE` overrides for the documented settings keys.
    pub fn apply_overrides(&mut self, overrides: &[(String, String)]) -> Result<(), String> {
        for (key, value) in overrides {
            match key.as_str() {
                "audio.master" => {
                    self.audio.master = value
                        .parse()
                        .map_err(|_| format!("{key}: expected a number"))?
                }
                "audio.sfx" => {
                    self.audio.sfx = value
                        .parse()
                        .map_err(|_| format!("{key}: expected a number"))?
                }
                "audio.music" => {
                    self.audio.music = value
                        .parse()
                        .map_err(|_| format!("{key}: expected a number"))?
                }
                "gameplay.difficulty" => {
                    let difficulty = Difficulty::parse(value)
                        .ok_or_else(|| format!("{key}: expected easy, normal or hard"))?;
                    self.gameplay.difficulty = difficulty.key().into();
                }
                "gameplay.show_fps" => {
                    self.gameplay.show_fps = value
                        .parse()
                        .map_err(|_| format!("{key}: expected true or false"))?
                }
                "gameplay.reduced_flash" => {
                    self.gameplay.reduced_flash = value
                        .parse()
                        .map_err(|_| format!("{key}: expected true or false"))?
                }
                other => return Err(format!("unknown configuration key '{other}'")),
            }
        }
        self.normalize();
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DisplaySettings {
    pub version: u32,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub linear: bool,
    pub integer_scale: bool,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            version: 1,
            width: 1280,
            height: 720,
            fullscreen: false,
            linear: true,
            integer_scale: false,
        }
    }
}

impl DisplaySettings {
    pub fn path(paths: &GamePaths) -> PathBuf {
        paths.config.join("display.json")
    }

    pub fn load(paths: &GamePaths) -> Result<Self, String> {
        let path = Self::path(paths);
        match read_json::<Self>(&path) {
            Ok(Some(mut display)) if display.version == 1 => {
                display.normalize();
                Ok(display)
            }
            Ok(Some(display)) => Err(format!(
                "{}: unsupported display version {}",
                path.display(),
                display.version
            )),
            Ok(None) => Ok(Self::default()),
            Err(error) => Err(format!("{}: {error}", path.display())),
        }
    }

    pub fn save(&self, paths: &GamePaths) -> Result<(), String> {
        write_json(&Self::path(paths), self).map_err(|error| error.to_string())
    }

    pub fn options(&self) -> quasar_render_2d::DisplayOptions {
        quasar_render_2d::DisplayOptions {
            filter: if self.linear {
                quasar_render_2d::Filter::Linear
            } else {
                quasar_render_2d::Filter::Nearest
            },
            scaling: if self.integer_scale {
                quasar_render_2d::Scaling::Integer
            } else {
                quasar_render_2d::Scaling::Fit
            },
            aspect: None,
        }
    }

    fn normalize(&mut self) {
        self.width = self.width.clamp(640, 16_384);
        self.height = self.height.clamp(360, 16_384);
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ModeRecord {
    pub best: i64,
    pub runs: u32,
    pub best_combo: u32,
    pub total_sliced: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Records {
    pub version: u32,
    pub classic: ModeRecord,
    pub arcade: ModeRecord,
    pub zen: ModeRecord,
}

impl Default for Records {
    fn default() -> Self {
        Self {
            version: 1,
            classic: ModeRecord::default(),
            arcade: ModeRecord::default(),
            zen: ModeRecord::default(),
        }
    }
}

impl Records {
    pub fn path(paths: &GamePaths) -> PathBuf {
        paths.save.join("records.json")
    }

    pub fn load(paths: &GamePaths) -> Result<Self, String> {
        let path = Self::path(paths);
        match read_json::<Self>(&path) {
            Ok(Some(records)) if records.version == 1 => Ok(records),
            Ok(Some(records)) => Err(format!(
                "{}: unsupported record version {}",
                path.display(),
                records.version
            )),
            Ok(None) => Ok(Self::default()),
            Err(error) => Err(format!("{}: {error}", path.display())),
        }
    }

    pub fn save(&self, paths: &GamePaths) -> Result<(), String> {
        write_json(&Self::path(paths), self).map_err(|error| error.to_string())
    }

    pub fn get(&self, mode: Mode) -> &ModeRecord {
        match mode {
            Mode::Classic => &self.classic,
            Mode::Arcade => &self.arcade,
            Mode::Zen => &self.zen,
        }
    }

    pub fn get_mut(&mut self, mode: Mode) -> &mut ModeRecord {
        match mode {
            Mode::Classic => &mut self.classic,
            Mode::Arcade => &mut self.arcade,
            Mode::Zen => &mut self.zen,
        }
    }

    pub fn best(&self, mode: Mode) -> i64 {
        self.get(mode).best
    }

    /// Adds a finished run and returns true when it set a new best score.
    pub fn record_run(&mut self, mode: Mode, score: i64, combo: u32, sliced: u32) -> bool {
        let record = self.get_mut(mode);
        record.runs = record.runs.saturating_add(1);
        record.total_sliced = record.total_sliced.saturating_add(u64::from(sliced));
        record.best_combo = record.best_combo.max(combo);
        let new_best = score > record.best;
        if new_best {
            record.best = score;
        }
        new_best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_are_atomic_and_mode_scoped() {
        let directory = tempfile::tempdir().unwrap();
        let paths = GamePaths::discover(Some(directory.path())).unwrap();
        let mut records = Records::default();
        assert!(records.record_run(Mode::Classic, 120, 4, 12));
        assert!(!records.record_run(Mode::Classic, 80, 2, 4));
        assert_eq!(records.best(Mode::Classic), 120);
        assert_eq!(records.best(Mode::Zen), 0);
        records.save(&paths).unwrap();
        let loaded = Records::load(&paths).unwrap();
        assert_eq!(loaded, records);
    }

    #[test]
    fn settings_round_trip_toml() {
        let directory = tempfile::tempdir().unwrap();
        let paths = GamePaths::discover(Some(directory.path())).unwrap();
        let mut settings = Settings::default();
        settings.audio.master = 0.42;
        settings.gameplay.difficulty = "hard".into();
        settings.save(&paths).unwrap();
        let loaded = Settings::load(&paths).unwrap();
        assert_eq!(loaded.audio.master, 0.42);
        assert_eq!(loaded.difficulty(), Difficulty::Hard);
    }
}
