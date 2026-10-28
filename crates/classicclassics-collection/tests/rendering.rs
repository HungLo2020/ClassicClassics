use classicclassics_collection::{
    Game, GameKind, Records, Session,
    controls::ControlHints,
    rendering::{SceneAssets, SceneTarget},
    settings::Settings,
};

const IMPORTS: [GameKind; 4] = [
    GameKind::MicroMages,
    GameKind::Jackal,
    GameKind::MissileCommand,
    GameKind::RiverRaid,
];

#[test]
fn imports_cache_video_frames_and_render_one_textured_quad_without_changing_snapshots() {
    let mut assets = SceneAssets::new().unwrap();
    for kind in IMPORTS {
        let mut game = Game::new(kind);
        let before = serde_json::to_value(&game).unwrap();
        let first = game.pixel_frame(&ControlHints::default()).unwrap();
        let repeated = game.pixel_frame(&ControlHints::default()).unwrap();
        assert!(std::ptr::eq(
            first.image.rgba().as_ptr(),
            repeated.image.rgba().as_ptr()
        ));
        assert_eq!(serde_json::to_value(&game).unwrap(), before);
        let id = assets.upload_frame(&first).unwrap();
        assert_eq!(assets.upload_frame(&repeated).unwrap(), id);
        let canvas = first.canvas(id);
        canvas.validate(&assets.textures).unwrap();
        assert_eq!(
            canvas
                .quads()
                .iter()
                .filter(|quad| quad.texture == id)
                .count(),
            1
        );
        assert!(
            canvas.quads().len() < 512,
            "pixel frames must never expand into per-pixel geometry"
        );
        // The 120 Hz host redraw between video ticks must reuse the image.
        game.tick(Default::default(), &mut Vec::new());
        let half_tick = game.pixel_frame(&ControlHints::default()).unwrap();
        assert!(std::ptr::eq(
            first.image.rgba().as_ptr(),
            half_tick.image.rgba().as_ptr()
        ));
        game.tick(Default::default(), &mut Vec::new());
        let next = game.pixel_frame(&ControlHints::default()).unwrap();
        assert!(!std::ptr::eq(
            first.image.rgba().as_ptr(),
            next.image.rgba().as_ptr()
        ));
    }
}

#[test]
fn textured_frame_preserves_pixel_colors_and_four_by_three_geometry() {
    use classicclassics_collection::rendering::PixelFrame;
    let image = quasar_render_2d::Image::new(
        2,
        2,
        vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ],
    )
    .unwrap();
    let frame = PixelFrame {
        image,
        aspect: 4.0 / 3.0,
        help: String::new(),
    };
    let mut assets = SceneAssets::new().unwrap();
    let id = assets.upload_frame(&frame).unwrap();
    let canvas = frame.canvas(id);
    let rgba = quasar_render_2d::reference::rasterize(&canvas, &assets.textures).unwrap();
    for ((x, y), expected) in [
        ((293, 140), [255, 0, 0, 255]),
        ((667, 140), [0, 255, 0, 255]),
        ((293, 420), [0, 0, 255, 255]),
        ((667, 420), [255, 255, 255, 255]),
    ] {
        assert_eq!(&rgba[(y * 960 + x) * 4..(y * 960 + x) * 4 + 4], &expected);
    }
}

#[test]
fn both_catalog_pages_use_the_complete_basalt_covers_at_portrait_aspect() {
    let mut assets = SceneAssets::new().unwrap();
    let mut session = Session::new(Records::default(), Settings::default());
    for kind in IMPORTS {
        let texture = assets
            .artwork
            .get(kind)
            .expect("Basalt cover must be registered");
        let image = assets.textures.get(texture).unwrap();
        assert_eq!(image.width() * 4, image.height() * 3);
        session.selected = GameKind::ALL
            .iter()
            .position(|candidate| *candidate == kind)
            .unwrap();
        let canvas = session.render(&mut assets).unwrap();
        canvas.validate(&assets.textures).unwrap();
        let cover: Vec<_> = canvas
            .quads()
            .iter()
            .filter(|quad| quad.texture == texture)
            .collect();
        assert_eq!(cover.len(), 1);
        // A complete source image uses all UV corners; a crop would change these.
        assert!(cover[0].source.is_none());
        assert_eq!(cover[0].x[0] * 4.0, cover[0].y[1] * 3.0);
    }
}

#[test]
fn artwork_pixels_match_the_independent_basalt_source_manifest() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../docs/artwork-sources.json")).unwrap();
    let assets = SceneAssets::new().unwrap();
    for kind in IMPORTS {
        let entry = &manifest[kind.info().aliases[0]];
        let image = assets
            .textures
            .get(assets.artwork.get(kind).unwrap())
            .unwrap();
        assert_eq!(u64::from(image.width()), entry["width"].as_u64().unwrap());
        assert_eq!(u64::from(image.height()), entry["height"].as_u64().unwrap());
        let hash = image
            .rgba()
            .iter()
            .fold(0xcbf29ce484222325u64, |hash, byte| {
                (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
            });
        assert_eq!(
            format!("{hash:016x}"),
            entry["rgba_fnv1a64"].as_str().unwrap()
        );
    }
}
