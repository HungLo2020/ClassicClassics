//! Collection record persistence; individual schemas live in game crates.

use crate::{Result, games::Records};
use std::path::Path;

impl Records {
    /// Returns the records plus whether the file is safe to replace on exit.
    /// Missing files are safe (first run); corrupt or newer files are preserved.
    pub fn load(path: &Path) -> (Self, bool) {
        match quasar_storage::read_json::<Self>(path) {
            Ok(Some(records)) if records.version == 1 => (records, true),
            Ok(Some(_)) => {
                eprintln!("Unsupported records version; preserving file and starting fresh");
                (Self::default(), false)
            }
            Ok(None) => (Self::default(), true),
            Err(error) => {
                eprintln!("Cannot load records ({error}); preserving file and starting fresh");
                (Self::default(), false)
            }
        }
    }

    pub fn save(&self, path: &Path) -> Result {
        quasar_storage::write_json(path, self)?;
        Ok(())
    }
}
