//! Scrolling catalog grid. Game-specific presentation lives in game crates.
use crate::VERSION;
use crate::controls::ControlHints;
use crate::games::{GameKind, Records};
use crate::paint::{CYAN, DIM, GRID, MUTED, Paint, WHITE};
use crate::rendering::CatalogArt;
use quasar_render_2d::{Canvas, Rect, Sprite};

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
const ROW_GAP: f32 = 20.0;
const VIEW_BOTTOM: f32 = 1000.0;
const ROW_PITCH: f32 = CARD_HEIGHT + ROW_GAP;

pub fn scroll_offset(selected: usize) -> f32 {
    (selected / COLUMNS).saturating_sub(1) as f32 * ROW_PITCH
}

/// Move by one row without wrapping when using the wheel.
pub fn scroll_selection(selected: usize, count: usize, down: bool) -> usize {
    if count == 0 {
        return 0;
    }
    let selected = selected.min(count - 1);
    if down {
        if selected / COLUMNS >= (count - 1) / COLUMNS {
            selected
        } else {
            (selected + COLUMNS).min(count - 1)
        }
    } else {
        selected.saturating_sub(COLUMNS)
    }
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

/// Resolve a pointer against the clipped, scrolled card grid.
pub fn click(offset: f32, count: usize, [x, y]: [f32; 2]) -> Option<(usize, bool)> {
    if !(TOP..VIEW_BOTTOM).contains(&y) {
        return None;
    }
    (0..count).find_map(|index| {
        let left = LEFT + (index % COLUMNS) as f32 * (CARD_WIDTH + GAP);
        let top = TOP + (index / COLUMNS) as f32 * ROW_PITCH - offset;
        ((left..left + CARD_WIDTH).contains(&x) && (top..top + CARD_HEIGHT).contains(&y))
            .then_some((index, true))
    })
}

pub fn scene(selected: usize, offset: f32, records: &Records, hints: &ControlHints) -> Canvas {
    catalog_scene(&GameKind::ALL, selected, offset, records, hints, None)
}

pub fn scene_with_art(
    selected: usize,
    offset: f32,
    records: &Records,
    hints: &ControlHints,
    art: &CatalogArt,
) -> Canvas {
    catalog_scene(&GameKind::ALL, selected, offset, records, hints, Some(art))
}

fn catalog_scene(
    games: &[GameKind],
    selected: usize,
    offset: f32,
    records: &Records,
    hints: &ControlHints,
    artwork: Option<&CatalogArt>,
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
    if games.len() > PAGE_SIZE {
        paint.right(1840.0, 140.0, "SCROLL FOR MORE", 2.0, DIM);
    }
    let mut canvas = paint.into_canvas();
    canvas.set_clip(Some(Rect::new(
        0.0,
        TOP - 5.0,
        WIDTH as f32,
        VIEW_BOTTOM - TOP + 5.0,
    )));
    let mut paint = Paint::over(canvas);
    for (index, kind) in games.iter().copied().enumerate() {
        let x = LEFT + (index % COLUMNS) as f32 * (CARD_WIDTH + GAP);
        let y = TOP + (index / COLUMNS) as f32 * ROW_PITCH - offset;
        if y + CARD_HEIGHT + 5.0 > TOP && y - 5.0 < VIEW_BOTTOM {
            card(&mut paint, x, y, kind, index == selected, records, artwork);
        }
    }
    let mut canvas = paint.into_canvas();
    canvas.set_clip(None);
    let mut paint = Paint::over(canvas);
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
    paint.text(1100.0, 1024.0, "SETTINGS", 2.0, CYAN);
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

fn card(
    paint: &mut Paint,
    x: f32,
    y: f32,
    kind: GameKind,
    selected: bool,
    records: &Records,
    artwork: Option<&CatalogArt>,
) {
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
    if let Some(texture) = artwork.and_then(|art| art.get(kind)) {
        let mut sprite = Sprite::new(texture, Rect::new(x + 20.0, y + 20.0, 216.0, 288.0));
        sprite.filter = Some(quasar_render_2d::Filter::Linear);
        paint.sprite(sprite);
        paint.text(x + 260.0, y + 22.0, kind.number(), 2.0, accent);
        paint.text(x + 260.0, y + 59.0, &kind.name().to_uppercase(), 2.0, WHITE);
        let mut line = String::new();
        let mut row = 0;
        for word in kind.tagline().split_whitespace() {
            if line.len() + word.len() > 25 && !line.is_empty() {
                paint.text(x + 260.0, y + 104.0 + row as f32 * 22.0, &line, 1.4, MUTED);
                line.clear();
                row += 1;
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        paint.text(x + 260.0, y + 104.0 + row as f32 * 22.0, &line, 1.4, MUTED);
        let (label, value) = kind.catalog_record(records);
        paint.text(x + 260.0, y + 222.0, label, 1.5, DIM);
        paint.text(x + 260.0, y + 250.0, &value, 2.0, accent);
        paint.text(
            x + 260.0,
            y + 287.0,
            if selected { "READY" } else { "STANDBY" },
            1.5,
            accent,
        );
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
        return;
    }
    paint.text(x + 24.0, y + 20.0, kind.number(), 2.0, accent);
    paint.right(
        x + CARD_WIDTH - 24.0,
        y + 20.0,
        if selected { "READY" } else { "STANDBY" },
        2.0,
        if selected { accent } else { DIM },
    );
    paint.text(x + 24.0, y + 58.0, &kind.name().to_uppercase(), 3.0, WHITE);
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
    fn catalog_previews_next_row_and_scrolls_selected_row_into_view() {
        assert_eq!(scroll_offset(0), 0.0);
        assert_eq!(scroll_offset(5), 0.0);
        assert_eq!(scroll_offset(6), ROW_PITCH);
        let games = [GameKind::Pong; 8];
        for selected in 0..games.len() {
            let canvas = catalog_scene(
                &games,
                selected,
                scroll_offset(selected),
                &Records::default(),
                &ControlHints::default(),
                None,
            );
            canvas.validate(&Default::default()).unwrap();
        }
        assert_eq!(scroll_selection(0, 8, true), 3);
        assert_eq!(scroll_selection(3, 8, true), 6);
        assert_eq!(scroll_selection(5, 8, true), 7);
        assert_eq!(scroll_selection(7, 8, true), 7);
        assert_eq!(scroll_selection(6, 8, false), 3);
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

#[cfg(test)]
mod pointer_tests {
    use super::*;
    #[test]
    fn clicks_follow_scrolling_and_clipping() {
        assert_eq!(click(0.0, 8, [100.0, 200.0]), Some((0, true)));
        assert_eq!(click(0.0, 8, [100.0, 970.0]), Some((6, true)));
        assert_eq!(click(ROW_PITCH, 8, [100.0, 200.0]), Some((3, true)));
        assert_eq!(click(ROW_PITCH, 8, [100.0, 590.0]), Some((6, true)));
        assert_eq!(click(0.0, 8, [100.0, 1001.0]), None);
    }
}
