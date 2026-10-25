//! Game-specific command-line options on top of Quasar's `CommonArgs`.

use crate::Result;
use crate::session::GameKind;
use quasar_config::CommonArgs;
use std::path::PathBuf;

pub const HELP: &str = "\
Game options:
  --game NAME               start in pong, lunar, asteroids, or menu
  --settings                open the settings overlay at startup
  --ticks N                 fixed ticks for headless/autoplay runs
  --autoplay                deterministic attract mode
  --screenshot PATH         write a PNG after the run
  --state PATH              write a JSON state snapshot after the run
  --smoke-frames N          present N window frames and exit
  --exercise-window         resize/fullscreen/filter scaling smoke test
  --linear                  start with linear filtering
  --integer-scale           start with integer scaling
  --no-save                 do not read or write controls, settings, records
  --bindings PATH           use an explicit controls preset for this run
  --write-default-bindings PATH
  --check-bindings PATH
";

#[derive(Clone, Debug)]
pub struct Options {
    pub headless: bool,
    pub fullscreen: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub linear: bool,
    pub integer: bool,
    pub no_audio: bool,
    pub audio_device: Option<String>,
    pub no_save: bool,
    pub game: Option<GameKind>,
    pub settings: bool,
    pub ticks: Option<usize>,
    pub autoplay: bool,
    pub screenshot: Option<PathBuf>,
    pub state: Option<PathBuf>,
    pub smoke_frames: Option<u64>,
    pub exercise_window: bool,
    pub bindings: Option<PathBuf>,
    pub write_default_bindings: Option<PathBuf>,
    pub check_bindings: Option<PathBuf>,
}

impl Options {
    pub fn from_common(common: &CommonArgs) -> Result<Self> {
        let mut options = Self {
            headless: common.is_headless(),
            fullscreen: common.fullscreen.unwrap_or(false),
            width: common.resolution.map(|resolution| resolution.width),
            height: common.resolution.map(|resolution| resolution.height),
            linear: false,
            integer: false,
            no_audio: common.no_audio.unwrap_or(false),
            audio_device: common.audio_device.clone(),
            no_save: false,
            game: None,
            settings: false,
            ticks: None,
            autoplay: false,
            screenshot: None,
            state: None,
            smoke_frames: None,
            exercise_window: false,
            bindings: None,
            write_default_bindings: None,
            check_bindings: None,
        };
        let mut arguments = common.remaining.clone().into_iter();
        while let Some(argument) = arguments.next() {
            let (flag, inline) = argument
                .split_once('=')
                .map_or((argument.as_str(), None), |(flag, value)| {
                    (flag, Some(value))
                });
            let mut value = || -> Result<String> {
                inline
                    .map(str::to_owned)
                    .or_else(|| arguments.next())
                    .ok_or_else(|| format!("{flag} requires a value").into())
            };
            match flag {
                "--game" => {
                    options.game = Some(match value()?.as_str() {
                        "pong" => GameKind::Pong,
                        "lunar" | "lunar-lander" => GameKind::LunarLander,
                        "asteroids" => GameKind::Asteroids,
                        "menu" => {
                            options.game = None;
                            continue;
                        }
                        other => {
                            return Err(format!(
                                "unknown game '{other}'; use pong, lunar, asteroids or menu"
                            )
                            .into());
                        }
                    });
                }
                "--settings" => options.settings = true,
                "--ticks" => options.ticks = Some(value()?.parse()?),
                "--autoplay" => options.autoplay = true,
                "--screenshot" => options.screenshot = Some(value()?.into()),
                "--state" => options.state = Some(value()?.into()),
                "--smoke-frames" => options.smoke_frames = Some(value()?.parse()?),
                "--exercise-window" => options.exercise_window = true,
                "--linear" => options.linear = true,
                "--integer-scale" => options.integer = true,
                "--no-save" => options.no_save = true,
                "--bindings" => options.bindings = Some(value()?.into()),
                "--write-default-bindings" => {
                    options.write_default_bindings = Some(value()?.into())
                }
                "--check-bindings" => options.check_bindings = Some(value()?.into()),
                other => return Err(format!("unknown argument: {other}; use --help").into()),
            }
        }
        if options.ticks == Some(0) || options.smoke_frames == Some(0) {
            return Err("run limits must be greater than zero".into());
        }
        if options.headless && (options.smoke_frames.is_some() || options.exercise_window) {
            return Err("window smoke options require a desktop run".into());
        }
        if options.exercise_window && options.smoke_frames.is_none_or(|frames| frames < 20) {
            return Err("--exercise-window requires --smoke-frames of at least 20".into());
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_game_and_rejects_bad_runs() {
        for (name, kind) in [
            ("pong", GameKind::Pong),
            ("lunar", GameKind::LunarLander),
            ("asteroids", GameKind::Asteroids),
        ] {
            let common = CommonArgs {
                remaining: vec!["--game".into(), name.into()],
                ..CommonArgs::default()
            };
            assert_eq!(Options::from_common(&common).unwrap().game, Some(kind));
        }
        for remaining in [
            vec!["--game".into(), "pong".into(), "--ticks".into(), "0".into()],
            vec!["--exercise-window".into()],
            vec!["--unknown".into()],
        ] {
            let common = CommonArgs {
                remaining,
                ..CommonArgs::default()
            };
            assert!(Options::from_common(&common).is_err());
        }
    }
}
