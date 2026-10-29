//! Headless Quasar tile scenes, resource snapshots and game-local presentation caches.
use crate::{
    HEIGHT, WIDTH,
    paint::{Paint, VOID, WHITE},
};
use quasar_render_2d::{Canvas, Image, Rect, Sprite, TextureId, Textures, Transform2D};
use quasar_tilemap::{Chunk, Layer, LayerContent, Map, Runtime, TileEdit, Tileset};
use std::{cell::RefCell, collections::BTreeMap};

#[derive(Clone, Debug)]
pub struct DrawFrame {
    pub canvas: Canvas,
    pub images: BTreeMap<TextureId, Image>,
    pub help: String,
}
impl DrawFrame {
    pub fn fit(&self, mut resolve: impl FnMut(TextureId) -> TextureId) -> Canvas {
        let mut paint = Paint::new(WIDTH, HEIGHT);
        paint.rect(0., 0., WIDTH as f32, HEIGHT as f32, VOID);
        let h = (HEIGHT as f32 - 40.).min(WIDTH as f32 * 3. / 4.);
        let w = h * 4. / 3.;
        let origin = [(WIDTH as f32 - w) / 2., (HEIGHT as f32 - 40. - h) / 2.];
        let scale = [
            w / self.canvas.width() as f32,
            h / self.canvas.height() as f32,
        ];
        paint.rect(origin[0], origin[1], w, h, self.canvas.clear.0);
        let mut canvas = paint.into_canvas();
        self.append(&mut canvas, origin, scale, &mut resolve);
        canvas.set_transform(Transform2D::IDENTITY);
        canvas.set_clip(None);
        canvas.set_palette(None);
        canvas.set_pixel_state(Default::default());
        let mut paint = Paint::over(canvas);
        paint.text(16., HEIGHT as f32 - 25., &self.help, 1.4, WHITE);
        paint.into_canvas()
    }
    fn append(
        &self,
        canvas: &mut Canvas,
        origin: [f32; 2],
        scale: [f32; 2],
        resolve: &mut impl FnMut(TextureId) -> TextureId,
    ) {
        for q in self.canvas.quads() {
            canvas.set_transform(Transform2D {
                x: [q.x[0] * scale[0], q.x[1] * scale[1]],
                y: [q.y[0] * scale[0], q.y[1] * scale[1]],
                translation: [
                    origin[0] + q.origin[0] * scale[0],
                    origin[1] + q.origin[1] * scale[1],
                ],
            });
            canvas.set_clip(Some(Rect::new(
                origin[0] + q.clip.x * scale[0],
                origin[1] + q.clip.y * scale[1],
                q.clip.width * scale[0],
                q.clip.height * scale[1],
            )));
            canvas.set_palette(q.palette.map(|mut p| {
                p.texture = resolve(p.texture);
                p
            }));
            canvas.set_pixel_state(q.pixels);
            let mut sprite = Sprite::new(resolve(q.texture), Rect::new(0., 0., 1., 1.));
            sprite.source = q.source;
            sprite.tint = q.color;
            sprite.filter = q.filter;
            canvas.sprite(sprite);
        }
    }
    /// Native-resolution diagnostic pixels through Quasar's headless renderer.
    pub fn reference_pixels(&self) -> Vec<u32> {
        let mut textures = Textures::default();
        let ids: BTreeMap<_, _> = self
            .images
            .iter()
            .map(|(id, image)| (*id, textures.insert(image.clone())))
            .collect();
        let mut canvas = Canvas::new(self.canvas.width(), self.canvas.height()).unwrap();
        canvas.clear = self.canvas.clear;
        self.append(&mut canvas, [0., 0.], [1., 1.], &mut |id| {
            ids.get(&id).copied().unwrap_or(id)
        });
        quasar_render_2d::reference::rasterize(&canvas, &textures)
            .unwrap()
            .chunks_exact(4)
            .map(|p| u32::from(p[0]) << 16 | u32::from(p[1]) << 8 | u32::from(p[2]))
            .collect()
    }
}

#[derive(Default)]
pub struct Images {
    pub textures: Textures,
    pub images: BTreeMap<TextureId, Image>,
}
impl Images {
    pub fn insert(&mut self, image: Image) -> TextureId {
        let id = self.textures.insert(image.clone());
        self.images.insert(id, image);
        id
    }
    pub fn frame(&self, canvas: Canvas) -> DrawFrame {
        DrawFrame {
            canvas,
            images: self.images.clone(),
            help: String::new(),
        }
    }
}

/// Presentation state is never serialized or shared by cloned simulations.
pub struct DrawCache<T> {
    state: RefCell<Option<T>>,
    frame: RefCell<Option<DrawFrame>>,
}
impl<T> Default for DrawCache<T> {
    fn default() -> Self {
        Self {
            state: RefCell::new(None),
            frame: RefCell::new(None),
        }
    }
}
impl<T> Clone for DrawCache<T> {
    fn clone(&self) -> Self {
        Self::default()
    }
}
impl<T> std::fmt::Debug for DrawCache<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DrawCache")
    }
}
impl<T: Default> DrawCache<T> {
    pub fn invalidate(&self) {
        *self.frame.borrow_mut() = None;
    }
    pub fn render(&self, render: impl FnOnce(&mut T) -> DrawFrame) -> DrawFrame {
        if self.frame.borrow().is_none() {
            let mut state = self.state.borrow_mut();
            *self.frame.borrow_mut() = Some(render(state.get_or_insert_with(T::default)));
        }
        self.frame.borrow().as_ref().unwrap().clone()
    }
}

/// One mutable presentation layer. The game's level remains authoritative;
/// synchronize visible changes as one Quasar transactional batch, without rebuilds.
pub struct TileLayer {
    pub runtime: Runtime,
    cells: Vec<u32>,
    width: usize,
    height: usize,
}
impl TileLayer {
    pub fn new(width: usize, height: usize, atlas: (u32, u32), cells: Vec<u32>) -> Self {
        let map = Map {
            quasar_version: 1,
            tile_width: 8,
            tile_height: 8,
            width: width as u32,
            height: height as u32,
            infinite: false,
            background: [0, 0, 0, 255],
            properties: Default::default(),
            metadata: Default::default(),
            tilesets: vec![Tileset {
                first_gid: 1,
                name: "game atlas".into(),
                image: "atlas.png".into(),
                image_width: atlas.0,
                image_height: atlas.1,
                tile_width: 8,
                tile_height: 8,
                tile_count: atlas.0 / 8 * (atlas.1 / 8),
                columns: atlas.0 / 8,
                margin: 0,
                spacing: 0,
                tiles: Default::default(),
                properties: Default::default(),
                metadata: Default::default(),
            }],
            layers: vec![Layer {
                id: 1,
                name: "terrain".into(),
                visible: true,
                opacity: 1.,
                offset: [0., 0.],
                parallax: [1., 1.],
                tint: [255; 4],
                properties: Default::default(),
                metadata: Default::default(),
                content: LayerContent::Tiles {
                    chunks: vec![Chunk {
                        x: 0,
                        y: 0,
                        width: width as u32,
                        height: height as u32,
                        cells: cells.clone(),
                    }],
                },
            }],
        };
        Self {
            runtime: Runtime::new(map).expect("validated game tile layer"),
            cells,
            width,
            height,
        }
    }
    pub fn sync_visible(&mut self, camera: [i32; 2], mut gid: impl FnMut(usize) -> u32) -> usize {
        let mut edits = Vec::new();
        let x0 = (camera[0].div_euclid(8)).max(0) as usize;
        let y0 = (camera[1].div_euclid(8)).max(0) as usize;
        for y in y0..(y0 + 32).min(self.height) {
            for x in x0..(x0 + 34).min(self.width) {
                let i = y * self.width + x;
                let next = gid(i);
                if self.cells[i] != next {
                    edits.push(TileEdit {
                        layer: 1,
                        x: x as i32,
                        y: y as i32,
                        gid: next,
                    });
                    self.cells[i] = next;
                }
            }
        }
        if !edits.is_empty() {
            self.runtime
                .set_tiles(&edits)
                .expect("validated game terrain edit");
        }
        edits.len()
    }
}
