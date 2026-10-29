//! Native steady-state profile: start directly in gameplay, without original
//! setup/cinematics. Uses exactly the application's host and Quasar presenter.
#[path = "../src/desktop.rs"]
mod desktop;
#[path = "../src/native_renderer.rs"]
mod native_renderer;
use classicclassics_collection::{
    Game, GameKind, Records, Screen, Session, cli::Options, settings::Settings,
};
fn main() -> classicclassics_collection::Result {
    let name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "micromages".into());
    let common = quasar_config::CommonArgs::parse_from(
        [
            "--windowed",
            "--resolution",
            "1280x720",
            "--no-audio",
            "--no-save",
            "--autoplay",
            "--smoke-frames",
            "120",
        ]
        .into_iter()
        .map(str::to_owned)
        .chain(std::env::args().skip(2)),
    )?;
    let options = Options::from_common(&common)?;
    let mut settings = Settings::default();
    settings.apply(&common)?;
    let mut session = Session::new(Records::default(), settings);
    session.launch(GameKind::from_alias(&name).ok_or("unknown game")?);
    match &mut session.screen {
        Screen::Game(Game::MicroMages(game)) => {
            game.engine.start(0, 1, false)?;
        }
        Screen::Game(Game::Jackal(game)) => {
            let assets = classicclassics_jackal::assets::Assets::load(
                &classicclassics_jackal::assets::Assets::default_directory(),
            )?;
            game.engine.start_new_game(1, &assets);
            game.engine.state = classicclassics_jackal::game::GameState::Playing;
        }
        _ => return Err("choose micromages or jackal".into()),
    }
    desktop::run(
        options,
        session,
        quasar_storage::GamePaths::discover(None)?,
        desktop::SavePolicy {
            controls: false,
            settings: false,
            records: false,
        },
    )
}
