//! Native desktop host: Quasar window, input, renderer, audio, storage and
//! fixed-step timing around the shared `Session`.

use classicclassics_collection::audio::{Audio, SoundBank};
use classicclassics_collection::cli::Options;
use classicclassics_collection::controls::{self, Controls};
use classicclassics_collection::controls_screen::{CaptureEvent, ControlsScreen};
use classicclassics_collection::rendering::{CatalogArt, SceneAssets, SceneTarget};
use classicclassics_collection::settings::Settings;
use classicclassics_collection::{Input, Session, TICK_HZ};
use quasar_core::FixedStep;
#[cfg(feature = "gamepad")]
use quasar_input::GilrsGamepads;
use quasar_input::{
    ActionState, Bindings, Button, GamepadAxis, GamepadButton, GamepadHub, InputState, Key,
    MouseButton, PlayerId,
};
use quasar_render_2d::{
    DisplayOptions, DynamicTexture, Filter, Image, PresentOutcome, Renderer2D, Scaling, TextureId,
    Viewport,
};
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
    mut session: Session,
    paths: GamePaths,
    save: SavePolicy,
) -> classicclassics_collection::Result {
    // The session owns the live settings so the settings overlay can change
    // them. This copy is the last state applied to the window/renderer/audio.
    let settings = session.settings.clone();

    let controls = session.controls.clone();
    let gamepads = Gamepads::new(controls.bindings.clone());
    let mut audio = Audio::open(
        settings.audio && !options.no_audio,
        settings.volume,
        options.audio_device.as_deref(),
    );
    if let Some(audio) = &mut audio {
        if let Err(error) = audio.prepare(session.game_kind(), &mut session.assets) {
            session.asset_failure(error);
        }
        audio.set_muted(settings.muted);
    }
    let profile = options.smoke_frames.map(|_| NativeProfile::default());
    let mut app = App {
        presenter: None,
        assets: SceneAssets::with_assets(&mut session.assets)?,
        pixel_texture: None,
        profile,
        window: None,
        options,
        session,
        controls,
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
        was_capturing: false,
        last_game_pointer: None,
        capture_axes: [false; 6],
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
        for slot in 0..4 {
            let _ = hub.add_player_with_bindings(PlayerId::new(slot), bindings.clone());
        }
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

    fn set_bindings(&mut self, bindings: Bindings) -> classicclassics_collection::Result {
        self.fallback.set_bindings(bindings.clone())?;
        for player in self.hub.players_mut() {
            player.input.set_bindings(bindings.clone())?;
        }
        Ok(())
    }

    fn poll(&mut self) {
        #[cfg(feature = "gamepad")]
        if let Some(backend) = &mut self.backend {
            let _ = backend.poll(&mut self.hub);
        }
        let assigned: Vec<_> = (0..4)
            .filter_map(|slot| self.hub.assignment(PlayerId::new(slot)))
            .collect();
        let unassigned: Vec<_> = self
            .hub
            .devices()
            .iter()
            .map(|device| device.id)
            .filter(|id| !assigned.contains(id))
            .collect();
        for device in unassigned {
            if let Some(slot) =
                (0..4).find(|slot| self.hub.assignment(PlayerId::new(*slot)).is_none())
            {
                let _ = self.hub.assign(PlayerId::new(slot), device);
            }
        }
    }

    fn input(&self) -> &InputState {
        self.hub
            .player(self.player)
            .map(|player| &player.input)
            .unwrap_or(&self.fallback)
    }

    fn consume_edges(&mut self) {
        for player in self.hub.players_mut() {
            player.input.consume_edges();
        }
        self.fallback.consume_edges();
    }

    fn release_all(&mut self) {
        for player in self.hub.players_mut() {
            player.release_all();
        }
        self.fallback.release_all();
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
    let stick_x = merged_action(keyboard, gamepad, controls::MENU_HORIZONTAL).value;
    let stick_y = merged_action(keyboard, gamepad, controls::MENU_VERTICAL).value;
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
        settings: settings.pressed || keyboard.pressed(Key::Escape),
        move_x: axis(merged_action(keyboard, gamepad, controls::MOVE_HORIZONTAL).value),
        move_y: axis(merged_action(keyboard, gamepad, controls::MOVE_VERTICAL).value),
        p1: axis(merged_action(keyboard, gamepad, controls::P1).value),
        p2: axis(merged_action(keyboard, gamepad, controls::P2).value),
        rotate: axis(merged_action(keyboard, gamepad, controls::ROTATE).value),
        thrust: merged_action(keyboard, gamepad, controls::THRUST).held(),
        jump: merged_action(keyboard, gamepad, controls::JUMP).held(),
        jump_pressed: merged_action(keyboard, gamepad, controls::JUMP).pressed,
        secondary: merged_action(keyboard, gamepad, controls::HYPERSPACE).held(),
        secondary_pressed: merged_action(keyboard, gamepad, controls::HYPERSPACE).pressed,
        other_players: [Default::default(); 3],
        pointer: None,
        fire: fire.held(),
        fire_pressed: fire.pressed,
        hyperspace: merged_action(keyboard, gamepad, controls::HYPERSPACE).pressed,
        pause: pause.pressed,
        restart: restart.pressed,
        mode: mode.pressed,
        mute: mute.pressed,
        quit: quit.pressed,
        fullscreen: merged_action(keyboard, gamepad, controls::FULLSCREEN).pressed,
        filter: merged_action(keyboard, gamepad, controls::FILTER).pressed,
        scale: merged_action(keyboard, gamepad, controls::SCALE).pressed,
    }
}

const CAPTURE_AXES: [GamepadAxis; 6] = [
    GamepadAxis::LeftStickX,
    GamepadAxis::LeftStickY,
    GamepadAxis::RightStickX,
    GamepadAxis::RightStickY,
    GamepadAxis::LeftTrigger,
    GamepadAxis::RightTrigger,
];

fn gamepad_capture(
    input: &InputState,
    armed: &mut [bool; 6],
    capturing: bool,
) -> Vec<CaptureEvent> {
    let mut events = Vec::new();
    if capturing {
        for button in [
            GamepadButton::South,
            GamepadButton::East,
            GamepadButton::North,
            GamepadButton::West,
            GamepadButton::C,
            GamepadButton::Z,
            GamepadButton::LeftBumper,
            GamepadButton::RightBumper,
            GamepadButton::LeftTrigger,
            GamepadButton::RightTrigger,
            GamepadButton::Select,
            GamepadButton::Start,
            GamepadButton::Mode,
            GamepadButton::LeftStick,
            GamepadButton::RightStick,
            GamepadButton::DPadUp,
            GamepadButton::DPadDown,
            GamepadButton::DPadLeft,
            GamepadButton::DPadRight,
        ] {
            if input.button_pressed(Button::Gamepad(button)) {
                events.push(CaptureEvent::Button {
                    button: Button::Gamepad(button),
                    pressed: true,
                    modifiers: Vec::new(),
                });
            }
        }
    }
    for (index, axis) in CAPTURE_AXES.into_iter().enumerate() {
        let value = input.gamepad_axis_value(axis);
        if !capturing || value.abs() < 0.25 {
            armed[index] = value.abs() < 0.25;
        } else if armed[index] && value.abs() >= 0.65 {
            events.push(CaptureEvent::Axis { axis, value });
            armed[index] = false;
        }
    }
    events
}

#[derive(Default)]
struct NativeProfile {
    frames: Vec<[f64; 5]>,
    last: Option<Instant>,
}
impl NativeProfile {
    fn record(&mut self, scene: Duration, present: Duration, stats: quasar_render_2d::RenderStats) {
        let now = Instant::now();
        let interval = self
            .last
            .map_or(0.0, |last| (now - last).as_secs_f64() * 1000.0);
        self.last = Some(now);
        self.frames.push([
            scene.as_secs_f64() * 1000.0,
            present.as_secs_f64() * 1000.0,
            interval,
            stats.geometry_upload_bytes as f64,
            stats.texture_upload_bytes as f64,
        ]);
    }
    fn report(&self) {
        if self.frames.is_empty() {
            return;
        }
        // Exclude startup/setup from steady-state frame timing.
        let frames = &self.frames[self.frames.len() / 3..];
        let percentile = |column: usize| {
            let mut values: Vec<_> = frames.iter().map(|frame| frame[column]).collect();
            values.sort_by(f64::total_cmp);
            values[values.len() * 95 / 100]
        };
        let mean = |column: usize| {
            frames.iter().map(|frame| frame[column]).sum::<f64>() / frames.len() as f64
        };
        println!(
            "native_profile samples={} scene_p95_ms={:.3} scene_max_ms={:.3} present_p95_ms={:.3} interval_p95_ms={:.3} intervals_over_25ms={} mean_geometry_bytes={:.0} mean_texture_bytes={:.0}",
            frames.len(),
            percentile(0),
            frames.iter().map(|v| v[0]).fold(0.0, f64::max),
            percentile(1),
            percentile(2),
            frames.iter().filter(|v| v[2] > 25.0).count(),
            mean(3),
            mean(4)
        );
    }
}

struct GpuSceneTarget<'a> {
    renderer: &'a mut Renderer2D,
    assets: &'a mut SceneAssets,
    pixel: &'a mut Option<(DynamicTexture, Image)>,
}
impl SceneTarget for GpuSceneTarget<'_> {
    fn artwork(&self) -> &CatalogArt {
        &self.assets.artwork
    }
    fn upload_frame(
        &mut self,
        frame: &classicclassics_collection::rendering::PixelFrame,
    ) -> classicclassics_collection::Result<TextureId> {
        let size = (frame.image.width(), frame.image.height());
        if self
            .pixel
            .as_ref()
            .is_some_and(|(texture, _)| texture.size() != size)
            && let Some((texture, _)) = self.pixel.take()
        {
            self.renderer
                .destroy_dynamic_texture(&mut self.assets.textures, texture)?;
        }
        if let Some((texture, image)) = self.pixel.as_mut() {
            if !std::ptr::eq(image.rgba().as_ptr(), frame.image.rgba().as_ptr()) {
                self.renderer
                    .update_dynamic_texture(*texture, frame.image.rgba())?;
                *image = frame.image.clone();
            }
            return Ok(texture.id());
        }
        let texture =
            self.renderer
                .create_dynamic_texture(&mut self.assets.textures, size.0, size.1)?;
        self.renderer
            .update_dynamic_texture(texture, frame.image.rgba())?;
        *self.pixel = Some((texture, frame.image.clone()));
        Ok(texture.id())
    }
}

struct App {
    // Graphics is explicitly destroyed before the window on suspension/exit.
    presenter: Option<Renderer2D>,
    assets: SceneAssets,
    pixel_texture: Option<(DynamicTexture, Image)>,
    profile: Option<NativeProfile>,
    window: Option<GameWindow>,
    options: Options,
    session: Session,
    controls: Controls,
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
    was_capturing: bool,
    last_game_pointer: Option<(f64, f64)>,
    capture_axes: [bool; 6],
    fullscreen_request: Option<bool>,
    size_request: Option<(u32, u32)>,
}

impl App {
    fn scene(&mut self) -> classicclassics_collection::Result<quasar_render_2d::Canvas> {
        let mut target = GpuSceneTarget {
            renderer: self.presenter.as_mut().expect("presenter exists"),
            assets: &mut self.assets,
            pixel: &mut self.pixel_texture,
        };
        self.session.render(&mut target)
    }

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

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> classicclassics_collection::Result {
        // Quasar exposes winit's monitor handles; borderless fullscreen uses
        // the compositor's native monitor mode without a display-mode switch.
        let monitor = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next());
        let monitor_size = monitor.map(|monitor| {
            let size = monitor.size().to_logical::<u32>(monitor.scale_factor());
            (size.width, size.height)
        });
        self.session.settings.resolve_monitor(monitor_size);
        self.settings = self.session.settings.clone();
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
        if let Some(monitor) = window.native().current_monitor() {
            let size = monitor.size().to_logical::<u32>(monitor.scale_factor());
            self.session
                .settings
                .resolve_monitor(Some((size.width, size.height)));
            self.settings = self.session.settings.clone();
        }
        window.input.set_bindings(self.controls.bindings.clone())?;
        let size = window.native().inner_size();
        if let Some((texture, _)) = self.pixel_texture.take() {
            self.assets.textures.remove(texture.id());
        }
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

    fn redraw(&mut self) -> classicclassics_collection::Result {
        let Some(window) = &self.window else {
            return Ok(());
        };
        if !window.drawable() {
            return Ok(());
        }
        let started = Instant::now();
        let scene = self.scene()?;
        let scene_time = started.elapsed();
        let presenting = Instant::now();
        let outcome = self
            .presenter
            .as_mut()
            .expect("presenter exists with a window")
            .present(&scene, &self.assets.textures)?;
        if outcome == PresentOutcome::Presented {
            if let Some(profile) = &mut self.profile {
                profile.record(
                    scene_time,
                    presenting.elapsed(),
                    self.presenter.as_ref().unwrap().stats(),
                );
            }
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
        let paused = self.session.settings_open()
            || matches!(&self.session.screen, classicclassics_collection::session::Screen::Game(game) if game.paused());
        audio.pause_music(paused);
        for sound in self.session.sounds.drain(..) {
            let pan = match sound {
                classicclassics_collection::audio::Sound::Paddle => 0.0,
                _ => 0.0,
            };
            audio.play(sound, &self.bank, pan);
        }
    }

    fn sync_controls(&mut self) -> classicclassics_collection::Result {
        let capturing = self.session.capturing_controls();
        let changed = self.controls != self.session.controls;
        if changed {
            self.session.controls.validate()?;
            let bindings = self.session.controls.bindings.clone();
            if let Some(window) = &mut self.window {
                window.input.set_bindings(bindings.clone())?;
            }
            self.gamepads.set_bindings(bindings)?;
            self.controls = self.session.controls.clone();
        }
        if changed || capturing != self.was_capturing {
            // A capture boundary must not leak held keys/edges into gameplay.
            if let Some(window) = &mut self.window {
                window.input.release_all();
            }
            if capturing && !self.was_capturing {
                // Keep actual stick positions for neutral-before-capture arming.
                self.gamepads.consume_edges();
            } else {
                self.gamepads.release_all();
            }
            self.stick = StickNav::default();
        }
        self.was_capturing = capturing;
        Ok(())
    }

    fn sync_settings(&mut self) -> classicclassics_collection::Result {
        if self.session.settings.auto_resolution
            && let Some(monitor) = self
                .window
                .as_ref()
                .and_then(|window| window.native().current_monitor())
        {
            let size = monitor.size().to_logical::<u32>(monitor.scale_factor());
            self.session
                .settings
                .resolve_monitor(Some((size.width, size.height)));
        }
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
                .save(&self.paths.config.join("display.toml"))
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
            && let Err(error) = self
                .session
                .controls
                .save(&self.paths.shared_config.join("controls.toml"))
        {
            self.error.get_or_insert_with(|| error.to_string());
        }
    }

    fn exercise(&mut self) -> classicclassics_collection::Result {
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
                let size = window.native().inner_size();
                let presenter = self.presenter.as_mut().expect("presenter exists");
                presenter.set_options(DisplayOptions {
                    filter: Filter::Linear,
                    scaling: Scaling::Integer,
                    aspect: None,
                })?;
                presenter.resize(0, 0);
                let scene = self.scene()?;
                let presenter = self.presenter.as_mut().expect("presenter exists");
                if presenter.present(&scene, &self.assets.textures)? != PresentOutcome::Deferred {
                    return Err("zero-size surface was not deferred".into());
                }
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
                self.settings.auto_resolution = false;
                self.session.settings.auto_resolution = false;
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

        if self.session.capturing_controls() {
            let event = match &event {
                WindowEvent::KeyboardInput {
                    event,
                    is_synthetic: false,
                    ..
                } if !event.repeat => {
                    if let quasar_window::winit::keyboard::PhysicalKey::Code(code) =
                        event.physical_key
                    {
                        quasar_window::input::physical_key(code).map(|key| {
                            if key == Key::Escape && event.state.is_pressed() {
                                CaptureEvent::Cancel
                            } else {
                                let modifiers = [
                                    Key::ControlLeft,
                                    Key::ControlRight,
                                    Key::AltLeft,
                                    Key::AltRight,
                                    Key::ShiftLeft,
                                    Key::ShiftRight,
                                    Key::SuperLeft,
                                    Key::SuperRight,
                                ]
                                .into_iter()
                                .filter(|key| window.input.held(*key))
                                .map(Button::Key)
                                .collect();
                                CaptureEvent::Button {
                                    button: Button::Key(key),
                                    pressed: event.state.is_pressed(),
                                    modifiers,
                                }
                            }
                        })
                    } else {
                        None
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    use quasar_window::winit::event::MouseButton as NativeButton;
                    let button = match button {
                        NativeButton::Left => MouseButton::Left,
                        NativeButton::Right => MouseButton::Right,
                        NativeButton::Middle => MouseButton::Middle,
                        NativeButton::Back => MouseButton::Back,
                        NativeButton::Forward => MouseButton::Forward,
                        NativeButton::Other(value) => MouseButton::Other(*value),
                    };
                    Some(CaptureEvent::Button {
                        button: Button::Mouse(button),
                        pressed: state.is_pressed(),
                        modifiers: Vec::new(),
                    })
                }
                WindowEvent::Focused(false) => Some(CaptureEvent::Cancel),
                _ => None,
            };
            if let Some(event) = event {
                self.session.capture_control(event);
                window.input.consume_edges();
            }
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
        let capture_events = gamepad_capture(
            self.gamepads.input(),
            &mut self.capture_axes,
            self.session.capturing_controls(),
        );
        if !capture_events.is_empty() {
            for event in capture_events {
                self.session.capture_control(event);
            }
            self.gamepads.consume_edges();
        }
        if let Err(error) = self.sync_controls() {
            self.fail(event_loop, error);
            return;
        }
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
            let mut input = if self.options.autoplay {
                self.session.autoplay()
            } else {
                let Some(window) = self.window.as_ref() else {
                    return;
                };
                build_input(&window.input, self.gamepads.input(), &mut self.stick)
            };
            if !self.options.autoplay
                && let Some(window) = &self.window
            {
                for (index, slot) in (1..4).enumerate() {
                    let pad = self
                        .gamepads
                        .hub
                        .player(PlayerId::new(slot))
                        .map(|player| &player.input)
                        .unwrap_or(&self.gamepads.fallback);
                    let mut state = classicclassics_collection::input::PlayerControls {
                        x: axis(pad.action(controls::MOVE_HORIZONTAL).value),
                        y: axis(pad.action(controls::MOVE_VERTICAL).value),
                        fire: pad.action(controls::FIRE).held()
                            || pad.action(controls::FIRE).pressed,
                        jump: pad.action(controls::JUMP).held()
                            || pad.action(controls::JUMP).pressed,
                        secondary: pad.action(controls::HYPERSPACE).held()
                            || pad.action(controls::HYPERSPACE).pressed,
                    };
                    if slot == 1 {
                        state.x = axis(
                            (f32::from(state.x)
                                + window.input.action(controls::P2_HORIZONTAL).value)
                                .clamp(-1.0, 1.0),
                        );
                        state.y = axis(
                            (f32::from(state.y) + window.input.action(controls::P2).value)
                                .clamp(-1.0, 1.0),
                        );
                        state.fire |= window.input.action(controls::P2_FIRE).held()
                            || window.input.action(controls::P2_FIRE).pressed;
                        state.jump |= window.input.action(controls::P2_JUMP).held()
                            || window.input.action(controls::P2_JUMP).pressed;
                        state.secondary |= window.input.action(controls::P2_SECONDARY).held()
                            || window.input.action(controls::P2_SECONDARY).pressed;
                    }
                    input.other_players[index] = state;
                }
                let cursor = window.input.cursor_physical;
                if cursor != self.last_game_pointer {
                    self.last_game_pointer = cursor;
                    let size = window.native().inner_size();
                    input.pointer = Viewport::calculate(
                        self.session.canvas_size(),
                        (size.width, size.height),
                        self.presenter.as_ref().expect("presenter exists").options(),
                    )
                    .and_then(|view| cursor.and_then(|(x, y)| view.to_source(x, y)))
                    .map(|(x, y)| [x as f32, y as f32]);
                }
            }
            // Recovery navigation keeps the shared settings accessible even
            // after the player clears/remaps all menu actions.
            if self.session.settings_open()
                && !self.session.capturing_controls()
                && let Some(window) = &self.window
            {
                input.menu_up |= window.input.pressed(Key::ArrowUp);
                input.menu_down |= window.input.pressed(Key::ArrowDown);
                input.menu_left |= window.input.pressed(Key::ArrowLeft);
                input.menu_right |= window.input.pressed(Key::ArrowRight);
                input.confirm |= window.input.pressed(Key::Enter);
            }
            if self.session.controls_screen.is_some()
                && !self.session.capturing_controls()
                && let Some(window) = &self.window
            {
                let size = window.native().inner_size();
                let source = self.session.canvas_size();
                let pointer = Viewport::calculate(
                    source,
                    (size.width, size.height),
                    self.presenter.as_ref().expect("presenter exists").options(),
                )
                .and_then(|viewport| {
                    window
                        .input
                        .cursor_physical
                        .and_then(|(x, y)| viewport.to_source(x, y))
                })
                .map(|point| ControlsScreen::pointer_position(source, point));
                let shift = window.input.held(Key::ShiftLeft) || window.input.held(Key::ShiftRight);
                self.session
                    .controls_screen
                    .as_mut()
                    .expect("controls open")
                    .set_pointer(quasar_ui::UiInput {
                        pointer,
                        primary_pressed: window.input.mouse_pressed(MouseButton::Left),
                        primary_down: window.input.mouse_held(MouseButton::Left),
                        primary_released: window.input.mouse_released(MouseButton::Left),
                        delete: window.input.pressed(Key::Delete),
                        next: window.input.pressed(Key::Tab) && !shift,
                        previous: window.input.pressed(Key::Tab) && shift,
                        ..Default::default()
                    });
            }
            let previous_game = self.session.game_kind();
            let previous_error = self.session.asset_error.is_some();
            let quit = self.session.tick(input);
            if self.session.game_kind() != previous_game
                || (!previous_error && self.session.asset_error.is_some())
            {
                if let Some(audio) = &mut self.audio
                    && let Err(error) =
                        audio.prepare(self.session.game_kind(), &mut self.session.assets)
                {
                    self.session.asset_failure(error);
                }
                // Static asset preparation is launch time, not elapsed gameplay.
                // Do not fast-forward a new game to catch up with loading.
                self.clock.reset();
                self.previous = Instant::now();
            }
            if let Some(window) = &mut self.window {
                window.input.consume_edges();
            }
            self.gamepads.consume_edges();
            if let Err(error) = self.sync_controls() {
                self.fail(event_loop, error);
                return;
            }
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
                "presented_frames={} window_exercise_stages={} fullscreen={} resolution={}x{}",
                self.frames,
                self.exercise_stage,
                self.settings.fullscreen,
                self.settings.width,
                self.settings.height
            );
            if let Some(presenter) = &self.presenter {
                println!("render_2d_stats={:?}", presenter.stats());
                println!("dynamic_textures={}", presenter.dynamic_texture_count());
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
        if let Some(profile) = &self.profile {
            profile.report();
        }
        if self.presenter.is_some()
            && let Some(path) = self.options.screenshot.clone()
        {
            let result = self.scene().and_then(|scene| {
                let renderer = self.presenter.as_mut().expect("presenter exists");
                Ok((
                    renderer.render_to_image(&scene, &self.assets.textures)?,
                    scene,
                ))
            });
            match result {
                Ok((pixels, scene)) => {
                    if let Err(error) =
                        quasar_render_2d::write_png(&path, scene.width(), scene.height(), &pixels)
                    {
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
    fn four_controller_slots_keep_inputs_independent_and_share_remapping() {
        let mut pads = Gamepads::new(Controls::default().bindings);
        #[cfg(feature = "gamepad")]
        {
            pads.backend = None;
        }
        let devices: Vec<_> = (0..4)
            .map(|slot| {
                pads.hub
                    .connect(quasar_input::GamepadFingerprint::new(format!(
                        "test controller {slot}"
                    )))
            })
            .collect();
        pads.poll();
        for (slot, device) in devices.iter().enumerate() {
            assert_eq!(
                pads.hub.assignment(PlayerId::new(slot as u8)),
                Some(*device)
            );
        }
        let mut controls = Controls::default();
        controls.bindings.actions.insert(
            controls::JUMP.into(),
            vec![quasar_input::Binding::gamepad_button(GamepadButton::West)],
        );
        pads.set_bindings(controls.bindings).unwrap();
        pads.hub.button(devices[2], GamepadButton::West, true);
        for slot in 0..4 {
            let pressed = pads
                .hub
                .player(PlayerId::new(slot))
                .unwrap()
                .input
                .action(controls::JUMP)
                .pressed;
            assert_eq!(pressed, slot == 2);
        }
        pads.consume_edges();
        assert!(
            !pads
                .hub
                .player(PlayerId::new(2))
                .unwrap()
                .input
                .action(controls::JUMP)
                .pressed
        );
        pads.release_all();
        assert!(
            !pads
                .hub
                .player(PlayerId::new(2))
                .unwrap()
                .input
                .action(controls::JUMP)
                .held()
        );
    }

    #[test]
    fn imported_player_one_movement_ignores_player_two_arrow_keys() {
        let (mut keyboard, gamepad) = states();
        keyboard.key(Key::ArrowLeft, true);
        let mut nav = StickNav::default();
        let input = build_input(&keyboard, &gamepad, &mut nav);
        assert_eq!(input.move_x, 0);
        assert_eq!(keyboard.action(controls::P2_HORIZONTAL).value, -1.0);
        keyboard.key(Key::KeyD, true);
        assert_eq!(build_input(&keyboard, &gamepad, &mut nav).move_x, 1);
    }

    #[test]
    fn opposite_gamepad_axes_do_not_navigate_twice() {
        let (keyboard, mut gamepad) = states();
        gamepad.gamepad_axis(GamepadAxis::LeftStickY, -1.0);
        let mut stick = StickNav::default();
        let input = build_input(&keyboard, &gamepad, &mut stick);
        assert!(input.menu_up);
        assert_eq!(input.p1, -1);
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
    fn catalog_stick_and_host_commands_follow_the_remapped_actions() {
        let mut controls = Controls::default();
        controls
            .set_binding(
                controls::MENU_HORIZONTAL,
                0,
                quasar_input::Binding::gamepad_axis(GamepadAxis::RightStickX),
            )
            .unwrap();
        controls
            .set_binding(
                controls::FULLSCREEN,
                0,
                quasar_input::Binding::gamepad_button(GamepadButton::North),
            )
            .unwrap();
        let mut keyboard = InputState::default();
        let mut gamepad = InputState::default();
        keyboard.set_bindings(controls.bindings.clone()).unwrap();
        gamepad.set_bindings(controls.bindings).unwrap();
        let mut stick = StickNav::default();
        gamepad.gamepad_axis(GamepadAxis::LeftStickX, 1.0);
        assert!(!build_input(&keyboard, &gamepad, &mut stick).menu_right);
        gamepad.gamepad_axis(GamepadAxis::RightStickX, 1.0);
        gamepad.gamepad_button(GamepadButton::North, true);
        let input = build_input(&keyboard, &gamepad, &mut stick);
        assert!(input.menu_right && input.fullscreen);
    }

    #[test]
    fn gamepad_capture_requires_neutral_and_ignores_stick_jitter() {
        let mut gamepad = InputState::default();
        let mut armed = [false; 6];
        gamepad.gamepad_axis(GamepadAxis::LeftStickX, 1.0);
        assert!(gamepad_capture(&gamepad, &mut armed, false).is_empty());
        assert!(gamepad_capture(&gamepad, &mut armed, true).is_empty());
        gamepad.gamepad_axis(GamepadAxis::LeftStickX, 0.1);
        assert!(gamepad_capture(&gamepad, &mut armed, true).is_empty());
        gamepad.gamepad_axis(GamepadAxis::LeftStickX, -0.9);
        assert!(matches!(
            gamepad_capture(&gamepad, &mut armed, true).as_slice(),
            [CaptureEvent::Axis {
                axis: GamepadAxis::LeftStickX,
                ..
            }]
        ));
        assert!(gamepad_capture(&gamepad, &mut armed, true).is_empty());
    }

    #[test]
    fn shared_editor_updates_inputs_for_different_games_and_persists_the_preset() {
        use classicclassics_collection::{GameKind, Records};
        let mut session = Session::new(Records::default(), Settings::default());
        session.launch(GameKind::Pong);
        let before = session.state_json()["game"].clone();
        session.open_controls();
        session.tick(Input {
            confirm: true,
            ..Default::default()
        });
        assert!(session.capturing_controls());
        assert!(!session.tick(Input {
            quit: true,
            fire: true,
            fire_pressed: true,
            ..Default::default()
        }));
        assert_eq!(session.state_json()["game"], before);
        session.capture_control(CaptureEvent::Button {
            button: Button::Key(Key::KeyF),
            pressed: true,
            modifiers: Vec::new(),
        });
        assert!(!session.capturing_controls());
        assert_eq!(session.controls.hints().fire, "F");
        session.tick(Input {
            back: true,
            ..Default::default()
        });
        session.tick(Input {
            settings: true,
            ..Default::default()
        });
        assert!(!session.settings_open());
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("controls.toml");
        session.controls.save(&path).unwrap();
        let controls = Controls::load(&path).unwrap();
        let mut keyboard = InputState::default();
        let gamepad = InputState::default();
        keyboard.set_bindings(controls.bindings).unwrap();
        keyboard.key(Key::KeyF, true);
        let input = build_input(&keyboard, &gamepad, &mut StickNav::default());
        assert!(input.fire && input.fire_pressed);
        session.tick(input);
        assert_eq!(session.state_json()["game"]["state"]["phase"], "Playing");
        session.launch(GameKind::Asteroids);
        session.tick(input);
        assert!(
            !session.state_json()["game"]["state"]["bullets"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn space_jumps_and_left_mouse_attacks_independently() {
        let (mut keyboard, gamepad) = states();
        let mut stick = StickNav::default();
        keyboard.key(Key::Space, true);
        let input = build_input(&keyboard, &gamepad, &mut stick);
        assert!(input.jump && input.jump_pressed);
        assert!(!input.fire && !input.fire_pressed);
        keyboard.key(Key::Space, false);
        keyboard.consume_edges();
        keyboard.mouse_button(quasar_input::MouseButton::Left, true);
        let input = build_input(&keyboard, &gamepad, &mut stick);
        assert!(input.fire && input.fire_pressed);
        assert!(!input.jump && !input.jump_pressed);
    }

    #[test]
    fn keyboard_edges_and_held_fire_are_separate() {
        let (mut keyboard, gamepad) = states();
        keyboard.key(Key::KeyW, true);
        keyboard.mouse_button(quasar_input::MouseButton::Left, true);
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
