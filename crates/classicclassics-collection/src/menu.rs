//! Six-card catalog pages. Game-specific presentation lives in game crates.
use crate::VERSION;
use crate::controls::ControlHints;
use crate::games::{GameKind, Records};
use crate::paint::{CYAN, DIM, GRID, MUTED, Paint, WHITE};
use quasar_render_2d::Canvas;
use std::ops::Range;

pub const WIDTH: u32 = 1920;
pub const HEIGHT: u32 = 1080;
pub const COLUMNS: usize = 3;
pub const ROWS: usize = 2;
pub const PAGE_SIZE: usize = COLUMNS * ROWS;
const CARD_WIDTH: f32 = 560.0;
const CARD_HEIGHT: f32 = 368.0;
const LEFT: f32 = 80.0;
const TOP: f32 = 184.0;
const GAP: f32 = 40.0;

fn page(selected: usize, count: usize) -> Range<usize> {
    let start = selected.min(count.saturating_sub(1)) / PAGE_SIZE * PAGE_SIZE;
    start..(start + PAGE_SIZE).min(count)
}

/// Spatial navigation across a three-column grid, wrapping at the ends.
pub fn navigate(selected: usize, input: crate::Input, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    let selected = selected.min(count - 1);
    if input.menu_left {
        return (selected + count - 1) % count;
    }
    if input.menu_right {
        return (selected + 1) % count;
    }
    // Preserve W/S selection while the collection only fills one row.
    if count <= COLUMNS {
        if input.menu_up {
            return (selected + count - 1) % count;
        }
        if input.menu_down {
            return (selected + 1) % count;
        }
    }
    if input.menu_up || input.menu_down {
        let column = selected % COLUMNS;
        let rows = (count - 1 - column) / COLUMNS + 1;
        let row = selected / COLUMNS;
        let next = if input.menu_up {
            (row + rows - 1) % rows
        } else {
            (row + 1) % rows
        };
        return next * COLUMNS + column;
    }
    selected
}

pub fn scene(selected: usize, records: &Records, hints: &ControlHints) -> Canvas {
    catalog_scene(&GameKind::ALL, selected, records, hints)
}

fn catalog_scene(
    games: &[GameKind],
    selected: usize,
    records: &Records,
    hints: &ControlHints,
) -> Canvas {
    let mut paint = Paint::new(WIDTH, HEIGHT);
    paint.background();
    paint.starfield(0xC1A551C, 150, DIM);
    for x in (0..WIDTH).step_by(80) {
        paint.rect(x as f32, 0.0, 1.0, HEIGHT as f32, [12, 20, 32, 255]);
    }
    for y in (0..HEIGHT).step_by(80) {
        paint.rect(0.0, y as f32, WIDTH as f32, 1.0, [12, 20, 32, 255]);
    }
    paint.rect(LEFT, 52.0, 1760.0, 2.0, CYAN);
    paint.text(LEFT, 78.0, "CLASSIC CLASSICS", 6.0, WHITE);
    paint.text(
        LEFT + 2.0,
        140.0,
        &format!("{} ARCADE MACHINES / ONE QUASAR RUNTIME", games.len()),
        2.0,
        MUTED,
    );
    paint.right(1840.0, 82.0, &format!("COLLECTION v{VERSION}"), 2.0, DIM);
    let visible = page(selected, games.len());
    paint.right(
        1840.0,
        116.0,
        &format!(
            "PAGE {} / {}",
            visible.start / PAGE_SIZE + 1,
            games.len().div_ceil(PAGE_SIZE).max(1)
        ),
        2.0,
        DIM,
    );
    for (slot, index) in visible.enumerate() {
        let x = LEFT + (slot % COLUMNS) as f32 * (CARD_WIDTH + GAP);
        let y = TOP + (slot / COLUMNS) as f32 * (CARD_HEIGHT + GAP);
        card(&mut paint, x, y, games[index], index == selected, records);
    }
    paint.rect(LEFT, 1000.0, 1760.0, 1.0, GRID);
    paint.text(
        LEFT,
        1024.0,
        &format!(
            "ARROWS / STICK SELECT    {} PLAY    {} SETTINGS",
            hints.confirm, hints.settings
        ),
        2.0,
        MUTED,
    );
    paint.right(
        1840.0,
        1024.0,
        &format!(
            "{} FULLSCREEN   {} FILTER   {} SCALE",
            hints.fullscreen, hints.filter, hints.scale
        ),
        2.0,
        DIM,
    );
    paint.into_canvas()
}

fn card(paint: &mut Paint, x: f32, y: f32, kind: GameKind, selected: bool, records: &Records) {
    let accent = kind.accent();
    if selected {
        paint.rect(
            x - 5.0,
            y - 5.0,
            CARD_WIDTH + 10.0,
            CARD_HEIGHT + 10.0,
            accent,
        );
    }
    paint.rect(x, y, CARD_WIDTH, CARD_HEIGHT, [10, 16, 26, 255]);
    paint.outline(
        x,
        y,
        CARD_WIDTH,
        CARD_HEIGHT,
        if selected { accent } else { GRID },
    );
    paint.text(x + 24.0, y + 20.0, kind.number(), 2.0, accent);
    paint.right(
        x + CARD_WIDTH - 24.0,
        y + 20.0,
        if selected { "READY" } else { "STANDBY" },
        2.0,
        if selected { accent } else { DIM },
    );
    paint.text(x + 24.0, y + 58.0, kind.name(), 3.0, WHITE);
    paint.text(x + 24.0, y + 92.0, kind.tagline(), 2.0, MUTED);
    paint.rect(x + 24.0, y + 124.0, CARD_WIDTH - 48.0, 1.0, GRID);
    paint.rect(
        x + 24.0,
        y + 132.0,
        CARD_WIDTH - 48.0,
        142.0,
        [7, 11, 19, 255],
    );
    kind.motif(paint, x + CARD_WIDTH * 0.5, y + 203.0);
    paint.rect(x + 24.0, y + 286.0, CARD_WIDTH - 48.0, 1.0, GRID);
    let (label, value) = kind.catalog_record(records);
    paint.text(x + 24.0, y + 304.0, label, 2.0, DIM);
    paint.right(x + CARD_WIDTH - 24.0, y + 304.0, &value, 2.0, accent);
    paint.text(
        x + 24.0,
        y + 338.0,
        if selected {
            "PRESS CONFIRM TO PLAY"
        } else {
            "SELECT TO INSPECT"
        },
        1.0,
        if selected { accent } else { DIM },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Input;

    #[test]
    fn six_cards_fit_and_later_pages_keep_selection_visible() {
        assert_eq!(page(5, 8), 0..6);
        assert_eq!(page(6, 8), 6..8);
        for slot in 0..PAGE_SIZE {
            let x = LEFT + (slot % COLUMNS) as f32 * (CARD_WIDTH + GAP);
            let y = TOP + (slot / COLUMNS) as f32 * (CARD_HEIGHT + GAP);
            assert!(x >= 5.0 && x + CARD_WIDTH + 5.0 <= WIDTH as f32);
            assert!(y >= 5.0 && y + CARD_HEIGHT + 5.0 < 1000.0);
        }
        // Repeated entries exercise six populated slots without adding fake games.
        let games = [GameKind::Pong; 8];
        for selected in 0..games.len() {
            let canvas = catalog_scene(
                &games,
                selected,
                &Records::default(),
                &ControlHints::default(),
            );
            assert_eq!((canvas.width(), canvas.height()), (1920, 1080));
            canvas.validate(&Default::default()).unwrap();
        }
        let canvas = catalog_scene(&games, 5, &Records::default(), &ControlHints::default());
        let pixels = quasar_render_2d::reference::rasterize(&canvas, &Default::default()).unwrap();
        for slot in 0..PAGE_SIZE {
            let x = (LEFT + (slot % COLUMNS) as f32 * (CARD_WIDTH + GAP)) as usize;
            let y = (TOP + (slot / COLUMNS) as f32 * (CARD_HEIGHT + GAP)) as usize;
            let offset = (y * WIDTH as usize + x) * 4;
            let expected = if slot == 5 {
                GameKind::Pong.accent()
            } else {
                GRID
            };
            assert_eq!(
                &pixels[offset..offset + 4],
                &expected,
                "card {slot} must be visible"
            );
        }
    }

    #[test]
    fn navigation_wraps_columns_and_partial_rows() {
        let up = Input {
            menu_up: true,
            ..Default::default()
        };
        let down = Input {
            menu_down: true,
            ..Default::default()
        };
        assert_eq!(navigate(0, down, 6), 3);
        assert_eq!(navigate(3, down, 6), 0);
        assert_eq!(navigate(0, up, 8), 6);
        assert_eq!(navigate(5, down, 8), 2);
        assert_eq!(navigate(7, down, 8), 1);
        assert_eq!(navigate(0, down, 3), 1);
        assert_eq!(navigate(0, up, 3), 2);
    }
}
