//! Shared, read-only asset access. Quasar owns mounts, typed loading and the cache;
//! games own the interpretation and validation of their recovered content.
use quasar_assets::{
    Asset, AssetError, AssetErrorKind, AssetLoader, AssetServer, CacheBudget, LoadContext,
};
use quasar_render_2d::Image;
use serde::de::DeserializeOwned;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub use quasar_assets::{AssetSource, CacheStats, FileSource, Handle, MemorySource};
pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone)]
pub struct Assets(Arc<Mutex<AssetServer>>);
impl Default for Assets {
    fn default() -> Self {
        let mut server = AssetServer::with_budget(CacheBudget::new(64 * 1024 * 1024, 256));
        server
            .register(PngLoader)
            .register(ClipLoader)
            .register(StreamLoader);
        Self(Arc::new(Mutex::new(server)))
    }
}
impl Assets {
    pub fn mount_file(&self, name: &str, root: &Path) -> Result<Directory> {
        self.0
            .lock()
            .map_err(|e| e.to_string())?
            .mount_file(name, root)
            .map_err(|e| e.to_string())?;
        Ok(self.directory(name))
    }
    pub fn mount(&self, name: &str, source: Arc<dyn AssetSource>) -> Result<Directory> {
        self.0
            .lock()
            .map_err(|e| e.to_string())?
            .mount(name, source)
            .map_err(|e| e.to_string())?;
        Ok(self.directory(name))
    }
    pub fn directory(&self, name: &str) -> Directory {
        Directory {
            assets: self.clone(),
            path: name.into(),
        }
    }
    pub fn stats(&self) -> Result<CacheStats> {
        Ok(self.0.lock().map_err(|e| e.to_string())?.cache_stats())
    }
    pub fn clear_cache(&self) -> Result<()> {
        self.0.lock().map_err(|e| e.to_string())?.clear_cache();
        Ok(())
    }
}

/// A virtual directory (or file) within a mounted namespace. Joining never
/// touches the filesystem; Quasar validates the full path before every access.
#[derive(Clone)]
pub struct Directory {
    assets: Assets,
    path: String,
}
impl Directory {
    pub fn join(&self, relative: impl AsRef<str>) -> Self {
        let relative = relative.as_ref();
        Self {
            assets: self.assets.clone(),
            path: if self.path.is_empty() {
                relative.into()
            } else {
                format!("{}/{relative}", self.path)
            },
        }
    }
    pub fn display(&self) -> &str {
        &self.path
    }
    pub fn is_file(&self) -> bool {
        self.assets
            .0
            .lock()
            .is_ok_and(|server| server.exists(&self.path))
    }
    pub fn read(&self) -> Result<Vec<u8>> {
        self.assets
            .0
            .lock()
            .map_err(|e| e.to_string())?
            .read(&self.path)
            .map_err(|e| format!("{}: {e}", self.path))
    }
    pub fn json<T: DeserializeOwned>(&self) -> Result<T> {
        serde_json::from_slice(&self.read()?).map_err(|e| format!("{}: {e}", self.path))
    }
    pub fn load<L: AssetLoader>(&self) -> Result<Handle<L::Asset>> {
        self.assets
            .0
            .lock()
            .map_err(|e| e.to_string())?
            .load::<L>(&self.path)
            .map_err(|e| format!("{}: {e}", self.path))
    }
    pub fn image(&self) -> Result<Image> {
        Ok(self.load::<PngLoader>()?.0.clone())
    }
}

/// Allows the original public loaders/tests to accept filesystem fixture roots.
/// Nested production loaders share the collection's existing mount and cache.
pub trait AssetLocation {
    fn open_assets(&self) -> Result<Directory>;
}
impl AssetLocation for Directory {
    fn open_assets(&self) -> Result<Directory> {
        Ok(self.clone())
    }
}
impl AssetLocation for Path {
    fn open_assets(&self) -> Result<Directory> {
        Assets::default().mount_file("", self)
    }
}
impl AssetLocation for PathBuf {
    fn open_assets(&self) -> Result<Directory> {
        self.as_path().open_assets()
    }
}

pub struct Png(pub Image);
impl Asset for Png {
    fn memory_size(&self) -> usize {
        self.0.rgba().len()
    }
}
pub struct PngLoader;
impl AssetLoader for PngLoader {
    type Asset = Png;
    fn load(&self, ctx: &LoadContext<'_>) -> std::result::Result<Png, AssetError> {
        Image::decode_png(ctx.bytes())
            .map(Png)
            .map_err(|e| AssetError::at(AssetErrorKind::Loader, ctx.path(), e.to_string()))
    }
}
pub struct AudioClip(pub quasar_audio::Clip);
impl Asset for AudioClip {
    fn memory_size(&self) -> usize {
        std::mem::size_of_val(self.0.samples())
    }
}
pub struct ClipLoader;
impl AssetLoader for ClipLoader {
    type Asset = AudioClip;
    fn load(&self, ctx: &LoadContext<'_>) -> std::result::Result<AudioClip, AssetError> {
        quasar_audio::Clip::from_bytes(ctx.bytes(), None, ctx.path().as_str())
            .map(AudioClip)
            .map_err(|e| AssetError::at(AssetErrorKind::Loader, ctx.path(), e.to_string()))
    }
}
pub struct AudioStream(pub quasar_audio::StreamSource, usize);
impl Asset for AudioStream {
    fn memory_size(&self) -> usize {
        self.1
    }
}
pub struct StreamLoader;
impl AssetLoader for StreamLoader {
    type Asset = AudioStream;
    fn load(&self, ctx: &LoadContext<'_>) -> std::result::Result<AudioStream, AssetError> {
        quasar_audio::StreamSource::from_bytes(Arc::<[u8]>::from(ctx.bytes()), ctx.path().as_str())
            .map(|stream| AudioStream(stream, ctx.bytes().len()))
            .map_err(|e| AssetError::at(AssetErrorKind::Loader, ctx.path(), e.to_string()))
    }
}

/// All games use the same Basalt/development layout policy. Explicit installed
/// paths never fall back to the checkout when content is missing.
pub fn game_root(game: &str, manifest: &str) -> PathBuf {
    resolve_game_root(
        game,
        Path::new(manifest),
        std::env::var_os("BASALT_APP_DIR").as_deref().map(Path::new),
        std::env::var_os("BASALT_GAME_DIR")
            .as_deref()
            .map(Path::new),
        std::env::current_exe().ok().as_deref(),
    )
}
pub fn resolve_game_root(
    game: &str,
    manifest: &Path,
    app: Option<&Path>,
    install: Option<&Path>,
    exe: Option<&Path>,
) -> PathBuf {
    if let Some(app) = app.filter(|p| !p.as_os_str().is_empty()) {
        return app.join("games").join(game);
    }
    if let Some(root) = install.filter(|p| !p.as_os_str().is_empty()) {
        return root.join("app/games").join(game);
    }
    if let Some(bin) = exe.and_then(Path::parent)
        && bin.file_name().is_some_and(|name| name == "bin")
        && let Some(app) = bin.parent()
    {
        return app.join("games").join(game);
    }
    manifest.join("assets")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingSource {
        inner: MemorySource,
        reads: Arc<AtomicUsize>,
    }
    impl AssetSource for CountingSource {
        fn name(&self) -> &str {
            "counted fixture"
        }
        fn read(
            &self,
            path: &quasar_assets::AssetPath,
        ) -> std::result::Result<Vec<u8>, AssetError> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            self.inner.read(path)
        }
    }
    fn png() -> Vec<u8> {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("pixel.png");
        quasar_render_2d::write_png(&path, 1, 1, &[12, 34, 56, 255]).unwrap();
        std::fs::read(path).unwrap()
    }
    #[test]
    fn typed_images_share_cache_and_live_handles_survive_eviction() {
        let assets = Assets::default();
        let mut source = MemorySource::new("fixture");
        source.insert("tile.png", png()).unwrap();
        let reads = Arc::new(AtomicUsize::new(0));
        let root = assets
            .mount(
                "game",
                Arc::new(CountingSource {
                    inner: source,
                    reads: reads.clone(),
                }),
            )
            .unwrap();
        let first = root.join("tile.png").load::<PngLoader>().unwrap();
        let second = root.join("tile.png").load::<PngLoader>().unwrap();
        assert_eq!(first.id(), second.id());
        assert_eq!(reads.load(Ordering::SeqCst), 1);
        assets.clear_cache().unwrap();
        assert_eq!(assets.stats().unwrap().entries, 0);
        assert_eq!(first.0.rgba(), &[12, 34, 56, 255]);
        root.join("tile.png").load::<PngLoader>().unwrap();
        assert_eq!(reads.load(Ordering::SeqCst), 2);
    }
    #[test]
    fn namespaces_and_invalid_paths_cannot_access_other_games() {
        let assets = Assets::default();
        for (name, value) in [("one", "1"), ("two", "2")] {
            let mut source = MemorySource::new(name);
            source
                .insert("level.json", value.as_bytes().to_vec())
                .unwrap();
            let root = assets.mount(name, Arc::new(source)).unwrap();
            assert_eq!(
                root.join("level.json").json::<u32>().unwrap().to_string(),
                value
            );
        }
        for invalid in [
            "../two/level.json",
            "/two/level.json",
            "a/../../two/level.json",
        ] {
            assert!(assets.directory("one").join(invalid).read().is_err());
        }
        let error = assets
            .directory("one")
            .join("missing.png")
            .image()
            .unwrap_err();
        assert!(error.contains("one/missing.png"), "{error}");
    }
    #[test]
    fn invalid_content_names_the_virtual_file() {
        let assets = Assets::default();
        let mut source = MemorySource::new("broken");
        source.insert("level.json", b"{broken".to_vec()).unwrap();
        source.insert("image.png", b"not PNG".to_vec()).unwrap();
        let root = assets.mount("broken", Arc::new(source)).unwrap();
        assert!(
            root.join("level.json")
                .json::<serde_json::Value>()
                .unwrap_err()
                .contains("broken/level.json")
        );
        assert!(
            root.join("image.png")
                .image()
                .unwrap_err()
                .contains("broken/image.png")
        );
    }
    #[test]
    fn relocated_install_and_environment_precedence_do_not_fall_back_to_checkout() {
        let manifest = Path::new("/build/crate");
        let exe = Path::new("/a moved library/ClassicClassics/app/bin/classicclassics");
        for game in ["micromages", "jackal"] {
            assert_eq!(
                resolve_game_root(game, manifest, None, None, Some(exe)),
                Path::new("/a moved library/ClassicClassics/app/games").join(game)
            );
            assert_eq!(
                resolve_game_root(game, manifest, None, Some(Path::new("/install")), Some(exe)),
                Path::new("/install/app/games").join(game)
            );
            assert_eq!(
                resolve_game_root(
                    game,
                    manifest,
                    Some(Path::new("/app")),
                    Some(Path::new("/install")),
                    Some(exe)
                ),
                Path::new("/app/games").join(game)
            );
            assert_eq!(
                resolve_game_root(
                    game,
                    manifest,
                    Some(Path::new("")),
                    None,
                    Some(Path::new("/build/target/debug/classicclassics"))
                ),
                manifest.join("assets")
            );
        }
    }
    #[test]
    fn file_mounts_reject_symlink_escape() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("assets");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(directory.path().join("outside.json"), "1").unwrap();
        symlink(
            directory.path().join("outside.json"),
            root.join("escape.json"),
        )
        .unwrap();
        let mounted = Assets::default().mount_file("game", &root).unwrap();
        assert!(mounted.join("escape.json").json::<u32>().is_err());
    }
}
