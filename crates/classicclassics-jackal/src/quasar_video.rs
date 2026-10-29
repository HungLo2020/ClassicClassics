//! Native Quasar tilemap/atlas submission. Game-owned timing, patch selection,
//! metasprites and OAM ordering remain in renderer.rs.
use crate::{
    assets::{Assets, Level},
    game::Game,
    renderer::{Rect, Texture},
};
use classicclassics_core::tiled::{DrawFrame, Images, TileLayer};
use quasar_render_2d::{
    Canvas, Color, Filter, IndexedImage, Palette, PaletteBinding, PixelState, Priority2D, Sprite,
    SpriteTransform, TextureId,
};
use quasar_tilemap::render::{LoadedTilesets, View};
use std::collections::BTreeMap;

pub struct QuasarCanvas {
    pub canvas: Canvas,
    images: Images,
    rgba: BTreeMap<usize, TextureId>,
    indexed: BTreeMap<usize, (Texture, TextureId, TextureId)>,
    cycles: BTreeMap<(usize, usize, u8), Texture>,
    map: Option<(usize, TileLayer)>,
    atlases: BTreeMap<TextureId, LoadedTilesets>,
    pub background: bool,
}
impl Default for QuasarCanvas {
    fn default() -> Self {
        Self {
            canvas: Canvas::new(256, 240).unwrap(),
            images: Default::default(),
            rgba: Default::default(),
            indexed: Default::default(),
            cycles: Default::default(),
            map: None,
            atlases: Default::default(),
            background: false,
        }
    }
}
fn rect(r: Rect) -> quasar_render_2d::Rect {
    quasar_render_2d::Rect::new(r.x as f32, r.y as f32, r.width as f32, r.height as f32)
}
fn color(rgb: u32) -> Color {
    Color([(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255])
}
impl QuasarCanvas {
    pub fn begin(&mut self) {
        self.canvas = Canvas::new(256, 240).unwrap();
        self.background = false;
    }
    pub fn frame(&self) -> DrawFrame {
        self.images.frame(self.canvas.clone())
    }
    pub fn clear(&mut self, rgb: u32) {
        self.canvas = Canvas::new(256, 240).unwrap();
        self.canvas.clear = color(rgb);
    }
    pub fn rect(&mut self, r: Rect, rgb: u32) {
        self.canvas.rect(rect(r), color(rgb));
    }
    pub fn cycled(
        &mut self,
        source: &Texture,
        assets: &Assets,
        stage: usize,
        phase: u8,
    ) -> Texture {
        self.cycles
            .entry((source.rgba().as_ptr() as usize, stage, phase))
            .or_insert_with(|| assets.palette_cycle_art.compose(source, stage, phase))
            .clone()
    }
    fn rgba(&mut self, texture: &Texture) -> TextureId {
        *self
            .rgba
            .entry(texture.rgba().as_ptr() as usize)
            .or_insert_with(|| self.images.insert(texture.image()))
    }
    fn indexed(&mut self, texture: &Texture) -> (TextureId, TextureId) {
        let (_, id, palette) = self
            .indexed
            .entry(texture.rgba().as_ptr() as usize)
            .or_insert_with(|| {
                let mut colors = vec![Color::BLACK];
                let mut lookup = BTreeMap::from([([0, 0, 0, 255], 0u8)]);
                let mut indices = Vec::with_capacity(texture.width * texture.height);
                for rgba in texture.rgba().chunks_exact(4) {
                    let rgba: [u8; 4] = rgba.try_into().unwrap();
                    let index = *lookup.entry(rgba).or_insert_with(|| {
                        assert!(
                            colors.len() < 256,
                            "exported Jackal palette exceeds 256 colors"
                        );
                        let index = colors.len() as u8;
                        colors.push(Color(rgba));
                        index
                    });
                    indices.push(index);
                }
                let image = IndexedImage::new(texture.width as u32, texture.height as u32, indices)
                    .unwrap();
                let id = self.images.insert(image.image().clone());
                let palette = self.images.insert(Palette::new(colors).unwrap().image());
                (texture.clone(), id, palette)
            });
        (*id, *palette)
    }
    pub fn draw(
        &mut self,
        texture: &Texture,
        source: Rect,
        destination: Rect,
        flip_x: bool,
        flip_y: bool,
        priority: Option<bool>,
    ) {
        let pose = SpriteTransform {
            flip_x,
            flip_y,
            transpose: false,
        };
        if self.background && priority.is_none() {
            let (id, palette) = self.indexed(texture);
            self.canvas.set_palette(Some(PaletteBinding {
                texture: palette,
                transparent_index: None,
            }));
            let mut sprite = Sprite::new(id, rect(destination));
            sprite.source = Some(rect(source));
            sprite.filter = Some(Filter::Nearest);
            self.canvas
                .background_sprite(sprite, pose, Priority2D::default(), Some(0));
            self.canvas.set_palette(None);
        } else {
            let id = self.rgba(texture);
            let mut sprite = Sprite::new(id, rect(destination));
            sprite.source = Some(rect(source));
            sprite.filter = Some(Filter::Nearest);
            if let Some(behind) = priority {
                self.canvas
                    .priority_sprite(sprite, pose, Priority2D::default(), behind);
            } else {
                self.canvas.sprite_transformed(sprite, pose);
            }
        }
    }
    pub fn terrain(&mut self, game: &Game, level: &Level, stage: usize, atlas: &Texture) {
        self.background = true;
        let (id, palette) = self.indexed(atlas);
        let gid = |i: usize| {
            let (tile, palette) = game
                .terrain
                .patches
                .get(&i)
                .map(|p| (p.tile, p.palette))
                .unwrap_or((level.tiles[i], level.palettes[i]));
            u32::from(tile) + u32::from(palette) * 256 + 1
        };
        if self.map.as_ref().is_none_or(|(old, _)| *old != stage) {
            self.map = Some((
                stage,
                TileLayer::new(
                    level.width_tiles,
                    level.height_tiles,
                    (atlas.width as u32, atlas.height as u32),
                    (0..level.tiles.len()).map(gid).collect(),
                ),
            ));
        }
        let (_, layer) = self.map.as_mut().unwrap();
        layer.sync_visible(
            [game.camera.x.floor() as i32, game.camera.y.floor() as i32],
            gid,
        );
        let tiles = self.atlases.entry(id).or_insert_with(|| {
            // Atlas metadata is identical to the validated runtime; avoid scanning
            // a whole level again whenever an animated texture bank is selected.
            let template = TileLayer::new(1, 1, (atlas.width as u32, atlas.height as u32), vec![1]);
            LoadedTilesets::from_textures(template.runtime.map(), &self.images.textures, |_| Ok(id))
                .unwrap()
        });
        let mut pass = Canvas::new(256, 240).unwrap();
        pass.set_palette(Some(PaletteBinding {
            texture: palette,
            transparent_index: None,
        }));
        tiles
            .draw_layer(
                &layer.runtime,
                1,
                &mut pass,
                View {
                    x: f64::from(game.camera.x),
                    y: f64::from(game.camera.y),
                    width: 256,
                    height: 240,
                    zoom: 1.,
                },
                0,
                Filter::Nearest,
            )
            .unwrap();
        // Match the original game's per-tile pixel rounding, including half-pixel
        // camera ties. Quasar still owns traversal, culling and atlas selection.
        let sprites: Vec<_> = pass
            .quads()
            .iter()
            .map(|q| {
                let mut sprite = Sprite::new(
                    q.texture,
                    quasar_render_2d::Rect::new(q.origin[0].round(), q.origin[1].round(), 8., 8.),
                );
                sprite.source = q.source;
                sprite.filter = Some(Filter::Nearest);
                sprite
            })
            .collect();
        self.canvas.set_palette(Some(PaletteBinding {
            texture: palette,
            transparent_index: None,
        }));
        // Terrain cells do not overlap: color and occupancy can be submitted in
        // two contiguous batches, rather than switching stencil per tile.
        for sprite in &sprites {
            self.canvas.sprite(*sprite);
        }
        self.canvas.set_pixel_state(PixelState {
            write_mask: 1,
            reference: 1,
            color_write: false,
            alpha_min: 1,
            exclude_index: Some(0),
            ..Default::default()
        });
        for sprite in &sprites {
            self.canvas.sprite(*sprite);
        }
        self.canvas.set_pixel_state(Default::default());
        self.canvas.set_palette(None);
    }
    pub fn finish_background(&mut self) {
        self.background = false;
        self.canvas.set_pixel_state(PixelState::default());
    }
}
