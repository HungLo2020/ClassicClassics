//! Retained-menu painter using FruitSlicer's high-resolution bitmap font.
//!
//! `quasar-ui` supplies layout, focus and interaction state. This painter is a
//! drop-in replacement for `UiRenderer` that renders text from the 64 px
//! generated font atlas instead of the tiny built-in 5×7 font, so menu text
//! stays crisp at every window size.

use crate::art::BitmapFont;
use quasar_render_2d::{Canvas, Color, Rect};
use quasar_ui::{TextStyle, Ui, Widget};

/// Draws the whole retained tree in layout order.
pub fn draw(canvas: &mut Canvas, ui: &Ui, font: &BitmapFont) {
    let theme = ui.theme();
    let scale = ui.scale();
    let body = theme.font_size * scale;
    let small = theme.small_size * scale;
    let heading = theme.heading_size * scale;
    let padding = theme.padding * scale;

    for entry in ui.layout_entries() {
        let Some(node) = ui.node(entry.id) else {
            continue;
        };
        if !node.visible {
            continue;
        }
        canvas.set_clip(Some(Rect::new(
            entry.clip.x,
            entry.clip.y,
            entry.clip.width,
            entry.clip.height,
        )));
        let rect = Rect::new(
            entry.rect.x,
            entry.rect.y,
            entry.rect.width,
            entry.rect.height,
        );
        match &node.widget {
            Widget::Panel { .. } => {
                let color = if node.modal {
                    theme.panel
                } else {
                    theme.panel_alt
                };
                canvas.rect(rect, to_color(color));
                if node.modal {
                    focus_border(canvas, rect, theme.focus, theme.focus_width * scale);
                }
            }
            Widget::Label { text, style } => {
                let size = match style {
                    TextStyle::Heading => heading,
                    TextStyle::Body => body,
                    TextStyle::Small | TextStyle::Dim => small,
                };
                draw_centered_line(
                    canvas,
                    font,
                    rect,
                    ui.resolve_text(text),
                    size,
                    theme.color(*style),
                );
            }
            Widget::Button { text, .. } => {
                let fill = if node.state.pressed {
                    theme.button_active
                } else if node.state.hovered || node.state.focus_t > 0.5 {
                    theme.button_hover
                } else {
                    theme.button
                };
                canvas.rect(rect, to_color(fill));
                draw_centered_line(
                    canvas,
                    font,
                    rect,
                    ui.resolve_text(text),
                    body,
                    if node.enabled {
                        theme.text
                    } else {
                        theme.disabled
                    },
                );
                if node.state.focus_t > 0.01 {
                    focus_border(
                        canvas,
                        rect,
                        theme.focus.with_alpha((255.0 * node.state.focus_t) as u8),
                        theme.focus_width * scale,
                    );
                }
            }
            Widget::Slider {
                label,
                value,
                min,
                max,
                step,
                ..
            } => {
                if node.state.focus_t > 0.01 {
                    focus_border(canvas, rect, theme.focus, theme.focus_width * scale);
                }
                let track = Rect::new(
                    rect.x,
                    rect.y + rect.height - 5.0 * scale,
                    rect.width,
                    4.0 * scale,
                );
                canvas.rect(track, to_color(theme.track));
                let fraction = if *max > *min {
                    ((*value - *min) / (*max - *min)).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                canvas.rect(
                    Rect::new(track.x, track.y, track.width * fraction, track.height),
                    to_color(theme.fill),
                );
                let knob = 7.0 * scale;
                canvas.rect(
                    Rect::new(
                        track.x + track.width * fraction - knob * 0.5,
                        track.y - knob * 0.5,
                        knob,
                        knob,
                    ),
                    to_color(if node.state.focus_t > 0.5 {
                        theme.focus
                    } else {
                        theme.accent
                    }),
                );
                draw_line(
                    canvas,
                    font,
                    rect.x + 4.0 * scale,
                    rect.y + small * 1.28,
                    ui.resolve_text(label),
                    small,
                    theme.dim_text,
                );
                let value_text = format_value(*value, *step);
                let value_width = font.measure(&value_text, small);
                draw_line(
                    canvas,
                    font,
                    (rect.x + rect.width) - value_width,
                    rect.y + small * 1.28,
                    &value_text,
                    small,
                    theme.text,
                );
            }
            Widget::Checkbox { label, checked, .. } => {
                let box_size = (rect.height - 4.0 * scale).max(10.0);
                let box_rect = Rect::new(rect.x, rect.y + 2.0 * scale, box_size, box_size);
                canvas.rect(box_rect, to_color(theme.track));
                if *checked {
                    canvas.rect(
                        Rect::new(
                            box_rect.x + 3.0 * scale,
                            box_rect.y + 3.0 * scale,
                            box_size - 6.0 * scale,
                            box_size - 6.0 * scale,
                        ),
                        to_color(theme.fill),
                    );
                }
                draw_centered_line_in(
                    canvas,
                    font,
                    Rect::new(
                        box_rect.x + box_rect.width + 8.0 * scale,
                        rect.y,
                        (rect.x + rect.width) - (box_rect.x + box_rect.width) - 8.0 * scale,
                        rect.height,
                    ),
                    ui.resolve_text(label),
                    body,
                    theme.text,
                );
                if node.state.focus_t > 0.01 {
                    focus_border(canvas, rect, theme.focus, theme.focus_width * scale);
                }
            }
            Widget::Select {
                label,
                options,
                selected,
                ..
            } => {
                canvas.rect(rect, to_color(theme.button));
                draw_centered_line_in(
                    canvas,
                    font,
                    Rect::new(rect.x + padding, rect.y, rect.width * 0.58, rect.height),
                    ui.resolve_text(label),
                    body,
                    theme.text,
                );
                if let Some(option) = options.get(*selected) {
                    let text = ui.resolve_text(option);
                    let width = font.measure(text, body);
                    draw_centered_line_in(
                        canvas,
                        font,
                        Rect::new(
                            (rect.x + rect.width) - width - padding,
                            rect.y,
                            width,
                            rect.height,
                        ),
                        text,
                        body,
                        theme.accent,
                    );
                }
                if node.state.focus_t > 0.01 {
                    focus_border(canvas, rect, theme.focus, theme.focus_width * scale);
                }
            }
            Widget::Spacer
            | Widget::Tabs { .. }
            | Widget::TextInput { .. }
            | Widget::Progress { .. }
            | Widget::Image { .. }
            | Widget::List { .. }
            | Widget::Binding { .. } => {
                // The current menus use only the widgets above. Unknown widgets
                // still occupy layout space and keep focus behavior intact.
            }
        }
    }
    canvas.set_clip(None);
}

fn draw_line(
    canvas: &mut Canvas,
    font: &BitmapFont,
    x: f32,
    baseline: f32,
    text: &str,
    size: f32,
    color: quasar_ui::Color,
) {
    font.draw(
        canvas,
        x.round(),
        baseline.round(),
        text,
        size,
        to_color(color),
        crate::art::Align::Left,
        true,
    );
}

fn draw_centered_line(
    canvas: &mut Canvas,
    font: &BitmapFont,
    rect: Rect,
    text: &str,
    size: f32,
    color: quasar_ui::Color,
) {
    let width = font.measure(text, size);
    draw_centered_line_in(
        canvas,
        font,
        Rect::new(
            rect.x + (rect.width - width) * 0.5,
            rect.y,
            width,
            rect.height,
        ),
        text,
        size,
        color,
    );
}

fn draw_centered_line_in(
    canvas: &mut Canvas,
    font: &BitmapFont,
    rect: Rect,
    text: &str,
    size: f32,
    color: quasar_ui::Color,
) {
    // The generated font has a 47/76 cap height; centre the visible cap box.
    let baseline = rect.y + rect.height * 0.5 + size * 0.31;
    draw_line(canvas, font, rect.x, baseline, text, size, color);
}

fn focus_border(canvas: &mut Canvas, rect: Rect, color: quasar_ui::Color, width: f32) {
    let color = to_color(color);
    let width = width.max(1.0);
    canvas.rect(Rect::new(rect.x, rect.y, rect.width, width), color);
    canvas.rect(
        Rect::new(rect.x, rect.y + rect.height - width, rect.width, width),
        color,
    );
    canvas.rect(Rect::new(rect.x, rect.y, width, rect.height), color);
    canvas.rect(
        Rect::new(rect.x + rect.width - width, rect.y, width, rect.height),
        color,
    );
}

fn format_value(value: f32, step: f32) -> String {
    let step = step.abs();
    if !step.is_finite() || step <= 0.0 {
        format!("{value:.2}")
    } else if step >= 1.0 {
        format!("{value:.0}")
    } else if step >= 0.1 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}

fn to_color(color: quasar_ui::Color) -> Color {
    Color([color.r, color.g, color.b, color.a])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::art::{Art, discovery_root};
    use crate::menu::Menu;
    use crate::settings::{Records, Settings};
    use quasar_render_2d::{Canvas, Textures};
    use quasar_storage::GamePaths;
    use quasar_ui::UiInput;

    #[test]
    fn generated_font_paints_main_menu() {
        let root = discovery_root().expect("checkout assets");
        let paths = GamePaths::discover(Some(root)).expect("paths");
        let mut textures = Textures::default();
        let art = Art::load(&paths, &mut textures).expect("assets");
        let mut menu = Menu::new(Settings::default(), false, Records::default());
        menu.open_main();
        menu.update(&UiInput::default(), 1.0 / 60.0)
            .expect("layout");
        let mut canvas = Canvas::new(1280, 720).expect("canvas");
        draw(&mut canvas, menu.ui().expect("open"), &art.font);
        assert!(canvas.quads().len() > 40, "menu did not draw");
        let pixels = quasar_render_2d::reference::rasterize(&canvas, &textures).expect("raster");
        assert_eq!(pixels.len(), 1280 * 720 * 4);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel != [0, 0, 0, 255]));
    }
}
