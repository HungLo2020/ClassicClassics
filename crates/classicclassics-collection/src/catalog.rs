//! One card per native game or cartridge.

use crate::games::{GameKind, Records};
use crate::paint::{Color, Paint};
use classicclassics_core::RomEntry;

#[derive(Clone, Debug)]
pub enum CatalogEntry {
    Game(GameKind),
    Rom { rom: RomEntry, number: String },
}

impl CatalogEntry {
    pub fn kind(&self) -> GameKind {
        match self {
            Self::Game(kind) => *kind,
            Self::Rom { rom, .. } => {
                GameKind::from_emulator_system(&rom.system).unwrap_or(GameKind::Nes)
            }
        }
    }

    pub fn rom(&self) -> Option<&RomEntry> {
        match self {
            Self::Game(_) => None,
            Self::Rom { rom, .. } => Some(rom),
        }
    }

    pub fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Game(a), Self::Game(b)) => a == b,
            (Self::Rom { rom: a, .. }, Self::Rom { rom: b, .. }) => a.id == b.id,
            _ => false,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Game(kind) => kind.name(),
            Self::Rom { rom, .. } => &rom.name,
        }
    }

    pub fn number(&self) -> &str {
        match self {
            Self::Game(kind) => kind.number(),
            Self::Rom { number, .. } => number,
        }
    }

    pub fn system(&self) -> &'static str {
        self.kind().system_short().unwrap_or("NATIVE")
    }

    pub fn tagline(&self) -> String {
        match self {
            Self::Game(kind) => kind.tagline().to_owned(),
            Self::Rom { rom, .. } => {
                let library = if rom.builtin { "BUILT-IN" } else { "USER" };
                format!("{library} {} CARTRIDGE", self.system())
            }
        }
    }

    pub fn accent(&self) -> Color {
        self.kind().accent()
    }

    pub fn motif(&self, paint: &mut Paint, cx: f32, cy: f32) {
        self.kind().motif(paint, cx, cy);
    }

    pub fn artwork_kind(&self) -> Option<GameKind> {
        match self {
            Self::Game(kind) => Some(*kind),
            Self::Rom { .. } => None,
        }
    }

    pub fn catalog_record(&self, records: &Records) -> (&'static str, String) {
        match self {
            Self::Game(kind) => kind.catalog_record(records),
            Self::Rom { rom, .. } => (
                "LIBRARY",
                if rom.builtin { "BUILT-IN" } else { "USER" }.to_owned(),
            ),
        }
    }
}

pub fn build(groups: Vec<(GameKind, Vec<RomEntry>)>) -> Vec<CatalogEntry> {
    let mut entries: Vec<_> = GameKind::ALL
        .into_iter()
        .filter(|kind| !kind.is_emulator())
        .map(CatalogEntry::Game)
        .collect();
    for (kind, roms) in groups {
        let prefix = match kind {
            GameKind::Nes => "N",
            GameKind::Snes => "S",
            GameKind::Gba => "G",
            GameKind::Atari2600 => "A",
            _ => continue,
        };
        for (index, rom) in roms.into_iter().enumerate() {
            entries.push(CatalogEntry::Rom {
                rom,
                number: format!("{prefix}{:02}", index + 1),
            });
        }
    }
    entries
}
