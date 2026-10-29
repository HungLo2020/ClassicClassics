//! Exercise the actual game loaders through injected Quasar sources.
use classicclassics_assets::{AssetSource, FileSource, MemorySource};
use classicclassics_collection::{
    GameKind, Input, Records, Screen, Session, rendering::SceneAssets, settings::Settings,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

struct TrackedSource {
    files: FileSource,
    reads: Arc<AtomicUsize>,
    allowed: Arc<AtomicBool>,
}
impl AssetSource for TrackedSource {
    fn name(&self) -> &str {
        "tracked installed assets"
    }
    fn read(&self, path: &quasar_assets::AssetPath) -> Result<Vec<u8>, quasar_assets::AssetError> {
        assert!(
            self.allowed.load(Ordering::SeqCst),
            "gameplay attempted to load {path}"
        );
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.files.read(path)
    }
}

#[test]
fn gameplay_rendering_and_restart_reuse_prepared_assets_without_reads() {
    for kind in [GameKind::MicroMages, GameKind::Jackal] {
        let path = match kind {
            GameKind::MicroMages => classicclassics_micromages::assets::default_root(),
            GameKind::Jackal => classicclassics_jackal::assets::Assets::default_directory(),
            _ => unreachable!(),
        };
        let allowed = Arc::new(AtomicBool::new(true));
        let reads = Arc::new(AtomicUsize::new(0));
        let source = TrackedSource {
            files: FileSource::new(path).unwrap(),
            reads: reads.clone(),
            allowed: allowed.clone(),
        };
        let mut session = Session::new(Records::default(), Settings::default());
        session.assets.mount_game(kind, Arc::new(source)).unwrap();
        session.launch(kind);
        assert!(session.asset_error.is_none(), "{:?}", session.asset_error);
        let prepared_reads = reads.load(Ordering::SeqCst);
        assert!(prepared_reads > 0);
        allowed.store(false, Ordering::SeqCst);
        session.assets.store.clear_cache().unwrap();
        let mut target = SceneAssets::with_assets(&mut session.assets).unwrap();
        for _ in 0..720 {
            session.tick(session.autoplay());
            session.render(&mut target).unwrap();
        }
        session.tick(Input {
            restart: true,
            ..Default::default()
        });
        session.render(&mut target).unwrap();
        assert_eq!(reads.load(Ordering::SeqCst), prepared_reads);
        session.tick(Input {
            back: true,
            ..Default::default()
        });
        assert!(matches!(session.screen, Screen::Menu));
        assert_eq!(session.assets.store.stats().unwrap().entries, 0);
    }
}

#[test]
fn missing_and_corrupt_game_assets_return_to_catalog_and_allow_another_game() {
    for (name, bytes) in [("missing", None), ("corrupt", Some(b"{broken".to_vec()))] {
        let mut source = MemorySource::new(name);
        if let Some(bytes) = bytes {
            source.insert("levels/levels.json", bytes).unwrap();
        }
        let mut session = Session::new(Records::default(), Settings::default());
        session
            .assets
            .mount_game(GameKind::MicroMages, Arc::new(source))
            .unwrap();
        session.launch(GameKind::MicroMages);
        assert!(matches!(session.screen, Screen::Menu));
        assert!(
            session
                .asset_error
                .as_deref()
                .unwrap()
                .contains("micromages/levels/levels.json")
        );
        let mut target = SceneAssets::with_assets(&mut session.assets).unwrap();
        let error_canvas = session.render(&mut target).unwrap();
        error_canvas.validate(&target.textures).unwrap();
        session.tick(Input {
            confirm: true,
            ..Default::default()
        });
        assert!(session.asset_error.is_some());
        session.tick(Input {
            back: true,
            ..Default::default()
        });
        assert!(session.asset_error.is_none());
        session.launch(GameKind::Pong);
        assert_eq!(session.game_kind(), Some(GameKind::Pong));
    }
}

#[test]
fn procedural_games_and_catalog_do_not_require_external_game_directories() {
    let mut session = Session::new(Records::default(), Settings::default());
    let mut target = SceneAssets::with_assets(&mut session.assets).unwrap();
    session.render(&mut target).unwrap();
    for kind in [
        GameKind::Pong,
        GameKind::LunarLander,
        GameKind::Asteroids,
        GameKind::MissileCommand,
        GameKind::RiverRaid,
    ] {
        assert!(session.assets.game(kind).unwrap().is_none());
        session.launch(kind);
        assert!(session.asset_error.is_none());
        session.render(&mut target).unwrap();
    }
}
