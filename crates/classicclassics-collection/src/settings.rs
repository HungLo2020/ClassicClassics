//! Machine-local settings, resolved by Quasar Config and persisted as TOML.
use crate::Result;
use quasar_config::{CommonArgs, Config, ConfigValue};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    /// Use the current monitor's dimensions; width/height remain the fallback.
    pub auto_resolution: bool,
    pub linear: bool,
    pub integer: bool,
    pub audio: bool,
    pub volume: f32,
    pub muted: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            width: 1920,
            height: 1080,
            fullscreen: true,
            auto_resolution: true,
            linear: false,
            integer: false,
            audio: true,
            volume: 0.8,
            muted: false,
        }
    }
}

fn key(name: &str) -> Result<&'static str> {
    Ok(match name {
        "version" => "version",
        "width" | "window.width" => "width",
        "height" | "window.height" => "height",
        "fullscreen" | "window.fullscreen" => "fullscreen",
        "auto_resolution" | "window.auto_resolution" => "auto_resolution",
        "linear" | "display.linear" => "linear",
        "integer" | "display.integer" => "integer",
        "audio" | "audio.enabled" => "audio",
        "volume" | "audio.volume" => "volume",
        "muted" | "audio.muted" => "muted",
        other => return Err(format!("unknown setting '{other}'").into()),
    })
}

impl Settings {
    /// Keep a deterministic HD fallback when monitor discovery is unavailable.
    pub fn resolve_monitor(&mut self, monitor: Option<(u32, u32)>) {
        if self.auto_resolution
            && let Some((width, height)) = monitor
            && (640..=16384).contains(&width)
            && (400..=16384).contains(&height)
        {
            self.width = width;
            self.height = height;
        }
    }

    pub fn valid(&self) -> bool {
        self.version == 1
            && (640..=16384).contains(&self.width)
            && (400..=16384).contains(&self.height)
            && self.volume.is_finite()
            && (0.0..=1.0).contains(&self.volume)
    }

    fn config(&self) -> Result<Config> {
        let mut values = Config::new();
        values.load_json_str(&serde_json::to_string(self)?)?;
        let mut config = Config::new();
        for (key, value, _) in values.entries() {
            config.set_default(key, value.clone())?;
        }
        Ok(config)
    }

    fn merge_file(config: &mut Config, path: &Path) -> Result {
        let mut file = Config::new();
        file.load_file(path)?;
        let mut has_size = false;
        let mut has_auto = false;
        let mut seen = std::collections::HashSet::new();
        for (name, value, _) in file.entries() {
            let name = key(name)?;
            if !seen.insert(name) {
                return Err(format!("duplicate setting alias '{name}'").into());
            }
            has_size |= name == "width" || name == "height";
            has_auto |= name == "auto_resolution";
            config.set_file(name, value.clone())?;
        }
        if has_size && !has_auto {
            config.set_file("auto_resolution", false)?;
        }
        Ok(())
    }

    fn from_config(config: Config) -> Result<Self> {
        let settings: Self = config.deserialize()?;
        if !settings.valid() {
            return Err("configuration settings are outside the supported range or version".into());
        }
        Ok(settings)
    }

    /// Quasar handles parsing, typed conversion and default/file/CLI precedence.
    /// Explicit run configuration is never persisted over the player's settings.
    pub fn apply(&mut self, common: &CommonArgs) -> Result {
        let mut config = self.config()?;
        if let Some(path) = &common.config {
            Self::merge_file(&mut config, path)?;
        }
        for (name, value) in &common.overrides {
            let name = key(name)?;
            config.set_cli(name, ConfigValue::parse(value))?;
            if name == "width" || name == "height" {
                config.set_cli("auto_resolution", false)?;
            }
        }
        if let Some(fullscreen) = common.fullscreen {
            config.set_cli("fullscreen", fullscreen)?;
        }
        if let Some(size) = common.resolution {
            config.set_cli("width", size.width)?;
            config.set_cli("height", size.height)?;
            config.set_cli("auto_resolution", false)?;
        }
        if common.no_audio == Some(true) {
            config.set_cli("audio", false)?;
        }
        *self = Self::from_config(config)?;
        Ok(())
    }

    /// Prefer TOML. Load legacy JSON only when TOML is absent; never overwrite
    /// a corrupt/newer file or fall back past it to older preferences.
    pub fn load(path: &Path) -> (Self, bool) {
        let load = || -> Result<Self> {
            let Some(source) = crate::config_file::source(path)? else {
                return Ok(Self::default());
            };
            let mut config = Self::default().config()?;
            Self::merge_file(&mut config, &source)?;
            Self::from_config(config)
        };
        match load() {
            Ok(settings) => (settings, true),
            Err(error) => {
                eprintln!(
                    "Cannot load display settings ({error}); preserving file and using defaults"
                );
                (Self::default(), false)
            }
        }
    }

    pub fn save(&self, path: &Path) -> Result {
        if !self.valid() {
            return Err("refusing to save invalid settings".into());
        }
        crate::config_file::write(path, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_fullscreen_with_monitor_detection_and_hd_fallback() {
        let mut settings = Settings::default();
        assert_eq!((settings.width, settings.height), (1920, 1080));
        assert!(settings.fullscreen && settings.auto_resolution);
        settings.resolve_monitor(None);
        assert_eq!((settings.width, settings.height), (1920, 1080));
        settings.resolve_monitor(Some((2560, 1440)));
        assert_eq!((settings.width, settings.height), (2560, 1440));
        settings.resolve_monitor(Some((0, 0)));
        assert_eq!((settings.width, settings.height), (2560, 1440));
        settings.auto_resolution = false;
        settings.resolve_monitor(Some((1920, 1080)));
        assert_eq!((settings.width, settings.height), (2560, 1440));
    }

    #[test]
    fn toml_roundtrip_legacy_migration_and_corruption_protection() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("display.toml");
        assert!(Settings::load(&path).1);
        let legacy = path.with_extension("json");
        let old = r#"{"version":1,"width":960,"height":600,"fullscreen":false,"volume":0.35}"#;
        std::fs::write(&legacy, old).unwrap();
        let (settings, safe) = Settings::load(&path);
        assert!(safe);
        assert_eq!((settings.width, settings.height), (960, 600));
        assert!(!settings.fullscreen && !settings.auto_resolution);
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path).0, settings);
        assert_eq!(std::fs::read_to_string(&legacy).unwrap(), old);
        for corrupt in ["not toml", "version = 99", "volume = nan", "width = 0"] {
            std::fs::write(&path, corrupt).unwrap();
            assert!(!Settings::load(&path).1);
            assert_eq!(std::fs::read_to_string(&path).unwrap(), corrupt);
        }
    }

    #[test]
    fn quasar_merges_nested_toml_then_cli_and_honors_windowed() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("run.toml");
        std::fs::write(
            &path,
            "[window]\nwidth = 1280\nheight = 720\n[audio]\nvolume = 0.4\n",
        )
        .unwrap();
        let mut common = CommonArgs {
            config: Some(path),
            ..Default::default()
        };
        common
            .parse_into(["--windowed", "--set", "audio.volume=0.5"])
            .unwrap();
        let mut settings = Settings::default();
        settings.apply(&common).unwrap();
        assert_eq!((settings.width, settings.height), (1280, 720));
        assert_eq!(settings.volume, 0.5);
        assert!(!settings.fullscreen && !settings.auto_resolution);
        common.overrides.push(("typo".into(), "true".into()));
        assert!(settings.apply(&common).is_err());
    }
}
