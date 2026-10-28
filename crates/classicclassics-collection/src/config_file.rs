//! Persistence adapter for Quasar's typed TOML/JSON configuration readers.
use crate::Result;
use quasar_config::FileFormat;
use serde::Serialize;
use std::{
    io::Write,
    path::{Path, PathBuf},
};

/// Existing TOML wins, even when invalid. A legacy JSON file is left intact.
pub fn source(path: &Path) -> Result<Option<PathBuf>> {
    if path.try_exists()? {
        return Ok(Some(path.to_path_buf()));
    }
    let legacy = path.with_extension("json");
    Ok(legacy.try_exists()?.then_some(legacy))
}

pub fn write<T: Serialize>(path: &Path, value: &T) -> Result {
    match FileFormat::from_path(path) {
        Some(FileFormat::Json) => quasar_storage::write_json(path, value)?,
        Some(FileFormat::Toml) => {
            // Quasar 0.2.16 exposes atomic JSON writes only. Match its
            // same-directory temporary file, sync and rename contract for TOML.
            let text = toml::to_string_pretty(value)?;
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            std::fs::create_dir_all(parent)?;
            let mut file = tempfile::NamedTempFile::new_in(parent)?;
            file.write_all(text.as_bytes())?;
            file.as_file().sync_all()?;
            file.persist(path).map_err(|error| error.error)?;
        }
        None => return Err("configuration path must end in .toml or .json".into()),
    }
    Ok(())
}
