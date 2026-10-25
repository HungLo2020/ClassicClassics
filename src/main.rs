//! Classic Classics entry point. Shared simulation lives in the library; this
//! binary only wires Quasar configuration, storage, audio and the desktop host.

#[cfg(feature = "desktop")]
mod desktop;

use classicclassics::cli::{self, Options};
use classicclassics::controls::Controls;
use classicclassics::settings::Settings;
use classicclassics::{Input, Records, Session, VERSION};
use quasar_config::CommonArgs;
use quasar_storage::GamePaths;
use std::path::Path;

fn main() {
    if let Err(error) = run() {
        eprintln!("classicclassics: {error}");
        std::process::exit(1);
    }
}

fn run() -> classicclassics::Result {
    let mut common = CommonArgs::from_env("CLASSICCLASSICS").map_err(|error| error.to_string())?;
    common.parse_into(std::env::args().skip(1))?;
    if common.help {
        print!("{}", common.help("classicclassics", cli::HELP));
        return Ok(());
    }
    if common.version {
        println!("classicclassics {VERSION}");
        return Ok(());
    }
    let options = Options::from_common(&common)?;

    if let Some(path) = &options.write_default_bindings {
        quasar_storage::write_json(path, &Controls::default())?;
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
    let controls_path = paths.shared_config.join("controls.json");
    let (controls, save_controls) = if let Some(path) = &options.bindings {
        (Controls::load(path)?, false)
    } else if options.no_save {
        (Controls::default(), false)
    } else {
        match quasar_storage::read_json::<Controls>(&controls_path) {
            Ok(Some(controls)) => match controls.validate() {
                Ok(()) => (controls, false),
                Err(error) => {
                    eprintln!("Invalid controls ({error}); preserving file and using defaults");
                    (Controls::default(), false)
                }
            },
            Ok(None) => (Controls::default(), true),
            Err(error) => {
                eprintln!("Cannot load controls ({error}); preserving file and using defaults");
                (Controls::default(), false)
            }
        }
    };

    let (mut settings, mut save_settings) = if options.no_save {
        (Settings::default(), false)
    } else {
        Settings::load(&paths.config.join("display.json"))
    };
    if common.config.is_some() || !common.overrides.is_empty() {
        // Explicit run configuration is ephemeral; it never overwrites the
        // machine's saved display settings.
        save_settings = false;
        apply_settings_overrides(&mut settings, &common)?;
    }
    settings.width = options.width.unwrap_or(settings.width);
    settings.height = options.height.unwrap_or(settings.height);
    settings.fullscreen |= options.fullscreen;
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
    if let Some(kind) = options.game {
        session.launch(kind);
    }
    if options.settings {
        session.open_settings();
    }

    if options.headless {
        return run_headless(&options, &mut session, &controls);
    }

    #[cfg(feature = "desktop")]
    {
        desktop::run(
            options,
            session,
            controls,
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

fn apply_settings_overrides(
    settings: &mut Settings,
    common: &CommonArgs,
) -> classicclassics::Result {
    if let Some(path) = &common.config {
        *settings = quasar_config::load_file_typed::<Settings>(path)?;
    }
    for (key, value) in &common.overrides {
        let boolean = || -> classicclassics::Result<bool> {
            Ok(match value.as_str() {
                "true" | "1" | "yes" | "on" => true,
                "false" | "0" | "no" | "off" => false,
                other => return Err(format!("{key} expects a boolean, got '{other}'").into()),
            })
        };
        match key.as_str() {
            "width" | "window.width" => settings.width = value.parse()?,
            "height" | "window.height" => settings.height = value.parse()?,
            "fullscreen" | "window.fullscreen" => settings.fullscreen = boolean()?,
            "linear" | "display.linear" => settings.linear = boolean()?,
            "integer" | "display.integer" => settings.integer = boolean()?,
            "audio" | "audio.enabled" => settings.audio = boolean()?,
            "volume" | "audio.volume" => settings.volume = value.parse()?,
            "muted" | "audio.muted" => settings.muted = boolean()?,
            other => {
                return Err(format!(
                    "unknown setting '{other}'; use width, height, fullscreen, linear, integer, audio, volume or muted"
                )
                .into());
            }
        }
    }
    if !settings.valid() {
        return Err("configuration settings are outside the supported range".into());
    }
    Ok(())
}

fn run_headless(
    options: &Options,
    session: &mut Session,
    controls: &Controls,
) -> classicclassics::Result {
    let ticks = options.ticks.unwrap_or(600);
    for _ in 0..ticks {
        let input = if options.autoplay {
            session.autoplay()
        } else {
            Input::default()
        };
        session.tick(input);
    }
    let hints = controls.hints();
    if let Some(path) = &options.screenshot {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        let textures = quasar_render_2d::Textures::default();
        let canvas = session.scene(&hints);
        let pixels = quasar_render_2d::reference::rasterize(&canvas, &textures)?;
        quasar_render_2d::write_png(
            path,
            classicclassics::WIDTH,
            classicclassics::HEIGHT,
            &pixels,
        )?;
    }
    if let Some(path) = &options.state {
        quasar_storage::write_json(path, &session.state_json())?;
    }
    println!("{}", session.summary());
    Ok(())
}
