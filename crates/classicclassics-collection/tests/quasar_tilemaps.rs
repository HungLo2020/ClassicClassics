//! Pixel fidelity of the new Quasar paths against the retained software renderers.
use std::sync::Arc;
fn compare(label: &str, expected: &[u32], actual: &[u32]) {
    assert_eq!(expected.len(), actual.len(), "{label}: frame size");
    let mismatch = expected
        .iter()
        .zip(actual)
        .enumerate()
        .find(|(_, (a, b))| (*a & 0xffffff) != (*b & 0xffffff));
    if let Some((i, (a, b))) = mismatch {
        panic!(
            "{label}: pixel {},{} expected {a:06x}, got {b:06x}; {} mismatches",
            i % 256,
            i / 256,
            expected
                .iter()
                .zip(actual)
                .filter(|(a, b)| (**a & 0xffffff) != (**b & 0xffffff))
                .count()
        );
    }
}
#[test]
fn micro_mages_quasar_tiles_match_software_across_campaign_and_live_edits() {
    use classicclassics_micromages::{
        assets::{Assets, default_root},
        game::Game,
        video::Video,
    };
    let assets = Arc::new(Assets::load(&default_root()).unwrap());
    let mut original = Video::new();
    let mut quasar = Video::new();
    for hard in [false, true] {
        for stage in 0..if hard { 14 } else { 13 } {
            let mut game = Game::new(assets.clone());
            game.start(stage, 4, hard).unwrap();
            for tick in 0..48 {
                game.update([if tick < 24 { 147 } else { 66 }, 64, 128, 16]);
                if tick % 12 == 0 {
                    let expected = original.render(&game, &assets);
                    let frame = quasar.render_draw(&game, &assets);
                    compare(
                        &format!("Micro Mages hard={hard} stage={stage} tick={tick}"),
                        expected,
                        &frame.reference_pixels(),
                    );
                    assert!(frame.canvas.uses_pixel_masks());
                }
            }
            let world = game.world.as_mut().unwrap();
            let y = (world.camera.position.y as usize / 8 + 16).min(world.level.height - 1);
            assert!(world.level.set_metatile(5, y / 2, 0));
            compare(
                "Micro Mages live terrain edit",
                original.render(&game, &assets),
                &quasar.render_draw(&game, &assets).reference_pixels(),
            );
        }
    }
}
#[test]
fn jackal_quasar_tiles_match_software_across_stages_cameras_and_cinematics() {
    use classicclassics_jackal::{
        assets::Assets,
        game::{Game, GameState},
        renderer::Renderer,
    };
    let assets = Assets::load(&Assets::default_directory()).unwrap();
    let mut original = Renderer::new();
    let mut quasar = Renderer::new();
    let mut game = Game::default();
    compare(
        "Jackal title",
        original.render(&game, &assets),
        &quasar.render_draw(&game, &assets).reference_pixels(),
    );
    game.start_new_game(2, &assets);
    for stage in 0..6 {
        game.start_stage(stage, &assets);
        game.state = GameState::Playing;
        game.camera.y = assets.levels[stage].start_camera_y();
        game.camera.x = assets.levels[stage].start_camera_x;
        for offset in [0., 0.25, 0.5, 0.75, 1., 17.5, 64.] {
            game.camera.y = assets.levels[stage].start_camera_y() - offset;
            compare(
                &format!("Jackal stage={stage} camera={offset}"),
                original.render(&game, &assets),
                &quasar.render_draw(&game, &assets).reference_pixels(),
            );
        }
        for tick in 0..160 {
            game.update(classicclassics_jackal::input::Input::default(), &assets);
            if tick % 16 == 0 {
                compare(
                    &format!("Jackal stage={stage} animation tick={tick}"),
                    original.render(&game, &assets),
                    &quasar.render_draw(&game, &assets).reference_pixels(),
                );
            }
        }
        let i = (game.camera.y as usize / 8 + 12) * assets.levels[stage].width_tiles + 12;
        game.terrain.patches.insert(
            i,
            classicclassics_jackal::terrain::TilePatch {
                tile: 17,
                palette: 2,
                collision: 0,
            },
        );
        compare(
            "Jackal live terrain destruction",
            original.render(&game, &assets),
            &quasar.render_draw(&game, &assets).reference_pixels(),
        );
    }
}

#[test]
fn jackal_quasar_cinematic_timelines_match_software() {
    use classicclassics_jackal::{
        assets::Assets,
        game::{Game, GameState},
        renderer::Renderer,
    };
    let assets = Assets::load(&Assets::default_directory()).unwrap();
    let mut game = Game::default();
    game.start_new_game(2, &assets);
    let mut original = Renderer::new();
    let mut quasar = Renderer::new();
    for state in [
        GameState::Briefing { ticks: 82 },
        GameState::Briefing { ticks: 546 },
        GameState::Ending { ticks: 3000 },
        GameState::StageClear { ticks: 200 },
    ] {
        game.state = state;
        compare(
            &format!("Jackal cinematic {:?}", game.state),
            original.render(&game, &assets),
            &quasar.render_draw(&game, &assets).reference_pixels(),
        );
    }
}

#[test]
fn production_session_draws_tiled_scenes_and_releases_resources_on_catalog_return() {
    use classicclassics_collection::{
        Game, GameKind, Input, Records, Screen, Session, rendering::SceneAssets, settings::Settings,
    };
    for kind in [GameKind::MicroMages, GameKind::Jackal] {
        let mut session = Session::new(Records::default(), Settings::default());
        session.launch(kind);
        if let Screen::Game(Game::MicroMages(game)) = &mut session.screen {
            game.engine.start(0, 1, false).unwrap();
        }
        let mut assets = SceneAssets::with_assets(&mut session.assets).unwrap();
        let first = session.render(&mut assets).unwrap();
        assert!(
            first.quads().len() > 100,
            "production must draw tile/atlas quads instead of a framebuffer"
        );
        let ids: Vec<_> = first
            .quads()
            .iter()
            .filter_map(|q| {
                (q.texture != quasar_render_2d::TextureId::WHITE
                    && q.texture != quasar_render_2d::TextureId::FONT)
                    .then_some(q.texture)
            })
            .collect();
        let repeated = session.render(&mut assets).unwrap();
        assert_eq!(first.quads().len(), repeated.quads().len());
        assert!(
            first
                .quads()
                .iter()
                .zip(repeated.quads())
                .all(|(a, b)| a.texture == b.texture && a.palette == b.palette)
        );
        session.tick(Input {
            back: true,
            ..Default::default()
        });
        session.render(&mut assets).unwrap();
        assert!(ids.iter().all(|id| assets.textures.get(*id).is_none()));
    }
}
