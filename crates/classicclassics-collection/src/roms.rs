//! Collection-owned cartridge discovery. Games never scan directories.

use crate::GameKind;
use classicclassics_core::RomEntry;
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default)]
pub struct EmulatorPaths {
    pub app: PathBuf,
    pub save: PathBuf,
    pub extra: Vec<PathBuf>,
    pub initial: Option<PathBuf>,
    pub cores: Option<PathBuf>,
}

#[derive(Default, Deserialize)]
struct Config {
    #[serde(default)]
    rom_dirs: Vec<PathBuf>,
}

pub fn discover(kind: GameKind, paths: &EmulatorPaths) -> Vec<RomEntry> {
    let Some(system) = kind.emulator_system() else {
        return Vec::new();
    };
    let mut roots: Vec<(PathBuf, bool)> = Vec::new();
    if let Some(manifest) = kind.asset_manifest_dir() {
        let base = classicclassics_assets::game_root(system, manifest);
        for candidate in [base.join("roms"), base.join(system).join("roms")] {
            roots.push((candidate, true));
        }
    }
    let data = paths.save.parent().map(Path::to_path_buf);
    if let Some(data) = &data {
        for candidate in [data.join("roms").join(system), data.join("roms")] {
            roots.push((candidate, false));
        }
        if let Ok(text) = std::fs::read_to_string(data.join("config/emulators.toml"))
            && let Ok(config) = toml::from_str::<Config>(&text)
        {
            roots.extend(config.rom_dirs.into_iter().map(|dir| {
                let dir = if dir.is_relative() {
                    data.join(dir)
                } else {
                    dir
                };
                (dir, false)
            }));
        }
    }
    roots.extend(paths.extra.iter().cloned().map(|dir| (dir, false)));

    let extensions = kind.rom_extensions();
    let mut seen = BTreeSet::new();
    let mut roms = Vec::new();
    for (root, builtin) in roots {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(extension) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            if !extensions
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate))
            {
                continue;
            }
            let Ok(id) = id_for(&path) else {
                continue;
            };
            if !seen.insert(id) {
                continue;
            }
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            roms.push(RomEntry {
                system: system.to_owned(),
                name: display_name(&path),
                path,
                size,
                id,
                builtin,
            });
        }
    }
    roms.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    roms
}

pub fn entry_for(kind: GameKind, path: &Path) -> Option<RomEntry> {
    let system = kind.emulator_system()?;
    let id = id_for(path).ok()?;
    Some(RomEntry {
        system: system.to_owned(),
        name: display_name(path),
        path: path.to_path_buf(),
        size: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        id,
        builtin: false,
    })
}

pub fn id_for(path: &Path) -> std::io::Result<u64> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        for byte in &buffer[..read] {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    Ok(hash)
}

fn display_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("cartridge");
    clean_name(stem)
}

/// Turn a dump filename into a catalog title: drop region/revision/hack tags and
/// punctuation the bitmap font cannot show.
fn clean_name(stem: &str) -> String {
    let mut text = String::new();
    let mut depth = 0i32;
    for character in stem.chars() {
        match character {
            '(' | '[' => depth += 1,
            ')' | ']' if depth > 0 => depth -= 1,
            _ if depth > 0 => {}
            '-' => text.push('-'),
            '&' => text.push_str(" AND "),
            ',' | '\'' | '!' | '?' | '.' | '_' => text.push(' '),
            other => text.push(other),
        }
    }
    let mut name = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase();
    if stem.to_lowercase().ends_with(", the")
        && let Some(base) = name.strip_suffix(" THE")
    {
        name = format!("THE {base}");
    }
    name.replace("LEGEND OF ZELDA THE -", "THE LEGEND OF ZELDA -")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_filters_extensions_deduplicates_and_uppercases_names() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("my_game.nes"), b"one").unwrap();
        std::fs::write(directory.path().join("notes.txt"), b"ignore").unwrap();
        std::fs::write(directory.path().join("copy.NES"), b"two").unwrap();
        let paths = EmulatorPaths {
            extra: vec![directory.path().to_path_buf()],
            ..Default::default()
        };
        let roms = discover(GameKind::Nes, &paths);
        let user: Vec<_> = roms.iter().filter(|rom| !rom.builtin).collect();
        assert_eq!(user.len(), 2);
        assert!(user.iter().any(|rom| rom.name == "MY GAME"));
        assert!(user.iter().any(|rom| rom.name == "COPY"));
        assert!(roms.iter().any(|rom| rom.builtin));
        assert_eq!(id_for(&roms[0].path).unwrap(), roms[0].id);

        let duplicate = tempfile::tempdir().unwrap();
        std::fs::write(duplicate.path().join("a.nes"), b"same").unwrap();
        std::fs::write(duplicate.path().join("b.nes"), b"same").unwrap();
        let paths = EmulatorPaths {
            extra: vec![duplicate.path().to_path_buf()],
            ..Default::default()
        };
        let user = discover(GameKind::Nes, &paths)
            .into_iter()
            .filter(|rom| !rom.builtin)
            .count();
        assert_eq!(user, 1);
    }

    #[test]
    fn every_emulated_kind_has_a_distinct_identity() {
        let mut systems = std::collections::BTreeSet::new();
        for kind in GameKind::ALL {
            if let Some(system) = kind.emulator_system() {
                assert!(systems.insert(system));
                assert!(!kind.rom_extensions().is_empty());
            }
        }
        assert_eq!(systems.len(), 4);
    }
}
