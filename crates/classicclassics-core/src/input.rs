/// A single fixed-step input frame. Edges (`*_pressed`, menu and command keys)
/// are consumed after one tick; held fields stay live for continuous control.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Input {
    pub menu_up: bool,
    pub menu_down: bool,
    pub menu_left: bool,
    pub menu_right: bool,
    pub confirm: bool,
    pub back: bool,
    pub settings: bool,
    pub move_x: i8,
    pub move_y: i8,
    pub p1: i8,
    pub p2: i8,
    pub rotate: i8,
    pub thrust: bool,
    pub jump: bool,
    pub jump_pressed: bool,
    pub secondary: bool,
    pub secondary_pressed: bool,
    /// Other local players (controller slots 2..4, keyboard player two).
    pub other_players: [PlayerControls; 3],
    /// Pointer position in the shared 960x600 logical game view, when moved.
    pub pointer: Option<[f32; 2]>,
    pub fire: bool,
    pub fire_pressed: bool,
    pub hyperspace: bool,
    pub pause: bool,
    pub restart: bool,
    pub mode: bool,
    pub mute: bool,
    pub quit: bool,
    pub fullscreen: bool,
    pub filter: bool,
    pub scale: bool,
}

/// Device-independent controls shared by imported local multiplayer games.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PlayerControls {
    pub x: i8,
    pub y: i8,
    pub fire: bool,
    pub jump: bool,
    pub secondary: bool,
}
impl Input {
    /// Preserve taps between original video frames while updating held inputs.
    pub fn latch(&mut self, next: Self) {
        let previous = *self;
        *self = next;
        self.confirm |= previous.confirm;
        self.mode |= previous.mode;
        self.fire_pressed |= previous.fire_pressed;
        self.jump_pressed |= previous.jump_pressed;
        self.secondary_pressed |= previous.secondary_pressed;
        self.pointer = next.pointer.or(previous.pointer);
        for (current, previous) in self.other_players.iter_mut().zip(previous.other_players) {
            current.fire |= previous.fire;
            current.jump |= previous.jump;
            current.secondary |= previous.secondary;
        }
    }
}
