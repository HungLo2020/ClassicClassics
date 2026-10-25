//! The collection catalog: three arcade cabinets drawn as vector panels.

use crate::controls::ControlHints;
use crate::paint::{AMBER, CORAL, CYAN, DIM, GRID, MUTED, Paint, WHITE, world};
use crate::session::{GameKind, Records};
use crate::{HEIGHT, VERSION, WIDTH};
use quasar_render_2d::Canvas;

const CARD_WIDTH: f32 = 248.0;
const CARD_HEIGHT: f32 = 322.0;
const CARD_TOP: f32 = 168.0;

pub fn scene(selected: usize, records: &Records, hints: &ControlHints) -> Canvas {
    let mut paint = Paint::new(WIDTH, HEIGHT);
    paint.background();
    paint.starfield(0xC1A551C, 150, DIM);

    // Faint cabinet grid behind the cards.
    for x in (0..WIDTH as i32).step_by(48) {
        paint.rect(x as f32, 0.0, 1.0, HEIGHT as f32, [12, 20, 32, 255]);
    }
    for y in (0..HEIGHT as i32).step_by(48) {
        paint.rect(0.0, y as f32, WIDTH as f32, 1.0, [12, 20, 32, 255]);
    }

    paint.rect(64.0, 60.0, 832.0, 2.0, CYAN);
    paint.rect(64.0, 64.0, 180.0, 1.0, [32, 81, 79, 255]);
    paint.text(64.0, 82.0, "CLASSIC CLASSICS", 4.0, WHITE);
    paint.text(
        66.0,
        122.0,
        "THREE ARCADE MACHINES / ONE QUASAR RUNTIME",
        1.0,
        MUTED,
    );
    paint.right(896.0, 88.0, &format!("COLLECTION v{VERSION}"), 1.0, DIM);
    paint.right(896.0, 106.0, "120 HZ FIXED STEP", 1.0, DIM);

    for (index, kind) in GameKind::ALL.iter().copied().enumerate() {
        let x = 64.0 + index as f32 * (CARD_WIDTH + 44.0);
        card(&mut paint, x, kind, index == selected, records);
    }

    paint.rect(64.0, 520.0, 832.0, 1.0, GRID);
    paint.text(
        64.0,
        542.0,
        &format!(
            "{}/{}  SELECT    {}  PLAY    {}  SETTINGS    {}  QUIT",
            hints.p1, hints.p2, hints.confirm, hints.settings, hints.back
        ),
        1.0,
        MUTED,
    );
    paint.right(
        896.0,
        542.0,
        &format!(
            "{} FULLSCREEN   {} FILTER   {} SCALE",
            hints.fullscreen, hints.filter, hints.scale
        ),
        1.0,
        DIM,
    );
    paint.text(
        64.0,
        570.0,
        "QUASAR / INPUT / RENDER-2D / AUDIO / STORAGE / CONFIG / CORE",
        1.0,
        [55, 77, 92, 255],
    );

    paint.into_canvas()
}

fn card(paint: &mut Paint, x: f32, kind: GameKind, selected: bool, records: &Records) {
    let accent = kind.accent();
    if selected {
        paint.rect(
            x - 5.0,
            CARD_TOP - 5.0,
            CARD_WIDTH + 10.0,
            CARD_HEIGHT + 10.0,
            accent,
        );
        paint.rect(
            x - 2.0,
            CARD_TOP - 2.0,
            CARD_WIDTH + 4.0,
            CARD_HEIGHT + 4.0,
            [8, 12, 20, 255],
        );
        paint.rect(x - 3.0, CARD_TOP - 12.0, 54.0, 3.0, accent);
    }
    paint.rect(x, CARD_TOP, CARD_WIDTH, CARD_HEIGHT, [10, 16, 26, 255]);
    paint.outline(
        x,
        CARD_TOP,
        CARD_WIDTH,
        CARD_HEIGHT,
        if selected { accent } else { GRID },
    );

    paint.text(
        x + 16.0,
        CARD_TOP + 18.0,
        kind.number(),
        2.0,
        if selected { accent } else { DIM },
    );
    paint.right(
        x + CARD_WIDTH - 16.0,
        CARD_TOP + 20.0,
        if selected { "READY" } else { "STANDBY" },
        1.0,
        if selected { accent } else { DIM },
    );
    paint.text(x + 16.0, CARD_TOP + 48.0, kind.name(), 2.0, WHITE);
    paint.text(x + 16.0, CARD_TOP + 74.0, kind.tagline(), 1.0, MUTED);

    paint.rect(x + 16.0, CARD_TOP + 96.0, CARD_WIDTH - 32.0, 1.0, GRID);
    paint.rect(
        x + 16.0,
        CARD_TOP + 100.0,
        CARD_WIDTH - 32.0,
        142.0,
        [7, 11, 19, 255],
    );

    let motif_x = x + CARD_WIDTH * 0.5;
    let motif_y = CARD_TOP + 171.0;
    match kind {
        GameKind::Pong => pong_motif(paint, motif_x, motif_y, accent),
        GameKind::LunarLander => lunar_motif(paint, motif_x, motif_y, accent),
        GameKind::Asteroids => asteroids_motif(paint, motif_x, motif_y, accent),
    }

    paint.rect(x + 16.0, CARD_TOP + 258.0, CARD_WIDTH - 32.0, 1.0, GRID);
    let (label, value) = match kind {
        GameKind::Pong => ("BEST RALLY", records.pong.best_rally.to_string()),
        GameKind::LunarLander => ("BEST SCORE", records.lunar.best_score.to_string()),
        GameKind::Asteroids => ("BEST SCORE", records.asteroids.best_score.to_string()),
    };
    paint.text(x + 16.0, CARD_TOP + 276.0, label, 1.0, DIM);
    paint.right(x + CARD_WIDTH - 16.0, CARD_TOP + 276.0, &value, 1.0, accent);
    paint.text(
        x + 16.0,
        CARD_TOP + 296.0,
        if selected {
            "PRESS ENTER TO PLAY"
        } else {
            "SELECT TO INSPECT"
        },
        1.0,
        if selected { accent } else { [55, 76, 92, 255] },
    );
}

fn pong_motif(paint: &mut Paint, cx: f32, cy: f32, accent: [u8; 4]) {
    let left = cx - 82.0;
    let top = cy - 58.0;
    paint.outline(left, top, 164.0, 116.0, [30, 51, 65, 255]);
    paint.dashed_line(
        [cx, top + 6.0],
        [cx, top + 110.0],
        6.0,
        7.0,
        1.0,
        [39, 66, 77, 255],
    );
    paint.rect(left + 12.0, cy - 21.0, 5.0, 42.0, CYAN);
    paint.rect(left + 147.0, cy - 14.0, 5.0, 36.0, CORAL);
    paint.rect(cx + 22.0, cy - 8.0, 7.0, 7.0, WHITE);
    paint.line([cx - 18.0, cy + 8.0], [cx + 12.0, cy - 13.0], 1.0, accent);
}

fn lunar_motif(paint: &mut Paint, cx: f32, cy: f32, accent: [u8; 4]) {
    paint.glow_circle([cx + 52.0, cy - 38.0], 17.0, 1.0, [54, 78, 92, 255]);
    let terrain = [
        [cx - 86.0, cy + 54.0],
        [cx - 50.0, cy + 28.0],
        [cx - 18.0, cy + 41.0],
        [cx + 5.0, cy + 52.0],
        [cx + 35.0, cy + 52.0],
        [cx + 62.0, cy + 30.0],
        [cx + 88.0, cy + 47.0],
    ];
    paint.glow_polyline(&terrain, 1.5, [50, 42, 26, 255], [103, 88, 50, 255], false);
    paint.line([cx + 5.0, cy + 52.0], [cx + 35.0, cy + 52.0], 3.0, AMBER);
    paint.text(cx + 8.0, cy + 56.0, "x3", 1.0, AMBER);

    let lander = [cx - 8.0, cy - 4.0];
    let body = [
        world(lander, 0.15, [-12.0, -10.0]),
        world(lander, 0.15, [12.0, -10.0]),
        world(lander, 0.15, [15.0, 6.0]),
        world(lander, 0.15, [8.0, 15.0]),
        world(lander, 0.15, [-8.0, 15.0]),
        world(lander, 0.15, [-15.0, 6.0]),
    ];
    paint.glow_polyline(&body, 1.4, [62, 49, 25, 255], accent, true);
    paint.line(
        world(lander, 0.15, [-8.0, 15.0]),
        world(lander, 0.15, [-15.0, 23.0]),
        1.2,
        accent,
    );
    paint.line(
        world(lander, 0.15, [8.0, 15.0]),
        world(lander, 0.15, [15.0, 23.0]),
        1.2,
        accent,
    );
    paint.line(
        world(lander, 0.15, [-15.0, 23.0]),
        world(lander, 0.15, [-10.0, 23.0]),
        1.2,
        accent,
    );
    paint.line(
        world(lander, 0.15, [15.0, 23.0]),
        world(lander, 0.15, [10.0, 23.0]),
        1.2,
        accent,
    );
    paint.line(
        world(lander, 0.15, [-4.0, 16.0]),
        world(lander, 0.15, [-2.0, 28.0]),
        1.0,
        [255, 210, 120, 255],
    );
    paint.line(
        world(lander, 0.15, [4.0, 16.0]),
        world(lander, 0.15, [2.0, 31.0]),
        1.0,
        [255, 210, 120, 255],
    );
}

fn asteroids_motif(paint: &mut Paint, cx: f32, cy: f32, accent: [u8; 4]) {
    let rocks = [
        (cx - 55.0, cy - 24.0, 20.0, 0.4),
        (cx + 45.0, cy + 18.0, 29.0, 2.1),
        (cx + 14.0, cy - 44.0, 12.0, 1.2),
    ];
    for (x, y, radius, angle) in rocks {
        let points: Vec<[f32; 2]> = (0..9)
            .map(|index| {
                let a = index as f32 / 9.0 * std::f32::consts::TAU;
                let wobble = 0.72 + ((index * 37) % 11) as f32 * 0.035;
                [
                    x + (a + angle).cos() * radius * wobble,
                    y + (a + angle).sin() * radius * wobble,
                ]
            })
            .collect();
        paint.glow_polyline(&points, 1.2, [65, 38, 32, 255], [166, 111, 94, 255], true);
    }
    let ship = [cx - 8.0, cy + 28.0];
    let body = [
        world(ship, -1.57, [0.0, -15.0]),
        world(ship, -1.57, [10.0, 10.0]),
        world(ship, -1.57, [0.0, 5.0]),
        world(ship, -1.57, [-10.0, 10.0]),
    ];
    paint.glow_polyline(&body, 1.4, [42, 64, 48, 255], accent, true);
    paint.line(
        world(ship, -1.57, [0.0, 5.0]),
        world(ship, -1.57, [-4.0, 18.0]),
        1.0,
        [255, 210, 120, 255],
    );
    paint.line(
        world(ship, -1.57, [0.0, 5.0]),
        world(ship, -1.57, [4.0, 18.0]),
        1.0,
        [255, 210, 120, 255],
    );
    paint.rect(cx - 30.0, cy - 8.0, 3.0, 3.0, WHITE);
    paint.rect(cx - 56.0, cy - 12.0, 2.0, 2.0, WHITE);
}
