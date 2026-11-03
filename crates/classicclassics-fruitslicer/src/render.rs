//! Ordered 2D scene construction shared by GPU presentation and headless PNGs.
//!
//! The renderer intentionally uses only `quasar-render-2d` primitives: ordinary
//! quads, atlas sprites and affine transforms. Fruit, halves, particles and the
//! blade trail are all drawn in a single ordered canvas.

use crate::art::{Align, Art};
use crate::game::{FRUIT_DRAW, Game, HEIGHT, Half, Kind, Mode, ParticleKind, Phase, Vec2, WIDTH};
use quasar_render_2d::{Canvas, Color, Rect, Sprite, TextureId, Transform2D};

const INK: Color = Color([247, 236, 210, 255]);
const DIM: Color = Color([206, 188, 156, 255]);
const GOLD: Color = Color([255, 208, 112, 255]);
const RED: Color = Color([255, 106, 92, 255]);
const BLUE: Color = Color([146, 226, 255, 255]);

#[derive(Debug, Clone)]
pub struct SceneOptions {
    pub best: i64,
    pub fps: f32,
    pub show_fps: bool,
    pub pointer: Option<Vec2>,
    pub virtual_cursor: bool,
    pub reduced_flash: bool,
    pub show_hud: bool,
}

impl Default for SceneOptions {
    fn default() -> Self {
        Self {
            best: 0,
            fps: 0.0,
            show_fps: false,
            pointer: None,
            virtual_cursor: false,
            reduced_flash: false,
            show_hud: true,
        }
    }
}

pub fn scene(game: &Game, art: &Art, options: &SceneOptions) -> Canvas {
    let mut canvas = Canvas::new(WIDTH as u32, HEIGHT as u32).expect("valid game canvas");
    canvas.clear = Color([8, 7, 10, 255]);

    draw_background(&mut canvas, art, game);

    // Effects in world space, with screen shake applied.
    let shake = shake_offset(game);
    let world = Transform2D::translation(shake.x, shake.y);

    // Juice stains sit on the background, behind everything else.
    canvas.set_transform(world);
    for splat in &game.splats {
        let alpha = ((splat.life / splat.max_life).clamp(0.0, 1.0) * 95.0) as u8;
        let tint = Color([splat.color[0], splat.color[1], splat.color[2], alpha]);
        rotated_texture(
            &mut canvas,
            art.splat,
            splat.position,
            splat.size,
            splat.size * 0.78,
            splat.rotation,
            tint,
            shake,
        );
    }

    for particle in &game.particles {
        if matches!(
            particle.kind,
            ParticleKind::Smoke | ParticleKind::Glow | ParticleKind::Seed
        ) {
            draw_particle(&mut canvas, art, particle, shake);
        }
    }

    // Halves and fruit share painter order by spawn age so nearby fruit overlap
    // naturally without a full scene graph.
    for half in &game.halves {
        draw_half(&mut canvas, art, half, shake);
    }
    let mut fruit_order: Vec<usize> = (0..game.fruits.len()).collect();
    fruit_order.sort_by(|a, b| {
        game.fruits[*a]
            .position
            .y
            .total_cmp(&game.fruits[*b].position.y)
    });
    for index in fruit_order {
        draw_fruit(&mut canvas, art, game, &game.fruits[index], game.elapsed);
    }

    for particle in &game.particles {
        if !matches!(
            particle.kind,
            ParticleKind::Smoke | ParticleKind::Glow | ParticleKind::Seed
        ) {
            draw_particle(&mut canvas, art, particle, shake);
        }
    }

    draw_trail(&mut canvas, art, game, shake);

    for floater in &game.floaters {
        let progress = 1.0 - floater.life / floater.max_life;
        let pop = if progress < 0.16 {
            0.7 + (progress / 0.16) * 0.45
        } else {
            1.15 - ((progress - 0.16) / 0.84) * 0.15
        };
        let alpha = if progress > 0.7 {
            ((1.0 - progress) / 0.3 * 255.0) as u8
        } else {
            255
        };
        let color = Color([floater.color[0], floater.color[1], floater.color[2], alpha]);
        canvas.set_transform(Transform2D::IDENTITY);
        art.text(
            &mut canvas,
            floater.position.x,
            floater.position.y,
            &floater.text,
            floater.size * pop,
            color,
            Align::Center,
        );
    }

    canvas.set_transform(Transform2D::IDENTITY);
    draw_screen_effects(&mut canvas, game, options.reduced_flash);
    if options.show_hud {
        draw_hud(&mut canvas, art, game, options);
    }
    if let Some(pointer) = options.pointer {
        draw_cursor(&mut canvas, art, pointer, options.virtual_cursor);
    }
    canvas
}

fn draw_background(canvas: &mut Canvas, art: &Art, game: &Game) {
    let zoom = 1.035 + (game.shake * 0.01);
    let width = WIDTH * zoom;
    let height = HEIGHT * zoom;
    canvas.set_transform(Transform2D::IDENTITY);
    canvas.sprite(Sprite {
        texture: art.background,
        destination: Rect::new(
            (WIDTH - width) * 0.5,
            (HEIGHT - height) * 0.5,
            width,
            height,
        ),
        source: None,
        tint: Color::WHITE,
        filter: Some(quasar_render_2d::Filter::Linear),
    });
}

fn shake_offset(game: &Game) -> Vec2 {
    if game.shake <= 0.0 {
        return Vec2::ZERO;
    }
    let t = game.elapsed * 61.0 + game.shake * 17.0;
    Vec2::new(t.sin() * 13.0, (t * 1.7).cos() * 9.0) * game.shake.min(1.0)
}

fn world_transform(shake: Vec2) -> Transform2D {
    Transform2D::translation(shake.x, shake.y)
}

fn rotated_transform(center: Vec2, radians: f32, shake: Vec2) -> Transform2D {
    Transform2D::translation(-center.x, -center.y)
        .then(Transform2D::rotation(radians))
        .then(Transform2D::translation(center.x, center.y))
        .then(Transform2D::translation(shake.x, shake.y))
}

#[allow(clippy::too_many_arguments)]
fn rotated_texture(
    canvas: &mut Canvas,
    texture: TextureId,
    center: Vec2,
    width: f32,
    height: f32,
    rotation: f32,
    tint: Color,
    shake: Vec2,
) {
    canvas.set_transform(rotated_transform(center, rotation, shake));
    canvas.sprite(Sprite {
        texture,
        destination: Rect::new(
            center.x - width * 0.5,
            center.y - height * 0.5,
            width,
            height,
        ),
        source: None,
        tint,
        filter: Some(quasar_render_2d::Filter::Linear),
    });
}

#[allow(clippy::too_many_arguments)]
fn draw_sprite_rotated(
    canvas: &mut Canvas,
    art: &Art,
    row: u32,
    column: u32,
    center: Vec2,
    size: f32,
    rotation: f32,
    tint: Color,
    shake: Vec2,
) {
    let Some(source) = art.fruit_region(row, column) else {
        return;
    };
    canvas.set_transform(rotated_transform(center, rotation, shake));
    canvas.sprite(Sprite {
        texture: art.fruits,
        destination: Rect::new(center.x - size * 0.5, center.y - size * 0.5, size, size),
        source: Some(source),
        tint,
        filter: Some(quasar_render_2d::Filter::Linear),
    });
}

#[allow(clippy::too_many_arguments)]
fn draw_shadow(
    canvas: &mut Canvas,
    art: &Art,
    row: u32,
    column: u32,
    center: Vec2,
    size: f32,
    rotation: f32,
    shake: Vec2,
) {
    draw_sprite_rotated(
        canvas,
        art,
        row,
        column,
        center + Vec2::new(5.0, 10.0),
        size * 1.01,
        rotation,
        Color([0, 0, 0, 82]),
        shake,
    );
}

fn draw_fruit(canvas: &mut Canvas, art: &Art, game: &Game, fruit: &crate::game::Fruit, time: f32) {
    let shake = shake_offset(game);
    let size = FRUIT_DRAW * fruit.scale;
    let info = fruit.kind.info();
    let pulse = 1.0 + (time * 3.4 + fruit.id as f32).sin() * 0.035;

    if fruit.kind.is_power() {
        let color = match fruit.kind {
            Kind::Ice => Color([132, 218, 255, 170]),
            Kind::Frenzy => Color([255, 206, 96, 180]),
            Kind::Heart => Color([255, 128, 170, 160]),
            _ => Color([255, 220, 150, 160]),
        };
        rotated_texture(
            canvas,
            art.dot,
            fruit.position,
            size * 1.75 * pulse,
            size * 1.75 * pulse,
            fruit.rotation,
            color,
            shake,
        );
    } else if fruit.kind.is_bomb() {
        rotated_texture(
            canvas,
            art.dot,
            fruit.position,
            size * 1.2,
            size * 1.2,
            fruit.rotation,
            Color([255, 92, 64, 38]),
            shake,
        );
    }

    if fruit.kind.is_bomb() {
        draw_shadow(
            canvas,
            art,
            info.art_row,
            0,
            fruit.position,
            size,
            fruit.rotation,
            shake,
        );
        canvas.set_transform(rotated_transform(fruit.position, fruit.rotation, shake));
        canvas.sprite(Sprite {
            texture: art.bomb,
            destination: Rect::new(
                fruit.position.x - size * 0.5,
                fruit.position.y - size * 0.5,
                size,
                size,
            ),
            source: None,
            tint: Color::WHITE,
            filter: Some(quasar_render_2d::Filter::Linear),
        });
        // Fuse spark: rotates with the bomb and pulses independently.
        let fuse = Vec2::new(size * 0.20, -size * 0.34).rotated(fruit.rotation) + fruit.position;
        let flicker = 0.72 + ((time * 23.0 + fruit.id as f32).sin() * 0.5 + 0.5) * 0.5;
        canvas.set_transform(world_transform(shake));
        canvas.sprite(Sprite {
            texture: art.spark,
            destination: Rect::new(
                fuse.x - 15.0 * flicker,
                fuse.y - 15.0 * flicker,
                30.0 * flicker,
                30.0 * flicker,
            ),
            source: None,
            tint: Color([255, 196, 104, 235]),
            filter: Some(quasar_render_2d::Filter::Linear),
        });
        return;
    }

    draw_shadow(
        canvas,
        art,
        info.art_row,
        0,
        fruit.position,
        size,
        fruit.rotation,
        shake,
    );
    draw_sprite_rotated(
        canvas,
        art,
        info.art_row,
        0,
        fruit.position,
        size,
        fruit.rotation,
        Color::WHITE,
        shake,
    );

    if fruit.kind.is_power() {
        let tint = match fruit.kind {
            Kind::Ice => BLUE,
            Kind::Frenzy => GOLD,
            Kind::Heart => Color([255, 164, 194, 255]),
            _ => INK,
        };
        canvas.set_transform(world_transform(shake));
        canvas.sprite(Sprite {
            texture: art.spark,
            destination: Rect::new(
                fruit.position.x - 18.0,
                fruit.position.y - size * 0.64 - 18.0,
                36.0,
                36.0,
            ),
            source: None,
            tint,
            filter: Some(quasar_render_2d::Filter::Linear),
        });
    }
}

fn draw_half(canvas: &mut Canvas, art: &Art, half: &Half, shake: Vec2) {
    let size = FRUIT_DRAW * half.scale;
    let info = half.kind.info();
    let column = 1 + half.side as u32;
    let alpha = ((half.life / half.max_life).clamp(0.0, 1.0) * 255.0) as u8;
    // Fade only in the final third so the cut face stays readable.
    let alpha = if half.life < 0.55 {
        ((half.life / 0.55).clamp(0.0, 1.0) * alpha as f32) as u8
    } else {
        alpha
    };
    draw_sprite_rotated(
        canvas,
        art,
        info.art_row,
        column,
        half.position + Vec2::new(4.0, 8.0),
        size,
        half.rotation,
        Color([0, 0, 0, (alpha as f32 * 0.28) as u8]),
        shake,
    );
    draw_sprite_rotated(
        canvas,
        art,
        info.art_row,
        column,
        half.position,
        size,
        half.rotation,
        Color([255, 255, 255, alpha]),
        shake,
    );
}

fn draw_particle(canvas: &mut Canvas, art: &Art, particle: &crate::game::Particle, shake: Vec2) {
    let life = (particle.life / particle.max_life).clamp(0.0, 1.0);
    let alpha = (life * particle.color[3] as f32) as u8;
    let color = Color([
        particle.color[0],
        particle.color[1],
        particle.color[2],
        alpha,
    ]);
    let texture = match particle.kind {
        ParticleKind::Juice => art.dot,
        ParticleKind::Spark => art.spark,
        ParticleKind::Smoke => art.smoke,
        ParticleKind::Seed => art.dot,
        ParticleKind::Glow => art.dot,
    };
    let (width, height) = match particle.kind {
        ParticleKind::Seed => (particle.size * 0.55, particle.size * 1.5),
        ParticleKind::Smoke => (particle.size * 1.5, particle.size * 1.5),
        ParticleKind::Glow => (particle.size * 2.4, particle.size * 2.4),
        _ => (particle.size, particle.size),
    };
    canvas.set_transform(rotated_transform(
        particle.position,
        particle.rotation,
        shake,
    ));
    canvas.sprite(Sprite {
        texture,
        destination: Rect::new(
            particle.position.x - width * 0.5,
            particle.position.y - height * 0.5,
            width,
            height,
        ),
        source: None,
        tint: color,
        filter: Some(quasar_render_2d::Filter::Linear),
    });
}

fn draw_trail(canvas: &mut Canvas, art: &Art, game: &Game, shake: Vec2) {
    let nodes = &game.trail;
    if nodes.len() < 2 {
        return;
    }
    canvas.set_transform(world_transform(shake));
    for pair in nodes.windows(2) {
        let a = pair[0].position;
        let b = pair[1].position;
        let delta = b - a;
        let distance = delta.length();
        if distance < 0.5 {
            continue;
        }
        let mid = a.lerp(b, 0.5);
        let angle = delta.y.atan2(delta.x) - std::f32::consts::FRAC_PI_2;
        let fade = (1.0 - pair[1].age / 0.20).clamp(0.0, 1.0);
        let width = (46.0 * fade).max(4.0);
        let height = distance + 34.0;
        let glow = Color([255, 176, 96, (70.0 * fade) as u8]);
        let core = Color([255, 252, 236, (215.0 * fade) as u8]);
        rotated_texture(
            canvas,
            art.streak,
            mid,
            width * 1.9,
            height * 1.08,
            angle,
            glow,
            shake,
        );
        rotated_texture(
            canvas,
            art.streak,
            mid,
            width * 0.52,
            height,
            angle,
            core,
            shake,
        );
        let size = (12.0 + 10.0 * fade) * 1.9;
        canvas.sprite(Sprite {
            texture: art.dot,
            destination: Rect::new(mid.x - size * 0.5, mid.y - size * 0.5, size, size),
            source: None,
            tint: glow,
            filter: Some(quasar_render_2d::Filter::Linear),
        });
    }
    if let Some(last) = nodes.last() {
        let fade = (1.0 - last.age / 0.20).clamp(0.0, 1.0);
        canvas.sprite(Sprite {
            texture: art.spark,
            destination: Rect::new(last.position.x - 34.0, last.position.y - 34.0, 68.0, 68.0),
            source: None,
            tint: Color([255, 246, 210, (210.0 * fade) as u8]),
            filter: Some(quasar_render_2d::Filter::Linear),
        });
    }
}
fn draw_screen_effects(canvas: &mut Canvas, game: &Game, reduced_flash: bool) {
    if game.is_frozen() {
        canvas.rect(
            Rect::new(0.0, 0.0, WIDTH, HEIGHT),
            Color([96, 190, 255, 28]),
        );
    }
    if game.in_frenzy() {
        canvas.rect(
            Rect::new(0.0, 0.0, WIDTH, HEIGHT),
            Color([255, 166, 58, 22]),
        );
    }
    if game.mode == Mode::Classic && game.lives <= 1 && game.phase != Phase::GameOver {
        let pulse = (game.elapsed * 4.0).sin() * 0.5 + 0.5;
        canvas.rect(
            Rect::new(0.0, 0.0, WIDTH, HEIGHT),
            Color([255, 30, 30, (18.0 + pulse * 20.0) as u8]),
        );
    }
    if game.flash > 0.0 {
        let scale = if reduced_flash { 0.22 } else { 1.0 };
        let alpha = (game.flash.clamp(0.0, 1.0) * 180.0 * scale) as u8;
        canvas.rect(
            Rect::new(0.0, 0.0, WIDTH, HEIGHT),
            Color([
                game.flash_color[0],
                game.flash_color[1],
                game.flash_color[2],
                alpha,
            ]),
        );
    }
}

fn draw_hud(canvas: &mut Canvas, art: &Art, game: &Game, options: &SceneOptions) {
    // A soft top and bottom scrim keeps text legible while fruit and bombs fly
    // underneath the HUD.
    for index in 0..18 {
        let alpha = (72.0 * (1.0 - index as f32 / 18.0)) as u8;
        canvas.rect(
            Rect::new(0.0, index as f32 * 8.0, WIDTH, 9.0),
            Color([5, 5, 8, alpha]),
        );
    }
    for index in 0..10 {
        let alpha = (6.0 + index as f32 * 6.0) as u8;
        canvas.rect(
            Rect::new(0.0, HEIGHT - 90.0 + index as f32 * 9.0, WIDTH, 10.0),
            Color([5, 5, 8, alpha]),
        );
    }
    art.text(canvas, 46.0, 40.0, "SCORE", 18.0, DIM, Align::Left);
    art.text(
        canvas,
        44.0,
        96.0,
        &game.score.to_string(),
        58.0,
        INK,
        Align::Left,
    );
    art.text(
        canvas,
        47.0,
        126.0,
        &format!("BEST {}", options.best.max(game.score)),
        17.0,
        DIM,
        Align::Left,
    );

    art.text(
        canvas,
        WIDTH * 0.5,
        34.0,
        &format!("{}  /  {}", game.mode.label(), game.difficulty.label()),
        17.0,
        DIM,
        Align::Center,
    );

    if game.mode == Mode::Classic {
        art.text(canvas, WIDTH - 46.0, 40.0, "LIVES", 18.0, DIM, Align::Right);
        for index in 0..5 {
            let filled = index < game.lives;
            let x = WIDTH - 54.0 - index as f32 * 40.0;
            if filled {
                let source = art.fruit_region(14, 0);
                canvas.sprite(Sprite {
                    texture: art.fruits,
                    destination: Rect::new(x - 17.0, 54.0, 34.0, 34.0),
                    source,
                    tint: Color::WHITE,
                    filter: Some(quasar_render_2d::Filter::Linear),
                });
            } else {
                canvas.sprite(Sprite {
                    texture: art.dot,
                    destination: Rect::new(x - 12.0, 61.0, 24.0, 24.0),
                    source: None,
                    tint: Color([80, 66, 62, 90]),
                    filter: None,
                });
            }
        }
    } else {
        let seconds = game.time_left.ceil() as u32;
        let text = format!("{}:{:02}", seconds / 60, seconds % 60);
        let urgent = game.time_left < 10.0;
        art.text(
            canvas,
            WIDTH - 46.0,
            98.0,
            &text,
            52.0,
            if urgent { RED } else { INK },
            Align::Right,
        );
        art.text(
            canvas,
            WIDTH - 46.0,
            38.0,
            "TIME LEFT",
            18.0,
            DIM,
            Align::Right,
        );
        let fraction = game.time_left / game.mode.duration().unwrap_or(1.0);
        canvas.rect(
            Rect::new(WIDTH - 300.0, 112.0, 254.0, 8.0),
            Color([0, 0, 0, 90]),
        );
        canvas.rect(
            Rect::new(WIDTH - 300.0, 112.0, 254.0 * fraction.clamp(0.0, 1.0), 8.0),
            if urgent { RED } else { GOLD },
        );
    }

    if game.combo >= 2 {
        let fraction = game.combo_fraction();
        let pop = 1.0 + (fraction * 12.0).sin() * 0.06;
        art.text(
            canvas,
            WIDTH * 0.5,
            152.0,
            &format!("COMBO x{}", game.combo),
            42.0 * pop,
            if game.combo >= 4 { GOLD } else { INK },
            Align::Center,
        );
        canvas.rect(
            Rect::new(WIDTH * 0.5 - 90.0, 164.0, 180.0, 6.0),
            Color([0, 0, 0, 100]),
        );
        canvas.rect(
            Rect::new(WIDTH * 0.5 - 90.0, 164.0, 180.0 * fraction, 6.0),
            GOLD,
        );
    }

    let mut y = HEIGHT - 42.0;
    if game.frenzy_left > 0.0 {
        let fraction = (game.frenzy_left / 8.0).clamp(0.0, 1.0);
        art.text(
            canvas,
            46.0,
            y,
            &format!("FRENZY 2X  {:.1}s", game.frenzy_left),
            18.0,
            GOLD,
            Align::Left,
        );
        canvas.rect(Rect::new(46.0, y + 6.0, 210.0, 5.0), Color([0, 0, 0, 110]));
        canvas.rect(Rect::new(46.0, y + 6.0, 210.0 * fraction, 5.0), GOLD);
        y -= 30.0;
    }
    if game.freeze_left > 0.0 {
        let fraction = (game.freeze_left / 6.0).clamp(0.0, 1.0);
        art.text(
            canvas,
            46.0,
            y,
            &format!("FREEZE  {:.1}s", game.freeze_left),
            18.0,
            BLUE,
            Align::Left,
        );
        canvas.rect(Rect::new(46.0, y + 6.0, 210.0, 5.0), Color([0, 0, 0, 110]));
        canvas.rect(Rect::new(46.0, y + 6.0, 210.0 * fraction, 5.0), BLUE);
    }

    if options.show_fps {
        art.text(
            canvas,
            WIDTH - 20.0,
            HEIGHT - 16.0,
            &format!("{:.0} FPS", options.fps),
            14.0,
            DIM,
            Align::Right,
        );
    }

    if game.phase == Phase::GameOver {
        let alpha = (game.over_timer * 1.8).clamp(0.0, 0.38);
        canvas.rect(
            Rect::new(0.0, 0.0, WIDTH, HEIGHT),
            Color([0, 0, 0, (alpha * 255.0) as u8]),
        );
    }
}

fn draw_cursor(canvas: &mut Canvas, art: &Art, pointer: Vec2, virtual_cursor: bool) {
    let pulse = 0.85 + (pointer.x * 0.01 + pointer.y * 0.013).sin() * 0.15;
    canvas.set_transform(Transform2D::IDENTITY);
    canvas.sprite(Sprite {
        texture: art.dot,
        destination: Rect::new(
            pointer.x - 34.0 * pulse,
            pointer.y - 34.0 * pulse,
            68.0 * pulse,
            68.0 * pulse,
        ),
        source: None,
        tint: Color([255, 246, 214, 48]),
        filter: Some(quasar_render_2d::Filter::Linear),
    });
    if virtual_cursor {
        canvas.sprite(Sprite {
            texture: art.ring,
            destination: Rect::new(pointer.x - 24.0, pointer.y - 24.0, 48.0, 48.0),
            source: None,
            tint: Color([255, 255, 255, 180]),
            filter: Some(quasar_render_2d::Filter::Linear),
        });
    }
    canvas.sprite(Sprite {
        texture: art.spark,
        destination: Rect::new(pointer.x - 14.0, pointer.y - 14.0, 28.0, 28.0),
        source: None,
        tint: Color([255, 255, 255, 220]),
        filter: Some(quasar_render_2d::Filter::Linear),
    });
}

/// Decorative fruit drifting behind the main menu.
pub fn draw_menu_ambience(canvas: &mut Canvas, art: &Art, time: f32) {
    let items = [
        (0u32, 176.0f32, 560.0f32, 250.0f32, 0.30f32, 1.0f32),
        (2, 1108.0, 556.0, 196.0, -0.22, 1.7),
        (1, 1082.0, 168.0, 156.0, 0.18, 2.4),
        (6, 176.0, 168.0, 150.0, -0.25, 0.6),
        (8, 246.0, 396.0, 118.0, 0.4, 3.1),
        (5, 1042.0, 352.0, 112.0, -0.5, 2.0),
    ];
    for (row, x, y, size, tilt, phase) in items {
        let bob = (time * 0.55 + phase).sin() * 10.0;
        let sway = (time * 0.37 + phase * 1.7).cos() * 8.0;
        let center = Vec2::new(x + sway, y + bob);
        let rotation = tilt + (time * 0.16 + phase).sin() * 0.08;
        let Some(source) = art.fruit_region(row, 0) else {
            continue;
        };
        canvas.set_transform(Transform2D::translation(6.0, 12.0));
        canvas.sprite(Sprite {
            texture: art.fruits,
            destination: Rect::new(center.x - size * 0.5, center.y - size * 0.5, size, size),
            source: Some(source),
            tint: Color([0, 0, 0, 70]),
            filter: Some(quasar_render_2d::Filter::Linear),
        });
        canvas.set_transform(rotated_transform(center, rotation, Vec2::ZERO));
        canvas.sprite(Sprite {
            texture: art.fruits,
            destination: Rect::new(center.x - size * 0.5, center.y - size * 0.5, size, size),
            source: Some(source),
            tint: Color([255, 255, 255, 225]),
            filter: Some(quasar_render_2d::Filter::Linear),
        });
    }
    canvas.set_transform(Transform2D::IDENTITY);
}

/// Small logo helper used by menus and the pause screen.
pub fn draw_logo(canvas: &mut Canvas, art: &Art, center: Vec2, width: f32, alpha: u8) {
    let height = width * 320.0 / 1100.0;
    canvas.sprite(Sprite {
        texture: art.logo,
        destination: Rect::new(
            center.x - width * 0.5,
            center.y - height * 0.5,
            width,
            height,
        ),
        source: None,
        tint: Color([255, 255, 255, alpha]),
        filter: Some(quasar_render_2d::Filter::Linear),
    });
}

/// A soft dim layer used under modal menus.
pub fn dim(canvas: &mut Canvas, alpha: u8) {
    canvas.rect(Rect::new(0.0, 0.0, WIDTH, HEIGHT), Color([6, 5, 9, alpha]));
}

/// Exposes the half-sprite atlas columns used by tests.
pub fn half_source_columns() -> (u32, u32) {
    (1, 2)
}
