/// Presentation labels derived from the first binding of each action.
#[derive(Clone, Debug)]
pub struct ControlHints {
    pub confirm: String,
    pub back: String,
    pub settings: String,
    pub menu_left: String,
    pub move_x: String,
    pub move_y: String,
    pub p1: String,
    pub p2: String,
    pub rotate: String,
    pub thrust: String,
    pub fire: String,
    pub jump: String,
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
            move_x: "A/D".into(),
            move_y: "W/S".into(),
            p1: "W/S".into(),
            p2: "UP/DOWN".into(),
            rotate: "A/D".into(),
            thrust: "W".into(),
            fire: "MOUSE1".into(),
            jump: "SPACE".into(),
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
