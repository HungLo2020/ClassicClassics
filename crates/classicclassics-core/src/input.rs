/// A single fixed-step input frame. Edges (`*_pressed`, menu and command keys)
/// are consumed after one tick; held fields stay live for continuous control.
#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub menu_up: bool,
    pub menu_down: bool,
    pub menu_left: bool,
    pub menu_right: bool,
    pub confirm: bool,
    pub back: bool,
    pub settings: bool,
    pub p1: i8,
    pub p2: i8,
    pub rotate: i8,
    pub thrust: bool,
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
