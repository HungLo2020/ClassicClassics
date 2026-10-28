//! Compatibility helpers for imported software-rendered games.
//! Original pixel renderers feed one Quasar texture. The collection owns GPU
//! uploads, presentation, scaling, overlays and headless rasterisation.
use crate::{
    HEIGHT, WIDTH,
    paint::{Paint, VOID, WHITE},
};
use quasar_render_2d::{Canvas, Image, Rect, Sprite, TextureId};
use std::cell::RefCell;
use std::path::{Path, PathBuf};

pub fn asset_root(game: &str, manifest: &str) -> PathBuf {
    if let Some(app) = std::env::var_os("BASALT_APP_DIR") {
        return PathBuf::from(app).join("games").join(game);
    }
    if let Some(root) = std::env::var_os("BASALT_GAME_DIR") {
        return PathBuf::from(root).join("app/games").join(game);
    }
    if let Some(bin) = std::env::current_exe()
        .ok()
        .as_deref()
        .and_then(Path::parent)
        && bin.file_name().is_some_and(|name| name == "bin")
        && let Some(app) = bin.parent()
    {
        return app.join("games").join(game);
    }
    Path::new(manifest).join("assets")
}

pub fn pixel_scene(pixels: &[u32], width: usize, height: usize, aspect: f32, help: &str) -> Canvas {
    assert_eq!(pixels.len(), width * height);
    let mut paint = Paint::new(WIDTH, HEIGHT);
    paint.rect(0.0, 0.0, WIDTH as f32, HEIGHT as f32, VOID);
    let available = HEIGHT as f32 - 40.0;
    let display_h = available.min(WIDTH as f32 / aspect);
    let display_w = display_h * aspect;
    let sx = display_w / width as f32;
    let sy = display_h / height as f32;
    let ox = (WIDTH as f32 - display_w) * 0.5;
    let oy = (available - display_h) * 0.5;
    for (y, row) in pixels.chunks_exact(width).enumerate() {
        let mut x = 0;
        while x < width {
            let color = row[x];
            let start = x;
            while x < width && row[x] == color {
                x += 1;
            }
            paint.rect(
                ox + start as f32 * sx,
                oy + y as f32 * sy,
                (x - start) as f32 * sx,
                sy,
                [(color >> 16) as u8, (color >> 8) as u8, color as u8, 255],
            );
        }
    }
    paint.text(16.0, HEIGHT as f32 - 25.0, help, 1.4, WHITE);
    paint.into_canvas()
}

/// A 120 Hz host preserves each original game's own fixed video frequency.
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct VideoClock {
    phase: f64,
}
impl VideoClock {
    pub fn advance(&mut self, frequency: f64) -> bool {
        self.phase += frequency;
        if self.phase >= crate::TICK_HZ {
            self.phase -= crate::TICK_HZ;
            true
        } else {
            false
        }
    }
}

/// Presentation-only cache, excluded from simulation snapshots. Cloning an Image
/// shares its RGBA allocation; a host redraw never copies or rerenders pixels.
#[derive(Default, Clone, Debug)]
pub struct FrameCache(RefCell<Option<Image>>);
impl FrameCache {
    pub fn invalidate(&self) {
        *self.0.borrow_mut() = None;
    }
    pub fn image(&self, render: impl FnOnce() -> (Vec<u32>, usize, usize)) -> Image {
        if self.0.borrow().is_none() {
            let (pixels, width, height) = render();
            let mut rgba = Vec::with_capacity(pixels.len() * 4);
            for pixel in pixels {
                rgba.extend_from_slice(&[
                    (pixel >> 16) as u8,
                    (pixel >> 8) as u8,
                    pixel as u8,
                    255,
                ]);
            }
            *self.0.borrow_mut() =
                Some(Image::new(width as u32, height as u32, rgba).expect("valid imported frame"));
        }
        self.0.borrow().as_ref().unwrap().clone()
    }
}

#[derive(Clone, Debug)]
pub struct PixelFrame {
    pub image: Image,
    pub aspect: f32,
    pub help: String,
}
impl PixelFrame {
    pub fn canvas(&self, texture: TextureId) -> Canvas {
        let mut paint = Paint::new(WIDTH, HEIGHT);
        paint.rect(0.0, 0.0, WIDTH as f32, HEIGHT as f32, VOID);
        let available = HEIGHT as f32 - 40.0;
        let height = available.min(WIDTH as f32 / self.aspect);
        let width = height * self.aspect;
        let mut canvas = paint.into_canvas();
        canvas.sprite(Sprite::new(
            texture,
            Rect::new(
                (WIDTH as f32 - width) * 0.5,
                (available - height) * 0.5,
                width,
                height,
            ),
        ));
        let mut paint = Paint::over(canvas);
        paint.text(16.0, HEIGHT as f32 - 25.0, &self.help, 1.4, WHITE);
        paint.into_canvas()
    }
    /// Geometry-only reference path for callers of the old ArcadeGame API.
    /// Native/headless hosts use canvas() and never expand pixels into quads.
    pub fn reference_canvas(&self) -> Canvas {
        let pixels: Vec<u32> = self
            .image
            .rgba()
            .chunks_exact(4)
            .map(|p| (u32::from(p[0]) << 16) | (u32::from(p[1]) << 8) | u32::from(p[2]))
            .collect();
        pixel_scene(
            &pixels,
            self.image.width() as usize,
            self.image.height() as usize,
            self.aspect,
            &self.help,
        )
    }
}
