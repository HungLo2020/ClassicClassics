//! Machine-local display and audio settings. These live in `data/config/`, not
//! in synced saves, because resolution and audio devices differ per computer.

use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
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
            width: 960,
            height: 600,
            fullscreen: false,
            linear: false,
            integer: false,
            audio: true,
            volume: 0.8,
            muted: false,
        }
    }
}

impl Settings {
    pub fn valid(&self) -> bool {
        self.version == 1
            && (640..=16384).contains(&self.width)
            && (400..=16384).contains(&self.height)
            && self.volume.is_finite()
            && (0.0..=1.0).contains(&self.volume)
    }

    /// Returns the settings plus whether the file is safe to replace on exit.
    /// Missing files are safe (first run); corrupt or newer files are preserved.
    pub fn load(path: &Path) -> (Self, bool) {
        match quasar_storage::read_json::<Self>(path) {
            Ok(Some(settings)) if settings.valid() => (settings, true),
            Ok(Some(_)) => {
                eprintln!("Unsupported display settings; preserving file and using defaults");
                (Self::default(), false)
            }
            Ok(None) => (Self::default(), true),
            Err(error) => {
                eprintln!(
                    "Cannot load display settings ({error}); preserving file and using defaults"
                );
                (Self::default(), false)
            }
        }
    }

    pub fn save(&self, path: &Path) -> Result {
        quasar_storage::write_json(path, self)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_is_writable_but_corrupt_and_newer_are_preserved() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("display.json");
        let (settings, safe) = Settings::load(&path);
        assert_eq!(settings.width, 960);
        assert!(safe);

        std::fs::write(&path, b"not json").unwrap();
        let (_, safe) = Settings::load(&path);
        assert!(!safe);
        assert_eq!(std::fs::read(&path).unwrap(), b"not json");

        std::fs::write(&path, br#"{"version":99}"#).unwrap();
        let (_, safe) = Settings::load(&path);
        assert!(!safe);
        assert_eq!(std::fs::read(&path).unwrap(), br#"{"version":99}"#);
    }
}
