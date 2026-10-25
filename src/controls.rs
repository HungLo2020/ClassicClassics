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
pub const P1: &str = "p1";
pub const P2: &str = "p2";
pub const ROTATE: &str = "rotate";
pub const THRUST: &str = "thrust";
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

pub const ACTIONS: [&str; 21] = [
    MENU_UP, MENU_DOWN, MENU_LEFT, MENU_RIGHT, CONFIRM, BACK, SETTINGS, P1, P2, ROTATE, THRUST,
    FIRE, HYPERSPACE, PAUSE, RESTART, MODE, MUTE, QUIT, FULLSCREEN, FILTER, SCALE,
];

/// Presentation labels derived from the first binding of each action.
#[derive(Clone, Debug)]
pub struct ControlHints {
    pub confirm: String,
    pub back: String,
    pub settings: String,
    pub menu_left: String,
    pub p1: String,
    pub p2: String,
    pub rotate: String,
    pub thrust: String,
    pub fire: String,
    pub hyperspace: String,
    pub pause: String,
    pub restart: String,
    pub mode: String,
    pub mute: String,
    pub fullscreen: String,
    pub filter: String,
    pub scale: String,
}

impl Default for ControlHints {
    fn default() -> Self {
        Self {
            confirm: "ENTER".into(),
            back: "BACKSPACE".into(),
            settings: "ESC".into(),
            menu_left: "A/LEFT".into(),
            p1: "W/S".into(),
            p2: "UP/DOWN".into(),
            rotate: "A/D".into(),
            thrust: "W".into(),
            fire: "SPACE".into(),
            hyperspace: "SHIFT".into(),
            pause: "P".into(),
            restart: "R".into(),
            mode: "TAB".into(),
            mute: "M".into(),
            fullscreen: "F11".into(),
            filter: "F2".into(),
            scale: "F3".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Controls {
    pub version: u32,
    pub bindings: Bindings,
}

impl Default for Controls {
    fn default() -> Self {
        let mut bindings = Bindings::default();
        bindings
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
            .bind(
                P1,
                Binding::gamepad_axis_scaled(GamepadAxis::LeftStickY, -1.0),
            )
            .bind(P2, Binding::axis(Key::ArrowUp, Key::ArrowDown))
            .bind(
                P2,
                Binding::gamepad_axis_scaled(GamepadAxis::RightStickY, -1.0),
            )
            .bind(ROTATE, Binding::axis(Key::KeyA, Key::KeyD))
            .bind(ROTATE, Binding::axis(Key::ArrowLeft, Key::ArrowRight))
            .bind(ROTATE, Binding::gamepad_axis(GamepadAxis::LeftStickX))
            .bind(THRUST, Binding::key(Key::KeyW))
            .bind(THRUST, Binding::key(Key::ArrowUp))
            .bind(THRUST, Binding::gamepad_button(GamepadButton::LeftTrigger))
            .bind(THRUST, Binding::gamepad_button(GamepadButton::RightTrigger))
            .bind(FIRE, Binding::key(Key::Space))
            .bind(FIRE, Binding::gamepad_button(GamepadButton::South))
            .bind(FIRE, Binding::gamepad_button(GamepadButton::RightTrigger))
            .bind(HYPERSPACE, Binding::key(Key::ShiftLeft))
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
            .bind(SCALE, Binding::key(Key::F3));
        Self {
            version: 1,
            bindings,
        }
    }
}

impl Controls {
    pub fn validate(&self) -> Result {
        if self.version != 1 {
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
        let controls: Self =
            quasar_storage::read_json(path)?.ok_or("controls file does not exist")?;
        controls.validate()?;
        Ok(controls)
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
            p2: label(P2),
            rotate: label(ROTATE),
            thrust: label(THRUST),
            fire: label(FIRE),
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

fn binding_label(binding: &Binding) -> String {
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
    fn missing_unknown_and_unsupported_actions_are_rejected() {
        let mut controls = Controls {
            version: 2,
            ..Default::default()
        };
        assert!(controls.validate().is_err());
        controls.version = 1;
        controls.bindings.actions.remove(FIRE);
        assert!(controls.validate().is_err());
        controls.bindings.actions.insert(FIRE.into(), vec![]);
        controls.bindings.actions.insert("typo".into(), vec![]);
        assert!(controls.validate().is_err());
    }
}
