use classicclassics_core::{
    ControlHints, GameInfo, RomEntry,
    paint::{DIM, MUTED, Paint, WHITE},
};
use quasar_render_2d::Canvas;

const WIDTH: f32 = 960.0;
const HEIGHT: f32 = 600.0;
const ROWS: usize = 12;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Picker {
    pub selected: usize,
    pub scroll: usize,
    pub error: Option<String>,
}

impl Picker {
    pub fn clamp(&mut self, len: usize) {
        if len == 0 {
            self.selected = 0;
            self.scroll = 0;
            return;
        }
        self.selected = self.selected.min(len - 1);
        if self.selected < self.scroll {
            self.scroll = self.selected;
        }
        if self.selected >= self.scroll + ROWS {
            self.scroll = self.selected + 1 - ROWS;
        }
    }

    pub fn scene(
        &self,
        info: &GameInfo,
        roms: &[RomEntry],
        hints: &ControlHints,
        controls_help: &str,
    ) -> Canvas {
        let mut paint = Paint::new(WIDTH as u32, HEIGHT as u32);
        paint.rect(0.0, 0.0, WIDTH, HEIGHT, [8, 10, 18, 255]);
        paint.text(48.0, 36.0, &info.name.to_uppercase(), 3.0, info.accent);
        paint.text(48.0, 84.0, info.tagline, 1.6, DIM);
        paint.text(
            760.0,
            48.0,
            &format!("{} CARTRIDGES", roms.len()),
            1.6,
            WHITE,
        );
        if roms.is_empty() {
            paint.text(
                48.0,
                180.0,
                "NO CARTRIDGES FOUND",
                2.6,
                [255, 170, 140, 255],
            );
            paint.text(
                48.0,
                240.0,
                "ADD ROMS TO data/roms, app/games, OR PASS --rom-dir",
                1.5,
                WHITE,
            );
            paint.text(
                48.0,
                274.0,
                "THE BUILT-IN LIBRARY AND USER FOLDERS ARE SCANNED AT LAUNCH",
                1.3,
                DIM,
            );
        } else {
            let end = (self.scroll + ROWS).min(roms.len());
            for (row, index) in (self.scroll..end).enumerate() {
                let y = 138.0 + row as f32 * 31.0;
                let selected = index == self.selected;
                if selected {
                    paint.rect(40.0, y - 6.0, 880.0, 27.0, [30, 34, 54, 255]);
                }
                let color = if selected { info.accent } else { WHITE };
                let label = if roms[index].name.len() > 54 {
                    format!("{}...", &roms[index].name[..51])
                } else {
                    roms[index].name.clone()
                };
                paint.text(56.0, y, &label, 1.6, color);
                let kind = if roms[index].builtin {
                    "BUILT-IN"
                } else {
                    "USER"
                };
                paint.text(820.0, y, kind, 1.2, DIM);
            }
            if roms.len() > ROWS {
                paint.text(
                    820.0,
                    540.0,
                    &format!("{}/{}", self.selected + 1, roms.len()),
                    1.3,
                    DIM,
                );
            }
        }
        paint.text(48.0, 520.0, controls_help, 1.2, MUTED);
        if let Some(error) = &self.error {
            let error: String = error.to_uppercase().chars().take(96).collect();
            paint.text(48.0, 548.0, &error, 1.3, [255, 160, 140, 255]);
        }
        paint.text(
            48.0,
            574.0,
            &format!(
                "{} {} SELECT   {} PLAY   {} CATALOG",
                hints.p1, hints.menu_left, hints.confirm, hints.back
            ),
            1.3,
            DIM,
        );
        paint.into_canvas()
    }
}
