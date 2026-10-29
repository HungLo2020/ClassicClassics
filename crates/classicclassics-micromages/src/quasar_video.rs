//! Quasar tilemaps and indexed atlas rendering; simulation/actor selection stays
//! in Video so the original CPU renderer remains an independent pixel oracle.
use crate::{
    assets::{Assets, Level},
    video::TileDraw,
};
use classicclassics_core::tiled::{DrawFrame, Images, TileLayer};
use quasar_render_2d::{
    Canvas, Color, Filter, IndexedImage, Palette, PaletteBinding, PixelState, Rect, Sprite,
    SpriteTransform, TextureId,
};
use quasar_tilemap::render::{LoadedTilesets, View};
use std::collections::BTreeMap;

type Terrain = ((usize, bool, usize, usize), TileLayer, LoadedTilesets);

pub struct QuasarVideo {
    pub canvas: Canvas,
    images: Images,
    atlas: Option<TextureId>,
    palettes: BTreeMap<Vec<u8>, TextureId>,
    map: Option<Terrain>,
    palette: Option<TextureId>,
}
impl Default for QuasarVideo {
    fn default() -> Self {
        Self {
            canvas: Canvas::new(256, 240).unwrap(),
            images: Default::default(),
            atlas: None,
            palettes: Default::default(),
            map: None,
            palette: None,
        }
    }
}
impl QuasarVideo {
    pub fn begin(&mut self) {
        self.canvas = Canvas::new(256, 240).unwrap();
    }
    pub fn frame(&self) -> DrawFrame {
        self.images.frame(self.canvas.clone())
    }
    fn prepare(&mut self, assets: &Assets, level: &Level) -> TextureId {
        if self.atlas.is_none() {
            // Eight atlas banks: four background palettes and four sprite palettes.
            // All shade-zero pixels retain index zero, independently of RGB color.
            let mut indices = Vec::with_capacity(assets.atlas.pixels.len() * 8);
            for bank in 0..8 {
                indices.extend(
                    assets
                        .atlas
                        .pixels
                        .iter()
                        .map(|&shade| if shade == 0 { 0 } else { bank * 4 + shade }),
                );
            }
            self.atlas = Some(
                self.images.insert(
                    IndexedImage::new(128, 2048, indices)
                        .unwrap()
                        .image()
                        .clone(),
                ),
            );
        }
        let id = *self
            .palettes
            .entry(level.colors.clone())
            .or_insert_with(|| {
                let colors: Vec<_> = level
                    .colors
                    .iter()
                    .map(|&index| {
                        let rgb = assets.colors[usize::from(index)];
                        Color([(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255])
                    })
                    .collect();
                self.images.insert(Palette::new(colors).unwrap().image())
            });
        self.palette = Some(id);
        self.atlas.unwrap()
    }
    pub fn level(&mut self, level: &Level, assets: &Assets, camera: i32) {
        let atlas = self.prepare(assets, level);
        let rgb = assets.colors[usize::from(level.colors[0])];
        self.canvas.clear = Color([(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255]);
        let key = (level.id, level.hard, level.width, level.height);
        let gid = |i: usize| u32::from(level.tiles[i]) + u32::from(level.palettes[i]) * 512 + 1;
        if self.map.as_ref().is_none_or(|(old, _, _)| *old != key) {
            let layer = TileLayer::new(
                level.width,
                level.height,
                (128, 2048),
                (0..level.tiles.len()).map(gid).collect(),
            );
            let tiles =
                LoadedTilesets::from_textures(layer.runtime.map(), &self.images.textures, |_| {
                    Ok(atlas)
                })
                .unwrap();
            self.map = Some((key, layer, tiles));
        }
        let (_, layer, tiles) = self.map.as_mut().unwrap();
        layer.sync_visible([0, camera], gid);
        self.canvas.set_palette(Some(PaletteBinding {
            texture: self.palette.unwrap(),
            transparent_index: None,
        }));
        let view = View {
            x: 0.,
            y: f64::from(camera),
            width: 256,
            height: 240,
            zoom: 1.,
        };
        tiles
            .draw_layer(
                &layer.runtime,
                1,
                &mut self.canvas,
                view,
                0,
                Filter::Nearest,
            )
            .unwrap();
        self.canvas.set_pixel_state(PixelState {
            write_mask: 1,
            reference: 1,
            color_write: false,
            alpha_min: 1,
            exclude_index: Some(0),
            ..Default::default()
        });
        tiles
            .draw_layer(
                &layer.runtime,
                1,
                &mut self.canvas,
                view,
                0,
                Filter::Nearest,
            )
            .unwrap();
        self.canvas.set_pixel_state(Default::default());
        self.canvas.set_palette(None);
    }
    pub fn tile(&mut self, draw: TileDraw, behind: bool) {
        let index = usize::from(draw.tile)
            + (usize::from(draw.palette) + usize::from(draw.sprite) * 4) * 512;
        let mut sprite = Sprite::new(
            self.atlas.unwrap(),
            Rect::new(draw.x as f32, draw.y as f32, 8., 8.),
        );
        sprite.source = Some(Rect::new(
            (index % 16 * 8) as f32,
            (index / 16 * 8) as f32,
            8.,
            8.,
        ));
        sprite.filter = Some(Filter::Nearest);
        self.canvas.set_palette(Some(PaletteBinding {
            texture: self.palette.unwrap(),
            transparent_index: draw.sprite.then_some(0),
        }));
        self.canvas.set_pixel_state(PixelState {
            read_mask: u8::from(behind),
            alpha_min: 1,
            ..Default::default()
        });
        self.canvas.sprite_transformed(
            sprite,
            SpriteTransform {
                flip_x: draw.flip_x,
                flip_y: draw.flip_y,
                transpose: false,
            },
        );
        self.canvas.set_pixel_state(Default::default());
        self.canvas.set_palette(None);
    }
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        self.canvas.rect(
            Rect::new(x as f32, y as f32, w as f32, h as f32),
            Color([(color >> 16) as u8, (color >> 8) as u8, color as u8, 255]),
        );
    }
}
