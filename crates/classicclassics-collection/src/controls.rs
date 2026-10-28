//! Action names and defaults owned by Classic Classics. Quasar owns devices,
//! bindings, edge detection, deadzones and controller hotplug.

use crate::Result;
use quasar_input::{Binding, Bindings, Button, GamepadAxis, GamepadButton, Key, MouseButton};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const MENU_UP: &str = "menu_up";
pub const MENU_DOWN: &str = "menu_down";
pub const MENU_LEFT: &str = "menu_left";
pub const MENU_RIGHT: &str = "menu_right";
pub const CONFIRM: &str = "confirm";
pub const BACK: &str = "back";
pub const SETTINGS: &str = "settings";
pub const MOVE_HORIZONTAL: &str = "move_horizontal";
pub const MOVE_VERTICAL: &str = "move_vertical";
pub const P1: &str = "p1";
pub const P2: &str = "p2";
pub const ROTATE: &str = "rotate";
pub const THRUST: &str = "thrust";
pub const JUMP: &str = "jump";
pub const P2_HORIZONTAL: &str = "p2_horizontal";
pub const P2_FIRE: &str = "p2_fire";
pub const P2_JUMP: &str = "p2_jump";
pub const P2_SECONDARY: &str = "p2_secondary";
pub const FIRE: &str = "fire";
pub const HYPERSPACE: &str = "hyperspace";
pub const PAUSE: &str = "pause";
pub const RESTART: &str = "restart";
pub const MODE: &str = "mode";
pub const MUTE: &str = "mute";
pub const QUIT: &str = "quit";
pub const FULLSCREEN: &str = "fullscreen";
pub const FILTER: &str = "filter";
pub const SCALE: &str = "scale";

pub const MENU_HORIZONTAL: &str = "menu_horizontal";
pub const MENU_VERTICAL: &str = "menu_vertical";

pub const ACTIONS: [&str; 30] = [
    MENU_UP,
    MENU_DOWN,
    MENU_LEFT,
    MENU_RIGHT,
    CONFIRM,
    BACK,
    SETTINGS,
    P1,
    P2,
    ROTATE,
    THRUST,
    FIRE,
    JUMP,
    MOVE_HORIZONTAL,
    MOVE_VERTICAL,
    P2_HORIZONTAL,
    P2_FIRE,
    P2_JUMP,
    P2_SECONDARY,
    HYPERSPACE,
    PAUSE,
    RESTART,
    MODE,
    MUTE,
    QUIT,
    FULLSCREEN,
    FILTER,
    SCALE,
    MENU_HORIZONTAL,
    MENU_VERTICAL,
];

pub use classicclassics_core::ControlHints;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Controls {
    pub version: u32,
    pub bindings: Bindings,
}

impl Default for Controls {
    fn default() -> Self {
        let mut bindings = Bindings::default();
        bindings
            .bind(MOVE_HORIZONTAL, Binding::axis(Key::KeyA, Key::KeyD))
            .bind(MOVE_VERTICAL, Binding::axis(Key::KeyW, Key::KeyS))
            .bind(
                MOVE_HORIZONTAL,
                Binding::gamepad_axis(GamepadAxis::LeftStickX),
            )
            .bind(
                MOVE_VERTICAL,
                Binding::gamepad_axis(GamepadAxis::LeftStickY),
            )
            .bind(
                MOVE_HORIZONTAL,
                Binding::Axis {
                    negative: Button::Gamepad(GamepadButton::DPadLeft),
                    positive: Button::Gamepad(GamepadButton::DPadRight),
                },
            )
            .bind(
                MOVE_VERTICAL,
                Binding::Axis {
                    negative: Button::Gamepad(GamepadButton::DPadUp),
                    positive: Button::Gamepad(GamepadButton::DPadDown),
                },
            )
            .bind(JUMP, Binding::key(Key::Space))
            .bind(
                JUMP,
                Binding::Button {
                    button: Button::Mouse(MouseButton::Middle),
                },
            )
            .bind(JUMP, Binding::gamepad_button(GamepadButton::North))
            .bind(
                P2_HORIZONTAL,
                Binding::axis(Key::ArrowLeft, Key::ArrowRight),
            )
            .bind(P2_FIRE, Binding::key(Key::Numpad0))
            .bind(P2_JUMP, Binding::key(Key::Numpad1))
            .bind(P2_SECONDARY, Binding::key(Key::Numpad2))
            .bind(MENU_UP, Binding::key(Key::KeyW))
            .bind(MENU_UP, Binding::key(Key::ArrowUp))
            .bind(MENU_UP, Binding::gamepad_button(GamepadButton::DPadUp))
            .bind(MENU_DOWN, Binding::key(Key::KeyS))
            .bind(MENU_DOWN, Binding::key(Key::ArrowDown))
            .bind(MENU_DOWN, Binding::gamepad_button(GamepadButton::DPadDown))
            .bind(MENU_LEFT, Binding::key(Key::KeyA))
            .bind(MENU_LEFT, Binding::key(Key::ArrowLeft))
            .bind(MENU_LEFT, Binding::gamepad_button(GamepadButton::DPadLeft))
            .bind(MENU_RIGHT, Binding::key(Key::KeyD))
            .bind(MENU_RIGHT, Binding::key(Key::ArrowRight))
            .bind(
                MENU_RIGHT,
                Binding::gamepad_button(GamepadButton::DPadRight),
            )
            .bind(CONFIRM, Binding::key(Key::Enter))
            .bind(CONFIRM, Binding::key(Key::NumpadEnter))
            .bind(CONFIRM, Binding::key(Key::Space))
            .bind(CONFIRM, Binding::gamepad_button(GamepadButton::South))
            .bind(BACK, Binding::key(Key::Backspace))
            .bind(BACK, Binding::gamepad_button(GamepadButton::Select))
            .bind(SETTINGS, Binding::key(Key::Escape))
            .bind(SETTINGS, Binding::gamepad_button(GamepadButton::East))
            .bind(P1, Binding::axis(Key::KeyW, Key::KeyS))
            .bind(P1, Binding::gamepad_axis(GamepadAxis::LeftStickY))
            .bind(P2, Binding::axis(Key::ArrowUp, Key::ArrowDown))
            .bind(P2, Binding::gamepad_axis(GamepadAxis::RightStickY))
            .bind(ROTATE, Binding::axis(Key::KeyA, Key::KeyD))
            .bind(ROTATE, Binding::axis(Key::ArrowLeft, Key::ArrowRight))
            .bind(ROTATE, Binding::gamepad_axis(GamepadAxis::LeftStickX))
            .bind(THRUST, Binding::key(Key::KeyW))
            .bind(THRUST, Binding::key(Key::ArrowUp))
            .bind(THRUST, Binding::gamepad_button(GamepadButton::LeftTrigger))
            .bind(THRUST, Binding::gamepad_button(GamepadButton::RightTrigger))
            .bind(
                FIRE,
                Binding::Button {
                    button: Button::Mouse(MouseButton::Left),
                },
            )
            .bind(FIRE, Binding::gamepad_button(GamepadButton::South))
            .bind(FIRE, Binding::gamepad_button(GamepadButton::RightTrigger))
            .bind(HYPERSPACE, Binding::key(Key::ShiftLeft))
            .bind(
                HYPERSPACE,
                Binding::Button {
                    button: Button::Mouse(MouseButton::Right),
                },
            )
            .bind(HYPERSPACE, Binding::key(Key::ShiftRight))
            .bind(HYPERSPACE, Binding::gamepad_button(GamepadButton::West))
            .bind(PAUSE, Binding::key(Key::KeyP))
            .bind(PAUSE, Binding::gamepad_button(GamepadButton::Start))
            .bind(RESTART, Binding::key(Key::KeyR))
            .bind(RESTART, Binding::gamepad_button(GamepadButton::LeftBumper))
            .bind(MODE, Binding::key(Key::Tab))
            .bind(MODE, Binding::gamepad_button(GamepadButton::North))
            .bind(MUTE, Binding::key(Key::KeyM))
            .bind(QUIT, Binding::key(Key::KeyQ))
            .bind(FULLSCREEN, Binding::key(Key::F11))
            .bind(FULLSCREEN, Binding::chord([Key::AltLeft, Key::Enter]))
            .bind(FULLSCREEN, Binding::chord([Key::AltRight, Key::Enter]))
            .bind(FILTER, Binding::key(Key::F2))
            .bind(SCALE, Binding::key(Key::F3))
            .bind(
                MENU_HORIZONTAL,
                Binding::gamepad_axis(GamepadAxis::LeftStickX),
            )
            .bind(
                MENU_VERTICAL,
                Binding::gamepad_axis(GamepadAxis::LeftStickY),
            );
        Self {
            version: 2,
            bindings,
        }
    }
}

impl Controls {
    pub fn validate(&self) -> Result {
        if self.version != 2 {
            return Err("unsupported controls version".into());
        }
        self.bindings.validate()?;
        for action in ACTIONS {
            if !self.bindings.actions.contains_key(action) {
                return Err(format!(
                    "missing action '{action}'; use an empty list to disable it explicitly"
                )
                .into());
            }
        }
        for action in self.bindings.actions.keys() {
            if !ACTIONS.contains(&action.as_str()) {
                return Err(format!("unknown Classic Classics action '{action}'").into());
            }
        }
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let mut controls: Self = quasar_config::load_file_typed(path)?;
        // Upgrade only the shipped keyboard/mouse defaults; retain custom remaps
        // and controller assignments, including explicitly disabled actions.
        if controls.version == 1 {
            for (action, key, mouse) in [
                (FIRE, Key::Space, MouseButton::Left),
                (JUMP, Key::KeyZ, MouseButton::Middle),
            ] {
                if let Some(values) = controls.bindings.actions.get_mut(action) {
                    let old: Vec<_> = values
                        .iter()
                        .filter(|value| keyboard_mouse(value))
                        .cloned()
                        .collect();
                    let legacy_key = Binding::key(key);
                    let legacy_mouse = Binding::Button {
                        button: Button::Mouse(mouse),
                    };
                    if old == [legacy_key.clone()] || old == [legacy_key, legacy_mouse] {
                        let mut updated: Vec<_> = Self::default().bindings.actions[action]
                            .iter()
                            .filter(|value| keyboard_mouse(value))
                            .cloned()
                            .collect();
                        updated.extend(
                            values
                                .iter()
                                .filter(|value| !keyboard_mouse(value))
                                .cloned(),
                        );
                        *values = updated;
                    }
                }
            }
            controls.version = 2;
        }
        // Older presets predate configurable catalog stick axes.
        let defaults = Self::default();
        for name in [
            MENU_HORIZONTAL,
            MENU_VERTICAL,
            MOVE_HORIZONTAL,
            MOVE_VERTICAL,
            JUMP,
            P2_HORIZONTAL,
            P2_FIRE,
            P2_JUMP,
            P2_SECONDARY,
        ] {
            controls
                .bindings
                .actions
                .entry(name.into())
                .or_insert_with(|| defaults.bindings.actions[name].clone());
        }
        controls.validate()?;
        Ok(controls)
    }

    pub fn load_saved(path: &Path) -> (Self, bool) {
        let load = || -> Result<(Self, bool)> {
            let Some(source) = crate::config_file::source(path)? else {
                return Ok((Self::default(), true));
            };
            Ok((Self::load(&source)?, true))
        };
        match load() {
            Ok(result) => result,
            Err(error) => {
                eprintln!("Cannot load controls ({error}); preserving file and using defaults");
                (Self::default(), false)
            }
        }
    }

    pub fn save(&self, path: &Path) -> Result {
        self.validate()?;
        crate::config_file::write(path, self)
    }

    /// Edit one alternative while retaining all other keys, buttons and axes.
    pub fn set_binding(&mut self, action: &str, slot: usize, binding: Binding) -> Result {
        let mut next = self.clone();
        let values = next
            .bindings
            .actions
            .get_mut(action)
            .ok_or("unknown action")?;
        if slot > values.len() {
            return Err("invalid binding slot".into());
        }
        if values
            .iter()
            .enumerate()
            .any(|(index, existing)| index != slot && *existing == binding)
        {
            return Err("that binding is already assigned to this action".into());
        }
        if slot == values.len() {
            values.push(binding);
        } else {
            values[slot] = binding;
        }
        next.validate()?;
        *self = next;
        Ok(())
    }

    pub fn remove_binding(&mut self, action: &str, slot: usize) {
        if let Some(values) = self.bindings.actions.get_mut(action)
            && slot < values.len()
        {
            values.remove(slot);
        }
    }

    pub fn restore_action(&mut self, action: &str) {
        if let Some(values) = Self::default().bindings.actions.get(action) {
            self.bindings.actions.insert(action.into(), values.clone());
        }
    }

    pub fn hints(&self) -> ControlHints {
        let label = |name: &str| {
            self.bindings
                .actions
                .get(name)
                .and_then(|bindings| bindings.first())
                .map(binding_label)
                .unwrap_or_else(|| "UNBOUND".into())
        };
        ControlHints {
            confirm: label(CONFIRM),
            back: label(BACK),
            settings: label(SETTINGS),
            menu_left: label(MENU_LEFT),
            p1: label(P1),
            move_x: label(MOVE_HORIZONTAL),
            move_y: label(MOVE_VERTICAL),
            p2: label(P2),
            rotate: label(ROTATE),
            thrust: label(THRUST),
            fire: label(FIRE),
            jump: label(JUMP),
            hyperspace: label(HYPERSPACE),
            pause: label(PAUSE),
            restart: label(RESTART),
            mode: label(MODE),
            mute: label(MUTE),
            fullscreen: label(FULLSCREEN),
            filter: label(FILTER),
            scale: label(SCALE),
        }
    }
}

pub fn binding_label(binding: &Binding) -> String {
    match binding {
        Binding::Button { button } => button_label(*button),
        Binding::Axis { negative, positive } => {
            format!("{}/{}", button_label(*negative), button_label(*positive))
        }
        Binding::Chord { buttons } => buttons
            .iter()
            .map(|button| button_label(*button))
            .collect::<Vec<_>>()
            .join("+"),
        Binding::GamepadAxis { axis, scale } => {
            let label = format!("{axis:?}")
                .to_uppercase()
                .replace("STICK", "STICK ");
            if (*scale - 1.0).abs() < f32::EPSILON {
                label
            } else {
                format!("{label}*{scale}")
            }
        }
    }
}

fn keyboard_mouse(binding: &Binding) -> bool {
    let matches = |button: &Button| matches!(button, Button::Key(_) | Button::Mouse(_));
    match binding {
        Binding::Button { button } => matches(button),
        Binding::Axis { negative, positive } => matches(negative) || matches(positive),
        Binding::Chord { buttons } => buttons.iter().any(matches),
        Binding::GamepadAxis { .. } => false,
    }
}

fn button_label(button: Button) -> String {
    match button {
        Button::Key(key) => key_label(key),
        Button::Mouse(MouseButton::Left) => "MOUSE1".into(),
        Button::Mouse(MouseButton::Right) => "MOUSE2".into(),
        Button::Mouse(MouseButton::Middle) => "MOUSE3".into(),
        Button::Mouse(other) => format!("MOUSE {other:?}").to_uppercase(),
        Button::Gamepad(button) => gamepad_button_label(button),
    }
}

fn key_label(key: Key) -> String {
    match key {
        Key::ArrowUp => "UP".into(),
        Key::ArrowDown => "DOWN".into(),
        Key::ArrowLeft => "LEFT".into(),
        Key::ArrowRight => "RIGHT".into(),
        Key::AltLeft => "LALT".into(),
        Key::AltRight => "RALT".into(),
        Key::ControlLeft => "LCTRL".into(),
        Key::ControlRight => "RCTRL".into(),
        Key::ShiftLeft => "LSHIFT".into(),
        Key::ShiftRight => "RSHIFT".into(),
        other => {
            let label = format!("{other:?}");
            label
                .strip_prefix("Key")
                .or_else(|| label.strip_prefix("Digit"))
                .unwrap_or(&label)
                .to_uppercase()
        }
    }
}

fn gamepad_button_label(button: GamepadButton) -> String {
    match button {
        GamepadButton::South => "PAD SOUTH".into(),
        GamepadButton::East => "PAD EAST".into(),
        GamepadButton::North => "PAD NORTH".into(),
        GamepadButton::West => "PAD WEST".into(),
        GamepadButton::LeftBumper => "LB".into(),
        GamepadButton::RightBumper => "RB".into(),
        GamepadButton::LeftTrigger => "LT".into(),
        GamepadButton::RightTrigger => "RT".into(),
        GamepadButton::LeftStick => "LS".into(),
        GamepadButton::RightStick => "RS".into(),
        GamepadButton::DPadUp => "DPAD UP".into(),
        GamepadButton::DPadDown => "DPAD DOWN".into(),
        GamepadButton::DPadLeft => "DPAD LEFT".into(),
        GamepadButton::DPadRight => "DPAD RIGHT".into(),
        GamepadButton::Select => "SELECT".into(),
        GamepadButton::Start => "START".into(),
        GamepadButton::Mode => "MODE".into(),
        GamepadButton::C => "C".into(),
        GamepadButton::Z => "Z".into(),
        GamepadButton::Unknown => "PAD ?".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_complete_and_valid() {
        let controls = Controls::default();
        controls.validate().unwrap();
        assert_eq!(controls.bindings.actions.len(), ACTIONS.len());
    }

    #[test]
    fn toml_preserves_bindings_and_migrates_json_without_overwriting_it() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("controls.toml");
        let legacy = path.with_extension("json");
        let mut controls = Controls::default();
        controls
            .bindings
            .actions
            .insert(FIRE.into(), vec![Binding::key(Key::KeyF)]);
        controls.save(&legacy).unwrap();
        let old = std::fs::read(&legacy).unwrap();
        let (loaded, migrate) = Controls::load_saved(&path);
        assert!(migrate);
        assert_eq!(loaded.bindings, controls.bindings);
        loaded.save(&path).unwrap();
        assert_eq!(Controls::load(&path).unwrap().bindings, controls.bindings);
        assert!(Controls::load_saved(&path).1);
        assert_eq!(std::fs::read(&legacy).unwrap(), old);
        std::fs::write(&path, "version = 99").unwrap();
        assert!(!Controls::load_saved(&path).1);
        assert_eq!(std::fs::read_to_string(path).unwrap(), "version = 99");
    }

    #[test]
    fn old_presets_gain_menu_axes_but_explicitly_disabled_axes_stay_disabled() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("controls.toml");
        let mut old = Controls::default();
        old.bindings.actions.remove(MENU_HORIZONTAL);
        old.bindings
            .actions
            .insert(MENU_VERTICAL.into(), Vec::new());
        crate::config_file::write(&path, &old).unwrap();
        let migrated = Controls::load(&path).unwrap();
        assert_eq!(
            migrated.bindings.actions[MENU_HORIZONTAL],
            Controls::default().bindings.actions[MENU_HORIZONTAL]
        );
        assert!(migrated.bindings.actions[MENU_VERTICAL].is_empty());
    }

    #[test]
    fn version_one_defaults_upgrade_without_changing_controller_remaps() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("controls.toml");
        let mut old = Controls {
            version: 1,
            ..Default::default()
        };
        for (action, key, mouse) in [
            (FIRE, Key::Space, MouseButton::Left),
            (JUMP, Key::KeyZ, MouseButton::Middle),
        ] {
            old.bindings.actions.insert(
                action.into(),
                vec![
                    Binding::key(key),
                    Binding::Button {
                        button: Button::Mouse(mouse),
                    },
                    Binding::gamepad_button(GamepadButton::West),
                ],
            );
        }
        crate::config_file::write(&path, &old).unwrap();
        let loaded = Controls::load(&path).unwrap();
        assert_eq!(loaded.version, 2);
        assert_eq!(loaded.hints().jump, "SPACE");
        assert_eq!(loaded.hints().fire, "MOUSE1");
        for action in [FIRE, JUMP] {
            assert_eq!(
                loaded.bindings.actions[action].last(),
                Some(&Binding::gamepad_button(GamepadButton::West))
            );
        }
        assert_eq!(Controls::load(&path).unwrap(), loaded);
        assert_eq!(
            quasar_config::load_file_typed::<Controls>(&path).unwrap(),
            old
        );
    }

    #[test]
    fn version_one_custom_and_disabled_controls_are_preserved() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("controls.toml");
        let mut old = Controls {
            version: 1,
            ..Default::default()
        };
        old.bindings
            .actions
            .insert(FIRE.into(), vec![Binding::key(Key::KeyF)]);
        old.bindings.actions.insert(JUMP.into(), vec![]);
        crate::config_file::write(&path, &old).unwrap();
        let loaded = Controls::load(&path).unwrap();
        assert_eq!(loaded.bindings, old.bindings);
    }

    #[test]
    fn missing_unknown_and_unsupported_actions_are_rejected() {
        let mut controls = Controls {
            version: 99,
            ..Default::default()
        };
        assert!(controls.validate().is_err());
        controls.version = 2;
        controls.bindings.actions.remove(FIRE);
        assert!(controls.validate().is_err());
        controls.bindings.actions.insert(FIRE.into(), vec![]);
        controls.bindings.actions.insert("typo".into(), vec![]);
        assert!(controls.validate().is_err());
    }
}
