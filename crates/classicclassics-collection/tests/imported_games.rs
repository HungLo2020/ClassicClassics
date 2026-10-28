//! Prove the adapters retain the upstream simulations and share logical input.
use classicclassics_core::{ArcadeGame, Input, input::PlayerControls};
use serde::Serialize;

fn state(value: &impl Serialize) -> serde_json::Value {
    serde_json::to_value(value).unwrap()
}

#[test]
fn micro_mages_keeps_original_solo_and_cooperative_controls_and_sixty_hz() {
    let mut game = classicclassics_micromages::Game::default();
    game.engine.start(0, 2, false).unwrap();
    let mut original = game.engine.clone();
    let input = Input {
        move_x: 1,
        move_y: -1,
        jump: true,
        fire: true,
        other_players: [
            PlayerControls {
                x: -1,
                fire: true,
                ..Default::default()
            },
            Default::default(),
            Default::default(),
        ],
        ..Default::default()
    };
    for _ in 0..120 {
        game.tick(input, &mut Vec::new());
    }
    for _ in 0..60 {
        original.update([147, 66, 0, 0]);
    }
    assert_eq!(state(&game.engine), state(&original));
    // Fire is not Jump during gameplay, even if Confirm shares Space/South.
    let mut game = classicclassics_micromages::Game::default();
    game.engine.start(0, 1, false).unwrap();
    let mut original = game.engine.clone();
    game.tick(
        Input {
            fire: true,
            confirm: true,
            ..Default::default()
        },
        &mut Vec::new(),
    );
    game.tick(
        Input {
            fire: true,
            ..Default::default()
        },
        &mut Vec::new(),
    );
    original.update([2, 0, 0, 0]);
    assert_eq!(state(&game.engine), state(&original));
}

#[test]
fn jackal_keeps_original_two_player_movement_gun_and_grenade_rules() {
    let assets = classicclassics_jackal::assets::Assets::load(
        &classicclassics_jackal::assets::Assets::default_directory(),
    )
    .unwrap();
    let mut game = classicclassics_jackal::Game::default();
    game.engine.start_new_game(2, &assets);
    let mut original = game.engine.clone();
    let input = Input {
        move_y: -1,
        fire: true,
        secondary: true,
        other_players: [
            PlayerControls {
                x: 1,
                fire: true,
                ..Default::default()
            },
            Default::default(),
            Default::default(),
        ],
        ..Default::default()
    };
    for _ in 0..120 {
        game.tick(input, &mut Vec::new());
    }
    for _ in 0..60 {
        original.update(
            classicclassics_jackal::input::Input {
                players: [
                    classicclassics_jackal::input::PlayerInput {
                        up: true,
                        gun: true,
                        grenade: true,
                        ..Default::default()
                    },
                    classicclassics_jackal::input::PlayerInput {
                        right: true,
                        gun: true,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
            &assets,
        );
        original.drain_events();
    }
    assert_eq!(state(&game.engine), state(&original));
}

#[test]
fn missile_command_retains_all_three_battery_taps_between_video_frames() {
    let mut game = classicclassics_missile_command::Game::default();
    game.engine.start();
    // Original wave startup inhibits firing for 122 video frames.
    for _ in 0..122 {
        game.engine.update(Default::default());
    }
    let mut original = game.engine.clone();
    game.tick(
        Input {
            fire_pressed: true,
            secondary_pressed: true,
            jump_pressed: true,
            ..Default::default()
        },
        &mut Vec::new(),
    );
    game.tick(Input::default(), &mut Vec::new());
    original.update(classicclassics_missile_command::game::Input {
        aim: original.aim,
        fire: [true; 3],
        start: false,
    });
    assert_eq!(state(&game.engine), state(&original));
    assert_eq!(game.engine.bases.map(|base| base.ammo), [9; 3]);
}

#[test]
fn river_raid_keeps_original_steering_speed_fuel_and_firing_rules() {
    let mut game = classicclassics_river_raid::Game::default();
    game.engine.start();
    let mut original = game.engine.clone();
    for _ in 0..120 {
        game.tick(
            Input {
                move_x: -1,
                move_y: 1,
                fire: true,
                ..Default::default()
            },
            &mut Vec::new(),
        );
    }
    for _ in 0..60 {
        original.tick(classicclassics_river_raid::game::Input {
            left: true,
            down: true,
            fire: true,
            ..Default::default()
        });
    }
    assert_eq!(state(&game.engine), state(&original));
}

#[test]
fn every_import_pauses_resumes_and_restarts_through_the_shared_actions() {
    use classicclassics_collection::games::{Game, GameKind};
    for kind in [
        GameKind::MicroMages,
        GameKind::Jackal,
        GameKind::MissileCommand,
        GameKind::RiverRaid,
    ] {
        let mut game = Game::new(kind);
        game.tick(
            Input {
                pause: true,
                ..Default::default()
            },
            &mut Vec::new(),
        );
        assert!(game.paused());
        let paused = state(&game);
        for _ in 0..120 {
            game.tick(
                Input {
                    fire: true,
                    move_x: 1,
                    ..Default::default()
                },
                &mut Vec::new(),
            );
        }
        assert_eq!(state(&game), paused);
        game.tick(
            Input {
                pause: true,
                ..Default::default()
            },
            &mut Vec::new(),
        );
        assert!(!game.paused());
        game.tick(
            Input {
                restart: true,
                ..Default::default()
            },
            &mut Vec::new(),
        );
        let mut fresh = Game::new(kind);
        fresh.tick(Input::default(), &mut Vec::new());
        assert_eq!(state(&game), state(&fresh));
    }
}

#[test]
fn original_music_and_effect_assets_decode_through_quasar() {
    use classicclassics_core::{
        Sound,
        audio::{AssetSound, MusicLoop},
    };
    let mut cues = Vec::new();
    for (file, looping, start, end) in classicclassics_micromages::audio_cues::MUSIC {
        cues.push(AssetSound::Music {
            game: "micromages",
            file,
            region: if !looping {
                MusicLoop::Once
            } else if end > start {
                MusicLoop::Milliseconds((start * 1000.0) as u64, (end * 1000.0) as u64)
            } else {
                MusicLoop::Whole
            },
        });
    }
    for (file, _) in classicclassics_micromages::audio_cues::SFX {
        cues.push(AssetSound::Effect {
            game: "micromages",
            file,
        });
    }
    for id in 0..=0x59 {
        if let Some(Sound::Asset(cue)) = classicclassics_jackal::sound::cue(
            classicclassics_jackal::game::GameEvent::AudioCommand(id),
        ) {
            cues.push(cue);
        }
    }
    for cue in cues {
        let (game, file, region) = match cue {
            AssetSound::Music { game, file, region } => (game, file, region),
            AssetSound::Effect { game, file } => (game, file, MusicLoop::Once),
            AssetSound::StopMusic => continue,
        };
        let root = if game == "micromages" {
            classicclassics_micromages::assets::default_root()
        } else {
            classicclassics_jackal::assets::Assets::default_directory()
        };
        let clip = quasar_audio::Clip::load(root.join(file)).unwrap();
        assert!(clip.frames() > 0, "{game}/{file}");
        match region {
            MusicLoop::FromFrame(start) => assert!(start < clip.frames(), "{game}/{file}"),
            MusicLoop::Milliseconds(start, end) => {
                assert!(start < end);
                assert!(
                    start * u64::from(clip.sample_rate()) / 1000
                        < (end * u64::from(clip.sample_rate()) / 1000).min(clip.frames()),
                    "{game}/{file}"
                );
            }
            _ => {}
        }
    }
}

#[test]
fn micro_mages_setup_uses_the_same_confirm_and_difficulty_mapping_for_every_player() {
    use classicclassics_micromages::{Game, game::Screen};
    for other_player in [false, true] {
        for hard_mode in [false, true] {
            let mut game = Game::default();
            game.engine.screen = Screen::PlayerSelect;
            let mut input = Input::default();
            if other_player {
                input.other_players[0] = PlayerControls {
                    fire: !hard_mode,
                    secondary: hard_mode,
                    ..Default::default()
                };
            } else {
                input.fire = !hard_mode;
                input.confirm = !hard_mode;
                input.secondary = hard_mode;
                input.secondary_pressed = hard_mode;
                input.fire_pressed = !hard_mode;
            }
            game.tick(input, &mut Vec::new());
            game.tick(Input::default(), &mut Vec::new());
            assert_eq!(game.engine.hard, hard_mode);
            assert_eq!(
                game.engine.screen,
                if hard_mode {
                    Screen::PlayerSelect
                } else {
                    Screen::Password
                }
            );
        }
    }
}

#[test]
fn micro_mages_autoplay_advances_setup_and_reaches_gameplay() {
    let mut game = classicclassics_micromages::Game::default();
    let mut reached_gameplay = false;
    for _ in 0..1200 {
        game.tick(game.autoplay(), &mut Vec::new());
        reached_gameplay |= game.engine.screen == classicclassics_micromages::game::Screen::Playing;
    }
    assert!(reached_gameplay, "{}", game.summary());
}
