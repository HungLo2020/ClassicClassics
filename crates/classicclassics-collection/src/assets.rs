//! Collection-owned namespaces. File-backed games mount only when selected;
//! embedded catalog covers use the same Quasar loaders and cache.
use crate::GameKind;
use classicclassics_assets::{Assets, Directory, MemorySource, Result};
use std::{collections::BTreeSet, sync::Arc};

#[derive(Default)]
pub struct GameAssets {
    pub store: Assets,
    mounted: BTreeSet<&'static str>,
    catalog: bool,
}
impl GameAssets {
    pub fn game(&mut self, kind: GameKind) -> Result<Option<Directory>> {
        let Some(manifest) = kind.asset_manifest_dir() else {
            return Ok(None);
        };
        let name = kind.info().aliases[0];
        if !self.mounted.contains(name) {
            let root = classicclassics_assets::game_root(name, manifest);
            self.store.mount_file(name, &root)?;
            self.mounted.insert(name);
        }
        Ok(Some(self.store.directory(name)))
    }
    /// Explicit source injection for headless fixtures and future embedded packs.
    pub fn mount_game(
        &mut self,
        kind: GameKind,
        source: Arc<dyn classicclassics_assets::AssetSource>,
    ) -> Result<()> {
        let name = kind.info().aliases[0];
        self.store.mount(name, source)?;
        self.mounted.insert(name);
        Ok(())
    }
    pub fn catalog(&mut self) -> Result<Directory> {
        if !self.catalog {
            let mut source = MemorySource::new("embedded catalog artwork");
            for kind in GameKind::ALL {
                if let Some(bytes) = kind.cover() {
                    source
                        .insert(format!("{}.png", kind.number()), bytes.to_vec())
                        .map_err(|e| e.to_string())?;
                }
            }
            self.store.mount("catalog", Arc::new(source))?;
            self.catalog = true;
        }
        Ok(self.store.directory("catalog"))
    }
}
