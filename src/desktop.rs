//! Native desktop host: Quasar window, input, renderer, audio, storage and
//! fixed-step timing around the shared `Session`.

use classicclassics::audio::{Audio, SoundBank};
use classicclassics::cli::Options;
use classicclassics::controls::{self, ControlHints, Controls};
use classicclassics::settings::Settings;
use classicclassics::{HEIGHT, Input, Session, TICK_HZ, WIDTH};
use quasar_core::FixedStep;
#[cfg(feature = "gamepad")]
use quasar_input::GilrsGamepads;
use quasar_input::{ActionState, Bindings, GamepadAxis, GamepadHub, InputState, PlayerId};
use quasar_render_2d::{DisplayOptions, Filter, PresentOutcome, Renderer2D, Scaling, Textures};
use quasar_storage::GamePaths;
use quasar_window::{
    GameWindow, WindowConfig,
    winit::{
        application::ApplicationHandler,
        event::{DeviceEvent, DeviceId, WindowEvent},
        event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
        window::WindowId,
    },
};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct SavePolicy {
    pub controls: bool,
    pub settings: bool,
    pub records: bool,
}

pub fn run(
    options: Options,
    session: Session,
    controls: Controls,
    paths: GamePaths,
    save: SavePolicy,
) -> classicclassics::Result {
    // The session owns the live settings so the settings overlay can change
    // them. This copy is the last state applied to the window/renderer/audio.
    let settings = session.settings.clone();

    let gamepads = Gamepads::new(controls.bindings.clone());
    let mut audio = Audio::open(
        settings.audio && !options.no_audio,
        settings.volume,
        options.audio_device.as_deref(),
    );
    if let Some(audio) = &mut audio {
        audio.set_muted(settings.muted);
    }
    let hints = controls.hints();
    let mut app = App {
        presenter: None,
        textures: Textures::default(),
        window: None,
        options,
        session,
        controls,
        hints,
        save,
        settings,
        paths,
        clock: FixedStep::new(TICK_HZ, 8)?,
        previous: Instant::now(),
        started: Instant::now(),
        frames: 0,
        ticks: 0,
        exercise_stage: 0,
        error: None,
        bank: SoundBank::generate(),
        audio,
        gamepads,
        stick: StickNav::default(),
        fullscreen_request: None,
        size_request: None,
    };
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.error.take() {
        return Err(error.into());
    }
    Ok(())
}

/// The subset of Quasar's gamepad hub that the collection needs. Keyboard is
/// always handled by `GameWindow`; this adds hotplugged controllers through the
/// same named action bindings.
struct Gamepads {
    hub: GamepadHub,
    player: PlayerId,
    fallback: InputState,
    #[cfg(feature = "gamepad")]
    backend: Option<GilrsGamepads>,
}

impl Gamepads {
    fn new(bindings: Bindings) -> Self {
        let player = PlayerId::new(0);
        let mut hub = GamepadHub::new();
        let _ = hub.add_player_with_bindings(player, bindings.clone());
        let mut fallback = InputState::default();
        let _ = fallback.set_bindings(bindings);
        #[cfg(feature = "gamepad")]
        let backend = match GilrsGamepads::new() {
            Ok(backend) => Some(backend),
            Err(error) => {
                eprintln!("Gamepad backend unavailable ({error}); using keyboard only");
                None
            }
        };
        Self {
            hub,
            player,
            fallback,
            #[cfg(feature = "gamepad")]
            backend,
        }
    }

    fn poll(&mut self) {
        #[cfg(feature = "gamepad")]
        if let Some(backend) = &mut self.backend {
            let _ = backend.poll(&mut self.hub);
        }
    }

    fn input(&self) -> &InputState {
        self.hub
            .player(self.player)
            .map(|player| &player.input)
            .unwrap_or(&self.fallback)
    }

    fn consume_edges(&mut self) {
        if let Some(player) = self.hub.player_mut(self.player) {
            player.input.consume_edges();
        } else {
            self.fallback.consume_edges();
        }
    }

    fn release_all(&mut self) {
        if let Some(player) = self.hub.player_mut(self.player) {
            player.release_all();
        } else {
            self.fallback.release_all();
        }
    }
}

fn merged_action(keyboard: &InputState, gamepad: &InputState, name: &str) -> ActionState {
    let keyboard = keyboard.action(name);
    let gamepad = gamepad.action(name);
    ActionState {
        value: (keyboard.value + gamepad.value).clamp(-1.0, 1.0),
        pressed: keyboard.pressed || gamepad.pressed,
        released: keyboard.released || gamepad.released,
    }
}

fn axis(value: f32) -> i8 {
    if value > 0.22 {
        1
    } else if value < -0.22 {
        -1
    } else {
        0
    }
}

#[derive(Default)]
struct StickNav {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

fn build_input(keyboard: &InputState, gamepad: &InputState, stick: &mut StickNav) -> Input {
    let confirm = merged_action(keyboard, gamepad, controls::CONFIRM);
    let back = merged_action(keyboard, gamepad, controls::BACK);
    let fire = merged_action(keyboard, gamepad, controls::FIRE);
    let pause = merged_action(keyboard, gamepad, controls::PAUSE);
    let restart = merged_action(keyboard, gamepad, controls::RESTART);
    let mode = merged_action(keyboard, gamepad, controls::MODE);
    let mute = merged_action(keyboard, gamepad, controls::MUTE);
    let quit = merged_action(keyboard, gamepad, controls::QUIT);
    let settings = merged_action(keyboard, gamepad, controls::SETTINGS);
    // D-pad/keyboard menu edges come from named actions. The analog stick is
    // latched here so a fast -1 -> +1 sweep still produces exactly one edge per
    // direction instead of relying on the action passing through zero.
    let up = merged_action(keyboard, gamepad, controls::MENU_UP);
    let down = merged_action(keyboard, gamepad, controls::MENU_DOWN);
    let left = merged_action(keyboard, gamepad, controls::MENU_LEFT);
    let right = merged_action(keyboard, gamepad, controls::MENU_RIGHT);
    let stick_x = gamepad.gamepad_axis_value(GamepadAxis::LeftStickX);
    let stick_y = gamepad.gamepad_axis_value(GamepadAxis::LeftStickY);
    let now_up = stick_y < -0.55;
    let now_down = stick_y > 0.55;
    let now_left = stick_x < -0.55;
    let now_right = stick_x > 0.55;
    let stick_up = now_up && !stick.up;
    let stick_down = now_down && !stick.down;
    let stick_left = now_left && !stick.left;
    let stick_right = now_right && !stick.right;
    stick.up = now_up;
    stick.down = now_down;
    stick.left = now_left;
    stick.right = now_right;
    let (menu_up, menu_down) = if stick_up {
        (true, false)
    } else if stick_down {
        (false, true)
    } else if up.pressed && !down.pressed {
        (true, false)
    } else if down.pressed {
        (false, true)
    } else {
        (false, false)
    };
    let (menu_left, menu_right) = if stick_left {
        (true, false)
    } else if stick_right {
        (false, true)
    } else if left.pressed && !right.pressed {
        (true, false)
    } else if right.pressed {
        (false, true)
    } else {
        (false, false)
    };
    Input {
        menu_up,
        menu_down,
        menu_left,
        menu_right,
        confirm: confirm.pressed,
        back: back.pressed,
        settings: settings.pressed,
        p1: axis(merged_action(keyboard, gamepad, controls::P1).value),
        p2: axis(merged_action(keyboard, gamepad, controls::P2).value),
        rotate: axis(merged_action(keyboard, gamepad, controls::ROTATE).value),
        thrust: merged_action(keyboard, gamepad, controls::THRUST).held(),
        fire: fire.held(),
        fire_pressed: fire.pressed,
        hyperspace: merged_action(keyboard, gamepad, controls::HYPERSPACE).pressed,
        pause: pause.pressed,
        restart: restart.pressed,
        mode: mode.pressed,
        mute: mute.pressed,
        quit: quit.pressed,
        fullscreen: false,
        filter: false,
        scale: false,
    }
}

struct App {
    // Graphics is explicitly destroyed before the window on suspension/exit.
    presenter: Option<Renderer2D>,
    textures: Textures,
    window: Option<GameWindow>,
    options: Options,
    session: Session,
    controls: Controls,
    hints: ControlHints,
    save: SavePolicy,
    settings: Settings,
    paths: GamePaths,
    clock: FixedStep,
    previous: Instant,
    started: Instant,
    frames: u64,
    ticks: u64,
    exercise_stage: u8,
    error: Option<String>,
    bank: SoundBank,
    audio: Option<Audio>,
    gamepads: Gamepads,
    stick: StickNav,
    fullscreen_request: Option<bool>,
    size_request: Option<(u32, u32)>,
}

impl App {
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl std::fmt::Display) {
        self.error = Some(error.to_string());
        event_loop.exit();
    }

    fn automated(&self) -> bool {
        self.options.autoplay || self.options.smoke_frames.is_some()
    }

    fn display_options(&self) -> DisplayOptions {
        DisplayOptions {
            filter: if self.settings.linear {
                Filter::Linear
            } else {
                Filter::Nearest
            },
            scaling: if self.settings.integer {
                Scaling::Integer
            } else {
                Scaling::Fit
            },
            aspect: None,
        }
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> classicclassics::Result {
        let mut window = GameWindow::create(
            event_loop,
            &WindowConfig {
                title: "Classic Classics".into(),
                application_id: "classicclassics".into(),
                size: (self.settings.width, self.settings.height),
                minimum_size: (640, 400),
                fullscreen: self.settings.fullscreen,
                resizable: true,
            },
        )?;
        window.input.set_bindings(self.controls.bindings.clone())?;
        let size = window.native().inner_size();
        self.presenter = Some(Renderer2D::for_window(
            window.shared_native(),
            (size.width.max(1), size.height.max(1)),
            self.display_options(),
        )?);
        self.window = Some(window);
        self.previous = Instant::now();
        self.clock.reset();
        Ok(())
    }

    fn redraw(&mut self) -> classicclassics::Result {
        let Some(window) = &self.window else {
            return Ok(());
        };
        if !window.drawable() {
            return Ok(());
        }
        let scene = self.session.scene(&self.hints);
        let outcome = self
            .presenter
            .as_mut()
            .expect("presenter exists with a window")
            .present(&scene, &self.textures)?;
        if outcome == PresentOutcome::Presented {
            self.frames += 1;
            if self.options.exercise_window {
                self.exercise()?;
            }
        }
        Ok(())
    }

    fn play_sounds(&mut self) {
        let Some(audio) = &self.audio else {
            self.session.sounds.clear();
            return;
        };
        for sound in self.session.sounds.drain(..) {
            let pan = match sound {
                classicclassics::audio::Sound::Paddle => 0.0,
                _ => 0.0,
            };
            audio.play(sound, &self.bank, pan);
        }
    }

    fn sync_settings(&mut self) -> classicclassics::Result {
        let desired = self.session.settings.clone();
        if let Some(window) = &self.window {
            if desired.fullscreen != window.fullscreen() {
                if self.fullscreen_request != Some(desired.fullscreen) {
                    window.set_fullscreen(desired.fullscreen);
                    self.fullscreen_request = Some(desired.fullscreen);
                }
            } else {
                self.fullscreen_request = None;
            }
            if !desired.fullscreen && !window.fullscreen() {
                let size = window
                    .native()
                    .inner_size()
                    .to_logical::<u32>(window.native().scale_factor());
                if size.width == desired.width && size.height == desired.height {
                    self.size_request = None;
                } else if self.size_request != Some((desired.width, desired.height)) {
                    let _ = window.native().request_inner_size(
                        quasar_window::winit::dpi::LogicalSize::new(desired.width, desired.height),
                    );
                    self.size_request = Some((desired.width, desired.height));
                }
            } else {
                self.size_request = None;
            }
        }
        if desired.linear != self.settings.linear || desired.integer != self.settings.integer {
            self.presenter
                .as_mut()
                .expect("presenter exists")
                .set_options(DisplayOptions {
                    filter: if desired.linear {
                        Filter::Linear
                    } else {
                        Filter::Nearest
                    },
                    scaling: if desired.integer {
                        Scaling::Integer
                    } else {
                        Scaling::Fit
                    },
                    aspect: None,
                })?;
        }
        if desired.muted != self.settings.muted
            && let Some(audio) = &mut self.audio
        {
            audio.set_muted(desired.muted);
        }
        if (desired.volume - self.settings.volume).abs() > f32::EPSILON
            && let Some(audio) = &self.audio
        {
            audio.set_volume(desired.volume);
        }
        self.settings = desired;
        Ok(())
    }

    fn save_outputs(&mut self) {
        if let Some(path) = &self.options.state
            && let Err(error) = quasar_storage::write_json(path, &self.session.state_json())
        {
            self.error.get_or_insert_with(|| error.to_string());
        }
        if self.options.no_save {
            return;
        }
        if self.save.settings
            && let Err(error) = self
                .session
                .settings
                .save(&self.paths.config.join("display.json"))
        {
            self.error.get_or_insert_with(|| error.to_string());
        }
        if self.save.records
            && let Err(error) = self
                .session
                .records
                .save(&self.paths.save.join("records.json"))
        {
            self.error.get_or_insert_with(|| error.to_string());
        }
        if self.save.controls
            && let Err(error) = quasar_storage::write_json(
                &self.paths.shared_config.join("controls.json"),
                &self.controls,
            )
        {
            self.error.get_or_insert_with(|| error.to_string());
        }
    }

    fn exercise(&mut self) -> classicclassics::Result {
        let window = self.window.as_ref().expect("window exists");
        match self.exercise_stage {
            0 if self.frames >= 2 => {
                let _ = window
                    .native()
                    .request_inner_size(quasar_window::winit::dpi::LogicalSize::new(800, 600));
                self.exercise_stage = 1;
            }
            1 if self.frames >= 5 => {
                window.set_fullscreen(true);
                self.exercise_stage = 2;
            }
            2 if self.frames >= 10 => {
                window.set_fullscreen(false);
                self.exercise_stage = 3;
            }
            3 if self.frames >= 13 => {
                let presenter = self.presenter.as_mut().expect("presenter exists");
                presenter.set_options(DisplayOptions {
                    filter: Filter::Linear,
                    scaling: Scaling::Integer,
                    aspect: None,
                })?;
                presenter.resize(0, 0);
                let scene = self.session.scene(&self.hints);
                if presenter.present(&scene, &self.textures)? != PresentOutcome::Deferred {
                    return Err("zero-size surface was not deferred".into());
                }
                let size = window.native().inner_size();
                presenter.resize(size.width, size.height);
                self.exercise_stage = 4;
            }
            _ => {}
        }
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(error) = self.initialize(event_loop)
        {
            self.fail(event_loop, error);
        }
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.presenter.take();
        self.window.take();
        self.clock.reset();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(window) = &mut self.window else {
            return;
        };
        if window.native().id() != id {
            return;
        }
        let update = window.process(&event);
        if update.close_requested {
            event_loop.exit();
            return;
        }
        if let Some(size) = update.resized {
            self.presenter
                .as_mut()
                .expect("presenter exists")
                .resize(size.width, size.height);
            if !window.fullscreen() && size.width > 0 && size.height > 0 {
                let logical = size.to_logical::<u32>(window.native().scale_factor());
                self.settings.width = logical.width.clamp(640, 16384);
                self.settings.height = logical.height.clamp(400, 16384);
                self.session.settings.width = self.settings.width;
                self.session.settings.height = self.settings.height;
            }
        }
        if update.focus_changed.is_some() {
            self.clock.reset();
            self.previous = Instant::now();
            if update.focus_changed == Some(false) {
                window.input.release_all();
                self.gamepads.release_all();
            }
        }

        if window.input.take_pressed(controls::QUIT) {
            event_loop.exit();
            return;
        }
        if window.input.take_pressed(controls::FULLSCREEN) {
            window.toggle_fullscreen();
            self.settings.fullscreen = window.fullscreen();
            self.session.settings.fullscreen = self.settings.fullscreen;
        }
        if window.input.take_pressed(controls::MUTE) {
            self.settings.muted = !self.settings.muted;
            self.session.settings.muted = self.settings.muted;
            if let Some(audio) = &mut self.audio {
                audio.set_muted(self.settings.muted);
            }
        }
        let mut options = self.presenter.as_ref().expect("presenter exists").options();
        let mut changed = false;
        if window.input.take_pressed(controls::FILTER) {
            self.settings.linear = !self.settings.linear;
            self.session.settings.linear = self.settings.linear;
            options.filter = if self.settings.linear {
                Filter::Linear
            } else {
                Filter::Nearest
            };
            changed = true;
        }
        if window.input.take_pressed(controls::SCALE) {
            self.settings.integer = !self.settings.integer;
            self.session.settings.integer = self.settings.integer;
            options.scaling = if self.settings.integer {
                Scaling::Integer
            } else {
                Scaling::Fit
            };
            changed = true;
        }
        if changed
            && let Err(error) = self
                .presenter
                .as_mut()
                .expect("presenter exists")
                .set_options(options)
        {
            self.fail(event_loop, error);
            return;
        }
        if update.redraw_requested
            && let Err(error) = self.redraw()
        {
            self.fail(event_loop, error);
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if let Some(window) = &mut self.window {
            window.process_device(&event);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.options.smoke_frames.is_some() && self.started.elapsed() > Duration::from_secs(30) {
            self.fail(
                event_loop,
                "native smoke run timed out before presenting requested frames",
            );
            return;
        }
        self.gamepads.poll();
        let automated = self.automated();
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let focused = window.focused();
        let now = Instant::now();
        if focused || automated {
            self.clock.advance(now - self.previous);
        } else {
            self.clock.reset();
        }
        self.previous = now;
        while self.clock.take_tick() {
            let input = if self.options.autoplay {
                self.session.autoplay()
            } else {
                let Some(window) = self.window.as_ref() else {
                    return;
                };
                build_input(&window.input, self.gamepads.input(), &mut self.stick)
            };
            if input.mute {
                self.settings.muted = !self.settings.muted;
                if let Some(audio) = &mut self.audio {
                    audio.set_muted(self.settings.muted);
                }
            }
            let quit = self.session.tick(input);
            if let Some(window) = &mut self.window {
                window.input.consume_edges();
            }
            self.gamepads.consume_edges();
            self.play_sounds();
            self.ticks += 1;
            if quit {
                event_loop.exit();
                break;
            }
            if self
                .options
                .ticks
                .is_some_and(|limit| self.ticks >= limit as u64)
            {
                event_loop.exit();
                break;
            }
        }
        if let Err(error) = self.sync_settings() {
            self.fail(event_loop, error);
            return;
        }
        if let Some(window) = &self.window
            && window.drawable()
        {
            window.native().request_redraw();
        }
        if self
            .options
            .smoke_frames
            .is_some_and(|frames| self.frames >= frames)
        {
            println!(
                "presented_frames={} window_exercise_stages={}",
                self.frames, self.exercise_stage
            );
            if let Some(presenter) = &self.presenter {
                println!("render_2d_stats={:?}", presenter.stats());
            }
            println!("{}", self.session.summary());
            event_loop.exit();
            return;
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            now + self.clock.until_next_tick().min(Duration::from_millis(16)),
        ));
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {
        if let Some(renderer) = &mut self.presenter
            && let Some(path) = &self.options.screenshot
        {
            let scene = self.session.scene(&self.hints);
            match renderer.render_to_image(&scene, &self.textures) {
                Ok(pixels) => {
                    if let Err(error) = quasar_render_2d::write_png(path, WIDTH, HEIGHT, &pixels) {
                        self.error.get_or_insert_with(|| error.to_string());
                    }
                }
                Err(error) => {
                    self.error.get_or_insert_with(|| error.to_string());
                }
            }
        }
        self.save_outputs();
        self.presenter.take();
        self.window.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quasar_input::{GamepadAxis, Key};

    fn states() -> (InputState, InputState) {
        let bindings = Controls::default().bindings;
        let mut keyboard = InputState::default();
        keyboard.set_bindings(bindings.clone()).unwrap();
        let mut gamepad = InputState::default();
        gamepad.set_bindings(bindings).unwrap();
        (keyboard, gamepad)
    }

    #[test]
    fn opposite_gamepad_axes_do_not_navigate_twice() {
        let (keyboard, mut gamepad) = states();
        gamepad.gamepad_axis(GamepadAxis::LeftStickY, -1.0);
        let mut stick = StickNav::default();
        let input = build_input(&keyboard, &gamepad, &mut stick);
        assert!(input.menu_up);
        assert!(!input.menu_down);
        gamepad.consume_edges();
        let input = build_input(&keyboard, &gamepad, &mut stick);
        assert!(!input.menu_up && !input.menu_down);

        gamepad.gamepad_axis(GamepadAxis::LeftStickY, 1.0);
        let input = build_input(&keyboard, &gamepad, &mut stick);
        assert!(!input.menu_up);
        assert!(input.menu_down);
    }

    #[test]
    fn keyboard_edges_and_held_fire_are_separate() {
        let (mut keyboard, gamepad) = states();
        keyboard.key(Key::KeyW, true);
        keyboard.key(Key::Space, true);
        let mut stick = StickNav::default();
        let input = build_input(&keyboard, &gamepad, &mut stick);
        assert!(input.menu_up);
        assert!(input.fire);
        assert!(input.fire_pressed);
        keyboard.consume_edges();
        let input = build_input(&keyboard, &gamepad, &mut stick);
        assert!(input.fire);
        assert!(!input.fire_pressed);
    }
}
