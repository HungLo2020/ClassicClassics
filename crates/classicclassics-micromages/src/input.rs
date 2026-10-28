//! Device-independent input retained from MicroMages.
/// Gameplay actions, independent of keyboard bindings and controller protocols.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PlayerInput {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub jump: bool,
    pub fire: bool,
}
impl PlayerInput {
    pub fn from_buttons(buttons: u8) -> Self {
        Self {
            jump: buttons & 1 != 0,
            fire: buttons & 2 != 0,
            up: buttons & 16 != 0,
            down: buttons & 32 != 0,
            left: buttons & 64 != 0,
            right: buttons & 128 != 0,
        }
    }
    fn without(self, other: Self) -> Self {
        Self {
            left: self.left && !other.left,
            right: self.right && !other.right,
            up: self.up && !other.up,
            down: self.down && !other.down,
            jump: self.jump && !other.jump,
            fire: self.fire && !other.fire,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct InputFrame {
    pub held: PlayerInput,
    pub pressed: PlayerInput,
    pub released: PlayerInput,
}
impl InputFrame {
    pub fn between(previous: PlayerInput, held: PlayerInput) -> Self {
        Self {
            held,
            pressed: held.without(previous),
            released: previous.without(held),
        }
    }
}
