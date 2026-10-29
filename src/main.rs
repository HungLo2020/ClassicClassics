//! Classic Classics entry point. Shared simulation lives in the library; this
//! binary only wires Quasar configuration, storage, audio and the desktop host.

#[cfg(feature = "desktop")]
mod desktop;

use classicclassics_collection::cli::{self, Options};
use classicclassics_collection::controls::Controls;
use classicclassics_collection::settings::Settings;
use classicclassics_collection::{Input, Records, Session, VERSION};
use quasar_config::CommonArgs;
use quasar_storage::GamePaths;
use std::path::Path;

fn main() {
    if let Err(error) = run() {
        eprintln!("classicclassics: {error}");
        std::process::exit(1);
    }
}

fn run() -> classicclassics_collection::Result {
    let mut common = CommonArgs::from_env("CLASSICCLASSICS").map_err(|error| error.to_string())?;
    common.parse_into(std::env::args().skip(1))?;
    if common.help {
        print!("{}", common.help("classicclassics", &cli::help()));
        return Ok(());
    }
    if common.version {
        println!("classicclassics {VERSION}");
        return Ok(());
    }
    let options = Options::from_common(&common)?;

    if let Some(path) = &options.write_default_bindings {
        Controls::default().save(path)?;
        println!(
            "Wrote default Classic Classics controls to {}",
            path.display()
        );
        return Ok(());
    }
    if let Some(path) = &options.check_bindings {
        Controls::load(path)?;
        println!("Classic Classics controls are valid");
        return Ok(());
    }

    let paths = GamePaths::discover(Some(Path::new(env!("CARGO_MANIFEST_DIR"))))?;
    let controls_path = paths.shared_config.join("controls.toml");
    let (controls, save_controls) = if let Some(path) = &options.bindings {
        (Controls::load(path)?, false)
    } else if options.no_save {
        (Controls::default(), false)
    } else {
        Controls::load_saved(&controls_path)
    };

    let (mut settings, mut save_settings) = if options.no_save {
        (Settings::default(), false)
    } else {
        Settings::load(&paths.config.join("display.toml"))
    };
    if common.config.is_some() || !common.overrides.is_empty() {
        // Explicit run configuration is ephemeral; it never overwrites the
        // machine's saved display settings.
        save_settings = false;
    }
    settings.apply(&common)?;
    settings.linear |= options.linear;
    settings.integer |= options.integer;
    if options.no_audio {
        settings.audio = false;
    }

    let (records, save_records) = if options.no_save {
        (Records::default(), false)
    } else {
        Records::load(&paths.save.join("records.json"))
    };
    let mut session = Session::new(records, settings);
    session.controls = controls;
    if let Some(kind) = options.game {
        session.launch(kind);
    }
    if options.settings {
        session.open_settings();
    }

    if options.controls {
        session.open_controls();
    }

    if options.headless {
        return run_headless(&options, &mut session);
    }

    #[cfg(feature = "desktop")]
    {
        desktop::run(
            options,
            session,
            paths,
            desktop::SavePolicy {
                controls: save_controls,
                settings: save_settings,
                records: save_records,
            },
        )
    }
    #[cfg(not(feature = "desktop"))]
    {
        let _ = (save_controls, save_settings, save_records, paths);
        Err("desktop support was disabled at build time; use --headless or rebuild with default features".into())
    }
}

fn run_headless(options: &Options, session: &mut Session) -> classicclassics_collection::Result {
    if let Some(error) = &session.asset_error {
        return Err(error.clone().into());
    }
    let ticks = options.ticks.unwrap_or(600);
    for _ in 0..ticks {
        let input = if options.autoplay {
            session.autoplay()
        } else {
            Input::default()
        };
        session.tick(input);
    }
    if let Some(path) = &options.screenshot {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        let mut assets =
            classicclassics_collection::rendering::SceneAssets::with_assets(&mut session.assets)?;
        let canvas = session.render(&mut assets)?;
        let pixels = quasar_render_2d::reference::rasterize(&canvas, &assets.textures)?;
        quasar_render_2d::write_png(path, canvas.width(), canvas.height(), &pixels)?;
    }
    if let Some(path) = &options.state {
        quasar_storage::write_json(path, &session.state_json())?;
    }
    println!("{}", session.summary());
    Ok(())
}
