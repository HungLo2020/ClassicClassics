//! Shared controls table. Quasar owns widgets, hit testing, focus and capture isolation.
use crate::{
    Input,
    controls::{self, ACTIONS, Controls},
    paint::{CYAN, DIM, MUTED, Paint, WHITE},
};
use quasar_input::{Binding, Button, GamepadAxis, GamepadButton, Key, MouseButton};
use quasar_render_2d::{Canvas, Transform2D};
use quasar_ui::{
    ActionId, BitmapTextMeasure, Edges, Layer, LayoutItem, Length, NodeId, PanelLayout, Rect,
    TextStyle, Theme, Ui, UiEvent, UiInput, UiRenderer, UiText, Vec2, Widget,
};

const WIDTH: f32 = 960.0;
const HEIGHT: f32 = 600.0;
const TABLE_TOP: f32 = 70.0;
const ROW_HEIGHT: f32 = 31.0;
const GAMEPLAY: &[&str] = &[
    controls::FIRE,
    controls::THRUST,
    controls::ROTATE,
    controls::P1,
    controls::P2,
    controls::HYPERSPACE,
    controls::PAUSE,
    controls::RESTART,
    controls::MODE,
];
const MENUS: &[&str] = &[
    controls::MENU_UP,
    controls::MENU_DOWN,
    controls::MENU_LEFT,
    controls::MENU_RIGHT,
    controls::CONFIRM,
    controls::BACK,
    controls::SETTINGS,
    controls::MENU_HORIZONTAL,
    controls::MENU_VERTICAL,
];
const SYSTEM: &[&str] = &[
    controls::FULLSCREEN,
    controls::FILTER,
    controls::SCALE,
    controls::MUTE,
    controls::QUIT,
];
const GROUPS: [(&str, &[&str]); 3] = [("GAMEPLAY", GAMEPLAY), ("MENUS", MENUS), ("SYSTEM", SYSTEM)];

pub fn axis_directions(action: &str) -> Option<(&'static str, &'static str)> {
    match action {
        controls::P1 | controls::P2 | controls::MENU_VERTICAL => Some(("UP", "DOWN")),
        controls::ROTATE | controls::MENU_HORIZONTAL => Some(("LEFT", "RIGHT")),
        _ => None,
    }
}

pub fn action_label(action: &str) -> String {
    match action {
        controls::FIRE => "FIRE / SERVE",
        controls::ROTATE => "TURN LEFT / RIGHT",
        controls::P1 => "PADDLE 1 UP / DOWN",
        controls::P2 => "PADDLE 2 UP / DOWN",
        controls::MODE => "CHANGE GAME MODE",
        controls::CONFIRM => "SELECT / CONFIRM",
        controls::BACK => "BACK TO CATALOG",
        controls::SETTINGS => "OPEN SETTINGS",
        controls::MENU_HORIZONTAL => "MENU STICK LEFT/RIGHT",
        controls::MENU_VERTICAL => "MENU STICK UP/DOWN",
        controls::FILTER => "CHANGE FILTERING",
        controls::SCALE => "CHANGE SCALING",
        _ => return action.replace('_', " ").to_uppercase(),
    }
    .into()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Device {
    Keyboard,
    Controller,
}
impl Device {
    const ALL: [Self; 2] = [Self::Keyboard, Self::Controller];
    fn name(self) -> &'static str {
        if self == Self::Keyboard {
            "KEYBOARD / MOUSE"
        } else {
            "CONTROLLER"
        }
    }
    fn matches(self, binding: &Binding) -> bool {
        let gamepad = |button: &Button| matches!(button, Button::Gamepad(_));
        let controller = match binding {
            Binding::Button { button } => gamepad(button),
            Binding::Axis { negative, positive } => gamepad(negative) || gamepad(positive),
            Binding::Chord { buttons } => buttons.iter().any(gamepad),
            Binding::GamepadAxis { .. } => true,
        };
        controller == (self == Self::Controller)
    }
    fn accepts(self, button: Button) -> bool {
        matches!(button, Button::Gamepad(_)) == (self == Self::Controller)
    }
}

fn button_name(button: Button) -> String {
    match button {
        Button::Mouse(MouseButton::Left) => "LEFT MOUSE".into(),
        Button::Mouse(MouseButton::Right) => "RIGHT MOUSE".into(),
        Button::Mouse(MouseButton::Middle) => "MIDDLE MOUSE".into(),
        Button::Key(Key::Enter | Key::NumpadEnter) => "ENTER".into(),
        Button::Key(Key::AltLeft | Key::AltRight) => "ALT".into(),
        Button::Key(Key::ControlLeft | Key::ControlRight) => "CTRL".into(),
        Button::Key(Key::ShiftLeft | Key::ShiftRight) => "SHIFT".into(),
        Button::Gamepad(button) => match button {
            GamepadButton::South => "A / CROSS",
            GamepadButton::East => "B / CIRCLE",
            GamepadButton::West => "X / SQUARE",
            GamepadButton::North => "Y / TRIANGLE",
            GamepadButton::LeftTrigger => "LT",
            GamepadButton::RightTrigger => "RT",
            GamepadButton::LeftBumper => "LB",
            GamepadButton::RightBumper => "RB",
            GamepadButton::DPadUp => "D-PAD UP",
            GamepadButton::DPadDown => "D-PAD DOWN",
            GamepadButton::DPadLeft => "D-PAD LEFT",
            GamepadButton::DPadRight => "D-PAD RIGHT",
            GamepadButton::Select => "SELECT",
            GamepadButton::Start => "START",
            GamepadButton::LeftStick => "LEFT STICK CLICK",
            GamepadButton::RightStick => "RIGHT STICK CLICK",
            _ => return format!("{button:?}").to_uppercase(),
        }
        .into(),
        _ => controls::binding_label(&Binding::Button { button }),
    }
}

fn binding_name(binding: &Binding) -> String {
    match binding {
        Binding::Button { button } => button_name(*button),
        Binding::Axis { negative, positive } => {
            format!("{} / {}", button_name(*negative), button_name(*positive))
        }
        Binding::Chord { buttons } => buttons
            .iter()
            .map(|button| button_name(*button))
            .collect::<Vec<_>>()
            .join("+"),
        Binding::GamepadAxis { axis, scale } => format!(
            "{}{}",
            match axis {
                GamepadAxis::LeftStickX => "LEFT STICK L/R",
                GamepadAxis::LeftStickY => "LEFT STICK U/D",
                GamepadAxis::RightStickX => "RIGHT STICK L/R",
                GamepadAxis::RightStickY => "RIGHT STICK U/D",
                GamepadAxis::LeftTrigger => "LT",
                GamepadAxis::RightTrigger => "RT",
            },
            if *scale < 0.0 { " REVERSED" } else { "" }
        ),
    }
}

fn cell_value(controls: &Controls, action: &str, device: Device) -> String {
    let mut names = Vec::new();
    for binding in controls.bindings.actions[action]
        .iter()
        .filter(|binding| device.matches(binding))
    {
        let name = binding_name(binding);
        if !names.contains(&name) {
            names.push(name);
        }
    }
    if names.is_empty() {
        "NOT SET".into()
    } else {
        names.join(" OR ")
    }
}

/// Replacing a visible cell changes that device's assignments only.
fn replace_cell(
    controls: &mut Controls,
    action: &str,
    device: Device,
    binding: Option<Binding>,
) -> crate::Result {
    let mut next = controls.clone();
    let values = next
        .bindings
        .actions
        .get_mut(action)
        .ok_or("unknown action")?;
    let index = values
        .iter()
        .position(|existing| device.matches(existing))
        .unwrap_or(values.len());
    values.retain(|existing| !device.matches(existing));
    if let Some(binding) = binding {
        values.insert(index.min(values.len()), binding);
    }
    next.validate()?;
    *controls = next;
    Ok(())
}
/// Device events use Quasar's portable identifiers; no native API enters the UI.
#[derive(Clone, Debug)]
pub enum CaptureEvent {
    Button {
        button: Button,
        pressed: bool,
        modifiers: Vec<Button>,
    },
    Axis {
        axis: GamepadAxis,
        value: f32,
    },
    Cancel,
}

pub fn is_modifier(button: Button) -> bool {
    matches!(
        button,
        Button::Key(
            Key::AltLeft
                | Key::AltRight
                | Key::ControlLeft
                | Key::ControlRight
                | Key::ShiftLeft
                | Key::ShiftRight
                | Key::SuperLeft
                | Key::SuperRight
        )
    )
}

struct Capture {
    action: &'static str,
    device: Device,
    negative: Option<Button>,
    modifier: Option<Button>,
}

pub struct ControlsScreen {
    ui: Ui,
    cells: Vec<(NodeId, &'static str, Device)>,
    tabs: Vec<NodeId>,
    reset: Option<NodeId>,
    back: Option<NodeId>,
    group: usize,
    selected: (&'static str, Device),
    capture: Option<Capture>,
    pointer: UiInput,
    message: String,
    selected_value: String,
}

impl ControlsScreen {
    pub fn new(controls: &Controls) -> Self {
        let mut screen = Self {
            ui: Ui::new(Theme::retro()),
            cells: Vec::new(),
            tabs: Vec::new(),
            reset: None,
            back: None,
            group: 0,
            selected: (controls::FIRE, Device::Keyboard),
            capture: None,
            pointer: UiInput::default(),
            message: String::new(),
            selected_value: String::new(),
        };
        screen.rebuild(controls);
        screen
    }
    pub fn capturing(&self) -> bool {
        self.ui.is_capturing()
    }

    /// Coordinates use the same fitted 960x600 design as the rendered menu.
    pub fn pointer_position(canvas: (u32, u32), position: (f64, f64)) -> Vec2 {
        let scale = (canvas.0 as f64 / WIDTH as f64).min(canvas.1 as f64 / HEIGHT as f64);
        Vec2::new(
            ((position.0 - (canvas.0 as f64 - WIDTH as f64 * scale) * 0.5) / scale) as f32,
            ((position.1 - (canvas.1 as f64 - HEIGHT as f64 * scale) * 0.5) / scale) as f32,
        )
    }
    pub fn set_pointer(&mut self, input: UiInput) {
        self.pointer = input;
    }

    fn viewport(width: u32, height: u32) -> (Rect, f32) {
        let scale = (width as f32 / WIDTH).min(height as f32 / HEIGHT);
        (
            Rect::new(
                (width as f32 - WIDTH * scale) * 0.5 + 48.0 * scale,
                (height as f32 - HEIGHT * scale) * 0.5 + 100.0 * scale,
                864.0 * scale,
                440.0 * scale,
            ),
            scale,
        )
    }

    fn add(ui: &mut Ui, root: NodeId, widget: Widget, rect: Rect) -> NodeId {
        ui.add_child(
            root,
            widget,
            LayoutItem::size(Length::Fixed(rect.width), Length::Fixed(rect.height))
                .with_offset(Vec2::new(rect.x, rect.y)),
        )
        .expect("valid controls widget")
    }

    fn rebuild(&mut self, controls: &Controls) {
        let mut theme = Theme::retro();
        theme.font_size = 14.0;
        theme.padding = 5.0;
        theme.panel = quasar_ui::Color::rgb(3, 5, 10);
        theme.panel_alt = theme.panel;
        theme.button = quasar_ui::Color::rgb(14, 22, 35);
        theme.text = quasar_ui::Color::rgb(232, 243, 245);
        theme.accent = quasar_ui::Color::rgb(91, 235, 214);
        theme.focus = theme.accent;
        let mut ui = Ui::new(theme);
        let root = ui.add_root(
            Widget::Panel {
                layout: PanelLayout::Absolute,
                padding: Edges::ZERO,
                clip: true,
                scrollable: false,
            },
            LayoutItem::size(Length::Fixed(864.0), Length::Fixed(440.0)),
            Layer::Overlay,
            false,
        );
        self.tabs.clear();
        self.cells.clear();
        for (index, (name, _)) in GROUPS.iter().enumerate() {
            let label = name.to_string();
            self.tabs.push(Self::add(
                &mut ui,
                root,
                Widget::Button {
                    text: UiText::literal(label),
                    action: Some(ActionId(10 + index as u64)),
                },
                Rect::new(index as f32 * 176.0, 0.0, 164.0, 30.0),
            ));
        }
        for (x, label) in [
            (0.0, "ACTION"),
            (272.0, "KEYBOARD / MOUSE"),
            (568.0, "CONTROLLER"),
        ] {
            Self::add(
                &mut ui,
                root,
                Widget::Label {
                    text: UiText::literal(label),
                    style: TextStyle::Dim,
                },
                Rect::new(x, 44.0, 264.0, 22.0),
            );
        }
        for (row, &action) in GROUPS[self.group].1.iter().enumerate() {
            let y = TABLE_TOP + row as f32 * ROW_HEIGHT;
            Self::add(
                &mut ui,
                root,
                Widget::Label {
                    text: UiText::literal(action_label(action)),
                    style: TextStyle::Body,
                },
                Rect::new(0.0, y, 264.0, 29.0),
            );
            for (column, device) in Device::ALL.into_iter().enumerate() {
                let value = cell_value(controls, action, device);
                let display = if value.chars().count() > 22 {
                    format!("{}...", value.chars().take(19).collect::<String>())
                } else {
                    value
                };
                let node = Self::add(
                    &mut ui,
                    root,
                    Widget::Binding {
                        label: UiText::literal(""),
                        action: action.into(),
                        value: display,
                    },
                    Rect::new(272.0 + column as f32 * 296.0, y, 280.0, 29.0),
                );
                self.cells.push((node, action, device));
            }
        }
        self.reset = Some(Self::add(
            &mut ui,
            root,
            Widget::Button {
                text: UiText::literal("RESTORE DEFAULTS"),
                action: Some(ActionId(1)),
            },
            Rect::new(0.0, 412.0, 240.0, 28.0),
        ));
        self.back = Some(Self::add(
            &mut ui,
            root,
            Widget::Button {
                text: UiText::literal("BACK"),
                action: Some(ActionId(2)),
            },
            Rect::new(704.0, 412.0, 144.0, 28.0),
        ));
        ui.set_modal(Some(root));
        ui.layout(
            Self::viewport(960, 600).0,
            1.0,
            Edges::ZERO,
            &BitmapTextMeasure,
        );
        ui.focus(
            self.cells
                .iter()
                .find(|(_, action, device)| (*action, *device) == self.selected)
                .map(|(node, _, _)| *node),
        );
        self.ui = ui;
        self.selected_value = cell_value(controls, self.selected.0, self.selected.1);
    }

    fn focus_cell(&mut self, action: &'static str, device: Device) {
        self.selected = (action, device);
        self.ui.focus(
            self.cells
                .iter()
                .find(|(_, name, column)| *name == action && *column == device)
                .map(|(node, _, _)| *node),
        );
    }

    fn navigate(&mut self, input: Input, controls: &Controls) {
        let focus = self.ui.focused();
        if let Some((_, action, device)) = self
            .cells
            .iter()
            .copied()
            .find(|(node, _, _)| Some(*node) == focus)
        {
            self.selected = (action, device);
            let row = GROUPS[self.group]
                .1
                .iter()
                .position(|name| *name == action)
                .unwrap();
            if input.menu_left || input.menu_right {
                self.focus_cell(
                    action,
                    if input.menu_left {
                        Device::Keyboard
                    } else {
                        Device::Controller
                    },
                );
            } else if input.menu_up {
                if row == 0 {
                    self.ui.focus(Some(self.tabs[self.group]));
                } else {
                    self.focus_cell(GROUPS[self.group].1[row - 1], device);
                }
            } else if input.menu_down {
                if row + 1 == GROUPS[self.group].1.len() {
                    self.ui.focus(self.back);
                } else {
                    self.focus_cell(GROUPS[self.group].1[row + 1], device);
                }
            }
        } else if self.tabs.iter().any(|node| Some(*node) == focus) {
            if input.menu_left || input.menu_right {
                self.group = (self.group as isize + if input.menu_left { -1 } else { 1 })
                    .rem_euclid(GROUPS.len() as isize) as usize;
                self.selected.0 = GROUPS[self.group].1[0];
                self.rebuild(controls);
                self.ui.focus(Some(self.tabs[self.group]));
            } else if input.menu_down {
                self.focus_cell(GROUPS[self.group].1[0], self.selected.1);
            }
        } else if focus == self.reset || focus == self.back {
            if input.menu_up {
                self.focus_cell(GROUPS[self.group].1.last().unwrap(), self.selected.1);
            } else if input.menu_left || input.menu_right {
                self.ui.focus(if input.menu_left {
                    self.reset
                } else {
                    self.back
                });
            }
        }
    }

    pub fn tick(&mut self, input: Input, controls: &mut Controls) -> bool {
        if self.capturing() {
            self.pointer = UiInput::default();
            return false;
        }
        if input.settings || input.back {
            return true;
        }
        self.navigate(input, controls);
        let mut pointer = std::mem::take(&mut self.pointer);
        if pointer.delete
            && self
                .cells
                .iter()
                .any(|(node, _, _)| Some(*node) == self.ui.focused())
        {
            let _ = replace_cell(controls, self.selected.0, self.selected.1, None);
            self.message = format!("{} CLEARED", action_label(self.selected.0));
            self.rebuild(controls);
            pointer.delete = false;
        }
        pointer.activate |= input.confirm;
        let frame = self.ui.update(
            &pointer,
            Self::viewport(960, 600).0,
            1.0,
            Edges::ZERO,
            &BitmapTextMeasure,
            1.0 / 120.0,
        );
        for event in frame.events {
            match event {
                UiEvent::FocusChanged(Some(node)) => {
                    if let Some((_, action, device)) =
                        self.cells.iter().find(|(id, _, _)| *id == node)
                    {
                        self.selected = (*action, *device);
                        self.message.clear();
                    }
                }
                UiEvent::RebindRequested { id, .. } => {
                    let (_, action, device) =
                        *self.cells.iter().find(|(node, _, _)| *node == id).unwrap();
                    self.selected = (action, device);
                    self.capture = Some(Capture {
                        action,
                        device,
                        negative: None,
                        modifier: None,
                    });
                    self.message.clear();
                }
                UiEvent::Pressed(ActionId(1)) => {
                    *controls = Controls::default();
                    self.message = "DEFAULT CONTROLS RESTORED".into();
                    self.rebuild(controls);
                }
                UiEvent::Pressed(ActionId(2)) => return true,
                UiEvent::Pressed(ActionId(id @ 10..=12)) => {
                    self.group = (id - 10) as usize;
                    self.selected.0 = GROUPS[self.group].1[0];
                    self.rebuild(controls);
                }
                _ => {}
            }
        }
        self.selected_value = cell_value(controls, self.selected.0, self.selected.1);
        false
    }

    fn prompt(&self) -> String {
        let Some(capture) = &self.capture else {
            return String::new();
        };
        if let Some((negative, positive)) = axis_directions(capture.action) {
            let direction = if capture.negative.is_some() {
                positive
            } else {
                negative
            };
            if capture.device == Device::Keyboard {
                format!("PRESS THE KEY FOR {direction}")
            } else {
                format!("MOVE A STICK {direction}, OR PRESS A BUTTON")
            }
        } else if capture.device == Device::Keyboard {
            "PRESS A NEW KEY OR MOUSE BUTTON".into()
        } else {
            "PRESS A CONTROLLER BUTTON OR TRIGGER".into()
        }
    }

    pub fn capture(&mut self, event: CaptureEvent, controls: &mut Controls) {
        if !self.capturing() {
            return;
        }
        if matches!(event, CaptureEvent::Cancel) {
            self.ui.cancel_rebind();
            self.capture = None;
            self.message = "CHANGE CANCELLED".into();
            self.rebuild(controls);
            return;
        }
        let Some(capture) = &mut self.capture else {
            return;
        };
        let signed = axis_directions(capture.action).is_some();
        let candidate = match event {
            CaptureEvent::Button {
                button,
                pressed,
                mut modifiers,
            } => {
                if !capture.device.accepts(button) {
                    return;
                }
                if is_modifier(button) {
                    if pressed {
                        capture.modifier = Some(button);
                        return;
                    }
                    if capture.modifier != Some(button) {
                        return;
                    }
                    capture.modifier = None;
                } else if !pressed {
                    return;
                }
                if signed {
                    if let Some(negative) = capture.negative {
                        if negative == button {
                            self.message =
                                "CHOOSE A DIFFERENT BUTTON FOR THE OTHER DIRECTION".into();
                            return;
                        }
                        Binding::Axis {
                            negative,
                            positive: button,
                        }
                    } else {
                        capture.negative = Some(button);
                        return;
                    }
                } else {
                    modifiers.retain(|modifier| *modifier != button);
                    if modifiers.is_empty() {
                        Binding::Button { button }
                    } else {
                        modifiers.push(button);
                        Binding::Chord { buttons: modifiers }
                    }
                }
            }
            CaptureEvent::Axis { axis, value } => {
                if capture.device != Device::Controller || !value.is_finite() || value.abs() < 0.65
                {
                    return;
                }
                let trigger = matches!(axis, GamepadAxis::LeftTrigger | GamepadAxis::RightTrigger);
                if signed && !trigger {
                    Binding::gamepad_axis_scaled(axis, -value.signum())
                } else if !signed && trigger {
                    Binding::gamepad_axis(axis)
                } else {
                    self.message = if signed {
                        "USE A STICK OR TWO BUTTONS"
                    } else {
                        "USE A BUTTON OR TRIGGER"
                    }
                    .into();
                    return;
                }
            }
            CaptureEvent::Cancel => unreachable!(),
        };
        let (action, device) = (capture.action, capture.device);
        match replace_cell(controls, action, device, Some(candidate.clone())) {
            Ok(()) => {
                let _ = self.ui.finish_rebind(binding_name(&candidate));
                self.capture = None;
                let shared: Vec<_> = ACTIONS
                    .iter()
                    .filter(|other| {
                        **other != action && controls.bindings.actions[**other].contains(&candidate)
                    })
                    .map(|other| action_label(other))
                    .collect();
                self.message = if shared.is_empty() {
                    format!("{} CHANGED", action_label(action))
                } else {
                    format!("ALSO USED BY: {}", shared.join(", "))
                };
                self.rebuild(controls);
            }
            Err(error) => self.message = error.to_string().to_uppercase(),
        }
    }

    pub fn scene(&mut self, base: Canvas) -> Canvas {
        let (width, height) = (base.width(), base.height());
        let mut paint = Paint::over(base);
        paint.rect(0.0, 0.0, width as f32, height as f32, [3, 5, 10, 255]);
        paint.fit_view(960, 600);
        paint.text(48.0, 30.0, "CONTROLS", 3.0, WHITE);
        paint.text(
            49.0,
            68.0,
            "CLICK A CONTROL TO CHANGE IT. APPLIES TO EVERY GAME.",
            1.0,
            MUTED,
        );
        let mut canvas = paint.into_canvas();
        canvas.set_transform(Transform2D::IDENTITY);
        let (viewport, scale) = Self::viewport(width, height);
        self.ui
            .layout(viewport, scale, Edges::ZERO, &BitmapTextMeasure);
        UiRenderer::new().draw(&mut canvas, &self.ui);
        canvas.set_clip(None);
        let mut paint = Paint::over(canvas);
        paint.fit_view(960, 600);
        paint.rect(48.0 + self.group as f32 * 176.0, 130.0, 164.0, 3.0, CYAN);
        paint.text(
            48.0,
            462.0,
            &if self.message.is_empty() && self.selected_value.chars().count() > 22 {
                &self.selected_value
            } else {
                &self.message
            }
            .chars()
            .take(125)
            .collect::<String>(),
            1.0,
            CYAN,
        );
        paint.text(
            48.0,
            486.0,
            "CLICK OR ARROWS SELECT    ENTER CHANGE    DELETE CLEAR    ESC BACK",
            1.0,
            MUTED,
        );
        paint.text(
            48.0,
            558.0,
            "CHANGES APPLY IMMEDIATELY AND SAVE WHEN YOU QUIT.",
            1.0,
            DIM,
        );
        if let Some(capture) = &self.capture {
            paint.rect(0.0, 0.0, 960.0, 600.0, [0, 0, 0, 210]);
            paint.rect(170.0, 190.0, 620.0, 200.0, [9, 14, 23, 255]);
            paint.outline(170.0, 190.0, 620.0, 200.0, CYAN);
            paint.text(194.0, 212.0, capture.device.name(), 2.0, CYAN);
            paint.text(194.0, 248.0, &action_label(capture.action), 2.0, WHITE);
            paint.text(194.0, 290.0, &self.prompt(), 1.0, WHITE);
            paint.text(
                194.0,
                320.0,
                &self.message.chars().take(88).collect::<String>(),
                1.0,
                MUTED,
            );
            paint.text(194.0, 360.0, "ESC CANCEL", 1.0, DIM);
        }
        paint.into_canvas()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quasar_input::InputState;

    fn press(key: Key) -> CaptureEvent {
        CaptureEvent::Button {
            button: Button::Key(key),
            pressed: true,
            modifiers: Vec::new(),
        }
    }
    fn start(
        screen: &mut ControlsScreen,
        controls: &mut Controls,
        action: &'static str,
        device: Device,
    ) {
        screen.group = GROUPS
            .iter()
            .position(|(_, actions)| actions.contains(&action))
            .unwrap();
        screen.selected = (action, device);
        screen.rebuild(controls);
        screen.tick(
            Input {
                confirm: true,
                ..Default::default()
            },
            controls,
        );
        assert!(screen.capturing());
    }
    fn click(screen: &mut ControlsScreen, controls: &mut Controls, x: f32, y: f32) -> bool {
        screen.set_pointer(UiInput::default().with_pointer(x, y).pressed());
        screen.tick(Input::default(), controls)
    }

    #[test]
    fn every_global_action_is_shown_once_and_cells_show_both_devices() {
        let mut actions = GROUPS
            .iter()
            .flat_map(|(_, actions)| actions.iter().copied())
            .collect::<Vec<_>>();
        actions.sort();
        let mut expected = ACTIONS.to_vec();
        expected.sort();
        assert_eq!(actions, expected);
        let controls = Controls::default();
        assert_eq!(
            cell_value(&controls, controls::FIRE, Device::Keyboard),
            "SPACE"
        );
        assert_eq!(
            cell_value(&controls, controls::FIRE, Device::Controller),
            "A / CROSS OR RT"
        );
        assert_eq!(
            cell_value(&controls, controls::CONFIRM, Device::Keyboard),
            "ENTER OR SPACE"
        );
    }

    #[test]
    fn changing_keyboard_cell_preserves_controller_and_is_modal() {
        let mut controls = Controls::default();
        let before = controls.clone();
        let mut screen = ControlsScreen::new(&controls);
        screen.tick(
            Input {
                confirm: true,
                ..Default::default()
            },
            &mut controls,
        );
        assert!(screen.capturing());
        assert!(!screen.tick(
            Input {
                quit: true,
                back: true,
                menu_down: true,
                ..Default::default()
            },
            &mut controls
        ));
        screen.capture(
            CaptureEvent::Button {
                button: Button::Gamepad(GamepadButton::North),
                pressed: true,
                modifiers: Vec::new(),
            },
            &mut controls,
        );
        assert_eq!(controls, before); // Wrong device cannot silently change another column.
        screen.capture(press(Key::KeyQ), &mut controls);
        assert!(!screen.capturing());
        assert_eq!(cell_value(&controls, controls::FIRE, Device::Keyboard), "Q");
        assert_eq!(
            cell_value(&controls, controls::FIRE, Device::Controller),
            cell_value(&before, controls::FIRE, Device::Controller)
        );
        assert!(screen.message.contains("QUIT"));
    }

    #[test]
    fn changing_controller_cell_replaces_its_visible_assignments_only() {
        let mut controls = Controls::default();
        let mut screen = ControlsScreen::new(&controls);
        screen.tick(
            Input {
                menu_right: true,
                ..Default::default()
            },
            &mut controls,
        );
        screen.tick(
            Input {
                confirm: true,
                ..Default::default()
            },
            &mut controls,
        );
        assert_eq!(screen.capture.as_ref().unwrap().device, Device::Controller);
        screen.capture(press(Key::KeyQ), &mut controls);
        assert!(screen.capturing());
        screen.capture(
            CaptureEvent::Button {
                button: Button::Gamepad(GamepadButton::North),
                pressed: true,
                modifiers: Vec::new(),
            },
            &mut controls,
        );
        assert_eq!(
            cell_value(&controls, controls::FIRE, Device::Keyboard),
            "SPACE"
        );
        assert_eq!(
            cell_value(&controls, controls::FIRE, Device::Controller),
            "Y / TRIANGLE"
        );
        assert_eq!(controls.bindings.actions[controls::FIRE].len(), 2);
    }

    #[test]
    fn axis_prompts_name_the_direction_and_cancellation_keeps_old_controls() {
        let mut controls = Controls::default();
        let before = controls.clone();
        let mut screen = ControlsScreen::new(&controls);
        start(
            &mut screen,
            &mut controls,
            controls::ROTATE,
            Device::Keyboard,
        );
        assert_eq!(screen.prompt(), "PRESS THE KEY FOR LEFT");
        screen.capture(press(Key::KeyJ), &mut controls);
        assert_eq!(screen.prompt(), "PRESS THE KEY FOR RIGHT");
        screen.capture(CaptureEvent::Cancel, &mut controls);
        assert_eq!(controls, before);
        start(
            &mut screen,
            &mut controls,
            controls::ROTATE,
            Device::Keyboard,
        );
        screen.capture(press(Key::KeyJ), &mut controls);
        screen.capture(press(Key::KeyL), &mut controls);
        assert_eq!(
            cell_value(&controls, controls::ROTATE, Device::Keyboard),
            "J / L"
        );
        let mut input = InputState::default();
        input.set_bindings(controls.bindings.clone()).unwrap();
        input.key(Key::KeyJ, true);
        assert_eq!(input.action(controls::ROTATE).value, -1.0);
        input.key(Key::KeyJ, false);
        input.key(Key::KeyL, true);
        assert_eq!(input.action(controls::ROTATE).value, 1.0);
        start(
            &mut screen,
            &mut controls,
            controls::ROTATE,
            Device::Controller,
        );
        screen.capture(
            CaptureEvent::Axis {
                axis: GamepadAxis::RightStickX,
                value: 0.1,
            },
            &mut controls,
        );
        assert!(screen.capturing());
        screen.capture(
            CaptureEvent::Axis {
                axis: GamepadAxis::RightStickX,
                value: 0.9,
            },
            &mut controls,
        );
        assert!(controls.bindings.actions[controls::ROTATE].contains(
            &Binding::gamepad_axis_scaled(GamepadAxis::RightStickX, -1.0)
        ));
    }

    #[test]
    fn mouse_clicks_choose_categories_and_bindings_with_scaled_coordinates() {
        let mut controls = Controls::default();
        let mut screen = ControlsScreen::new(&controls);
        assert!(!click(&mut screen, &mut controls, 240.0, 115.0));
        assert_eq!(screen.group, 1);
        let point =
            ControlsScreen::pointer_position((1920, 1080), (96.0 + 450.0 * 1.8, 185.0 * 1.8));
        assert!((point.x - 450.0).abs() < 0.001 && (point.y - 185.0).abs() < 0.001);
        click(&mut screen, &mut controls, point.x, point.y);
        assert_eq!(screen.capture.as_ref().unwrap().action, controls::MENU_UP);
        screen.capture(press(Key::KeyI), &mut controls);
        assert_eq!(
            cell_value(&controls, controls::MENU_UP, Device::Keyboard),
            "I"
        );
        assert_eq!(
            cell_value(&controls, controls::MENU_UP, Device::Controller),
            "D-PAD UP"
        );
        assert!(click(&mut screen, &mut controls, 820.0, 526.0));
    }

    #[test]
    fn keyboard_navigation_clear_restore_and_toml_roundtrip() {
        let mut controls = Controls::default();
        let mut screen = ControlsScreen::new(&controls);
        screen.tick(
            Input {
                menu_up: true,
                ..Default::default()
            },
            &mut controls,
        );
        screen.tick(
            Input {
                menu_right: true,
                ..Default::default()
            },
            &mut controls,
        );
        assert_eq!(screen.group, 1);
        screen.tick(
            Input {
                menu_down: true,
                ..Default::default()
            },
            &mut controls,
        );
        screen.set_pointer(UiInput {
            delete: true,
            ..Default::default()
        });
        screen.tick(Input::default(), &mut controls);
        assert_eq!(
            cell_value(&controls, controls::MENU_UP, Device::Keyboard),
            "NOT SET"
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("controls.toml");
        controls.save(&path).unwrap();
        assert_eq!(Controls::load(&path).unwrap(), controls);
        click(&mut screen, &mut controls, 160.0, 526.0);
        assert_eq!(controls, Controls::default());
    }

    #[test]
    fn chords_modifier_taps_mouse_buttons_and_triggers_remain_supported() {
        let mut controls = Controls::default();
        let mut screen = ControlsScreen::new(&controls);
        start(&mut screen, &mut controls, controls::FIRE, Device::Keyboard);
        screen.capture(press(Key::ControlLeft), &mut controls);
        assert!(screen.capturing());
        screen.capture(
            CaptureEvent::Button {
                button: Button::Key(Key::KeyF),
                pressed: true,
                modifiers: vec![Button::Key(Key::ControlLeft)],
            },
            &mut controls,
        );
        assert_eq!(
            cell_value(&controls, controls::FIRE, Device::Keyboard),
            "CTRL+F"
        );
        start(
            &mut screen,
            &mut controls,
            controls::HYPERSPACE,
            Device::Keyboard,
        );
        screen.capture(press(Key::ShiftLeft), &mut controls);
        screen.capture(
            CaptureEvent::Button {
                button: Button::Key(Key::ShiftLeft),
                pressed: false,
                modifiers: Vec::new(),
            },
            &mut controls,
        );
        assert_eq!(
            cell_value(&controls, controls::HYPERSPACE, Device::Keyboard),
            "SHIFT"
        );
        start(&mut screen, &mut controls, controls::FIRE, Device::Keyboard);
        screen.capture(
            CaptureEvent::Button {
                button: Button::Mouse(quasar_input::MouseButton::Left),
                pressed: true,
                modifiers: Vec::new(),
            },
            &mut controls,
        );
        assert!(!screen.capturing());
        start(
            &mut screen,
            &mut controls,
            controls::THRUST,
            Device::Controller,
        );
        screen.capture(
            CaptureEvent::Axis {
                axis: GamepadAxis::RightTrigger,
                value: 0.9,
            },
            &mut controls,
        );
        assert!(!screen.capturing());
        assert_eq!(
            cell_value(&controls, controls::THRUST, Device::Controller),
            "RT"
        );
    }

    #[test]
    fn tables_and_capture_dialog_fit_catalog_and_game_canvases() {
        let mut controls = Controls::default();
        for (width, height) in [(960, 600), (1920, 1080)] {
            let mut screen = ControlsScreen::new(&controls);
            for (group, (_, actions)) in GROUPS.iter().enumerate() {
                screen.group = group;
                screen.selected = (actions[0], Device::Keyboard);
                screen.rebuild(&controls);
                let canvas = screen.scene(Canvas::new(width, height).unwrap());
                canvas.validate(&Default::default()).unwrap();
                assert_eq!(screen.cells.len(), actions.len() * 2);
                for entry in screen.ui.layout_entries() {
                    assert!(
                        entry.rect.x >= 0.0
                            && entry.rect.y >= 0.0
                            && entry.rect.right() <= width as f32
                            && entry.rect.bottom() <= height as f32
                    );
                }
            }
            start(&mut screen, &mut controls, controls::FIRE, Device::Keyboard);
            screen
                .scene(Canvas::new(width, height).unwrap())
                .validate(&Default::default())
                .unwrap();
            screen.capture(CaptureEvent::Cancel, &mut controls);
        }
    }
}
