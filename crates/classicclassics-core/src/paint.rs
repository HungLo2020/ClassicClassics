//! Small drawing vocabulary built on `quasar_render_2d::Canvas`.
//!
//! Quasar deliberately exposes ordered quads and bitmap text rather than a
//! scene graph. The vector games draw their outlines as thin, rotated quads,
//! which keeps every frame a deterministic Quasar draw list that can be
//! rasterised headlessly.

use quasar_render_2d::{Canvas, Rect, Transform2D};

pub type Color = [u8; 4];

pub const VOID: Color = [6, 9, 16, 255];
pub const INK: Color = [10, 15, 25, 255];
pub const PANEL: Color = [14, 22, 35, 255];
pub const GRID: Color = [25, 39, 56, 255];
pub const DIM: Color = [74, 101, 122, 255];
pub const MUTED: Color = [128, 158, 178, 255];
pub const WHITE: Color = [232, 243, 245, 255];
pub const CYAN: Color = [91, 235, 214, 255];
pub const CYAN_DIM: Color = [34, 94, 89, 255];
pub const CORAL: Color = [255, 142, 112, 255];
pub const CORAL_DIM: Color = [105, 51, 42, 255];
pub const AMBER: Color = [255, 205, 105, 255];
pub const GREEN: Color = [132, 238, 131, 255];
pub const RED: Color = [255, 86, 95, 255];
pub const BLUE: Color = [112, 177, 255, 255];

/// A thin wrapper that keeps drawing calls terse and restores the canvas
/// transform after every vector primitive.
pub struct Paint {
    canvas: Canvas,
    transform: Transform2D,
}

impl Paint {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            canvas: Canvas::new(width, height).expect("valid logical canvas"),
            transform: Transform2D::IDENTITY,
        }
    }

    /// Continue drawing on an existing scene, used for modal overlays.
    pub fn over(canvas: Canvas) -> Self {
        Self {
            canvas,
            transform: Transform2D::IDENTITY,
        }
    }

    /// Fit subsequent drawing in a centered logical view, preserving aspect.
    pub fn fit_view(&mut self, width: u32, height: u32) {
        let scale = (self.canvas.width() as f32 / width as f32)
            .min(self.canvas.height() as f32 / height as f32);
        self.transform = Transform2D::scale(scale, scale).then(Transform2D::translation(
            (self.canvas.width() as f32 - width as f32 * scale) * 0.5,
            (self.canvas.height() as f32 - height as f32 * scale) * 0.5,
        ));
        self.canvas.set_transform(self.transform);
    }

    pub fn canvas(&self) -> &Canvas {
        &self.canvas
    }

    pub fn into_canvas(self) -> Canvas {
        self.canvas
    }

    pub fn background(&mut self) {
        self.rect(
            0.0,
            0.0,
            self.canvas.width() as f32,
            self.canvas.height() as f32,
            VOID,
        );
    }

    pub fn rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: Color) {
        self.canvas.rect(Rect::new(x, y, width, height), color);
    }

    pub fn outline(&mut self, x: f32, y: f32, width: f32, height: f32, color: Color) {
        self.rect(x, y, width, 1.0, color);
        self.rect(x, y + height - 1.0, width, 1.0, color);
        self.rect(x, y, 1.0, height, color);
        self.rect(x + width - 1.0, y, 1.0, height, color);
    }

    pub fn text(&mut self, x: f32, y: f32, text: &str, scale: f32, color: Color) {
        self.canvas.text([x, y], text, scale, color);
    }

    /// Draws text centred on the logical canvas.
    pub fn centered(&mut self, y: f32, text: &str, scale: f32, color: Color) {
        let width = text.chars().count() as f32 * 6.0 * scale - scale;
        self.text(
            (self.canvas.width() as f32 - width) / 2.0,
            y,
            text,
            scale,
            color,
        );
    }

    /// Text against the right edge with a pixel margin.
    pub fn right(&mut self, right: f32, y: f32, text: &str, scale: f32, color: Color) {
        let width = text.chars().count() as f32 * 6.0 * scale - scale;
        self.text(right - width, y, text, scale, color);
    }

    /// A padded label bar used by HUDs and menus.
    pub fn pill(&mut self, x: f32, y: f32, width: f32, text: &str, scale: f32, color: Color) {
        self.rect(x, y, width, 20.0, PANEL);
        self.outline(x, y, width, 20.0, GRID);
        self.text(x + 7.0, y + 6.0, text, scale, color);
    }

    /// A single vector line of the requested thickness.
    pub fn line(&mut self, a: [f32; 2], b: [f32; 2], width: f32, color: Color) {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let length = dx.hypot(dy);
        if !length.is_finite() || length < 0.01 || width <= 0.0 {
            return;
        }
        let angle = dy.atan2(dx);
        let midpoint = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
        self.canvas.set_transform(
            Transform2D::rotation(angle)
                .then(Transform2D::translation(midpoint[0], midpoint[1]))
                .then(self.transform),
        );
        self.canvas
            .rect(Rect::new(-length * 0.5, -width * 0.5, length, width), color);
        self.canvas.set_transform(self.transform);
    }

    /// A bright line with a wider muted halo, the core vector-arcade look.
    pub fn glow_line(&mut self, a: [f32; 2], b: [f32; 2], width: f32, halo: Color, core: Color) {
        self.line(a, b, width + 4.0, halo);
        self.line(a, b, width, core);
    }

    pub fn polyline(&mut self, points: &[[f32; 2]], width: f32, color: Color) {
        for pair in points.windows(2) {
            self.line(pair[0], pair[1], width, color);
        }
    }

    pub fn glow_polyline(
        &mut self,
        points: &[[f32; 2]],
        width: f32,
        halo: Color,
        core: Color,
        closed: bool,
    ) {
        for pair in points.windows(2) {
            self.glow_line(pair[0], pair[1], width, halo, core);
        }
        if closed && points.len() > 2 {
            self.glow_line(points[points.len() - 1], points[0], width, halo, core);
        }
    }

    pub fn circle(&mut self, center: [f32; 2], radius: f32, width: f32, color: Color) {
        let segments = (radius * 0.7).clamp(10.0, 40.0) as usize;
        let mut points = Vec::with_capacity(segments);
        for index in 0..segments {
            let angle = index as f32 / segments as f32 * std::f32::consts::TAU;
            points.push([
                center[0] + angle.cos() * radius,
                center[1] + angle.sin() * radius,
            ]);
        }
        self.polyline(&points, width, color);
        if let Some(last) = points.last() {
            self.line(*last, points[0], width, color);
        }
    }

    pub fn glow_circle(&mut self, center: [f32; 2], radius: f32, width: f32, color: Color) {
        let halo = [color[0] / 4, color[1] / 4, color[2] / 4, 255];
        self.circle(center, radius, width + 3.0, halo);
        self.circle(center, radius, width, color);
    }

    pub fn dashed_line(
        &mut self,
        a: [f32; 2],
        b: [f32; 2],
        dash: f32,
        gap: f32,
        width: f32,
        color: Color,
    ) {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let length = dx.hypot(dy);
        if length <= 0.0 {
            return;
        }
        let (ux, uy) = (dx / length, dy / length);
        let mut distance = 0.0;
        while distance < length {
            let end = (distance + dash).min(length);
            self.line(
                [a[0] + ux * distance, a[1] + uy * distance],
                [a[0] + ux * end, a[1] + uy * end],
                width,
                color,
            );
            distance += dash + gap;
        }
    }

    /// Deterministic star field shared by the menu and both vector games.
    pub fn starfield(&mut self, seed: u32, count: usize, brightness: Color) {
        let mut state = seed | 1;
        for _ in 0..count {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let x = (state % self.canvas.width().saturating_sub(4).max(1)) as f32 + 2.0;
            let y = (state.wrapping_mul(7) % self.canvas.height().saturating_sub(4).max(1)) as f32
                + 2.0;
            let size = if state & 31 == 0 { 2.0 } else { 1.0 };
            self.rect(x, y, size, size, brightness);
        }
    }

    pub fn dim_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.rect(x, y, width, height, [2, 3, 6, 255]);
    }
}

/// Applies a 2D rotation to a local point.
pub fn rotate(point: [f32; 2], angle: f32) -> [f32; 2] {
    let (sin, cos) = angle.sin_cos();
    [
        point[0] * cos - point[1] * sin,
        point[0] * sin + point[1] * cos,
    ]
}

/// World point plus the transform of an entity's position and heading.
pub fn world(position: [f32; 2], angle: f32, local: [f32; 2]) -> [f32; 2] {
    let rotated = rotate(local, angle);
    [position[0] + rotated[0], position[1] + rotated[1]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_helpers_produce_valid_draw_lists() {
        let mut paint = Paint::new(960, 600);
        paint.background();
        paint.line([10.0, 10.0], [100.0, 80.0], 2.0, CYAN);
        paint.polyline(&[[10.0, 10.0], [50.0, 20.0], [80.0, 90.0]], 1.0, CORAL);
        paint.circle([400.0, 300.0], 40.0, 2.0, GREEN);
        paint.dashed_line([0.0, 590.0], [960.0, 590.0], 8.0, 6.0, 1.0, GRID);
        paint.starfield(7, 80, DIM);
        paint.centered(20.0, "CLASSIC CLASSICS", 2.0, WHITE);
        let canvas = paint.into_canvas();
        canvas
            .validate(&quasar_render_2d::Textures::default())
            .unwrap();
        let pixels =
            quasar_render_2d::reference::rasterize(&canvas, &quasar_render_2d::Textures::default())
                .unwrap();
        assert_eq!(pixels.len(), 960 * 600 * 4);
    }
}
