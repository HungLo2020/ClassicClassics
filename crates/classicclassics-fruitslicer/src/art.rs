//! Generated asset loading through Quasar's mounted asset service.

use quasar_assets::AssetServer;
use quasar_render_2d::{
    Canvas, Color, Image, PngLoader, Rect, Sprite, SpriteAtlas, TextureId, Textures,
};
use quasar_storage::GamePaths;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const FRUIT_ATLAS: (u32, u32) = (768, 3840);

#[derive(Debug, Clone, Deserialize)]
pub struct Glyph {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub advance: u32,
    pub offset_x: i32,
    pub offset_y: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FontData {
    pub height: u32,
    pub glyphs: BTreeMap<String, Glyph>,
}

#[derive(Debug, Clone)]
pub struct BitmapFont {
    pub texture: TextureId,
    pub height: f32,
    glyphs: BTreeMap<char, Glyph>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

impl BitmapFont {
    pub fn new(texture: TextureId, data: FontData) -> Self {
        let glyphs = data
            .glyphs
            .into_iter()
            .filter_map(|(key, glyph)| key.chars().next().map(|ch| (ch, glyph)))
            .collect();
        Self {
            texture,
            height: data.height.max(1) as f32,
            glyphs,
        }
    }

    pub fn measure(&self, text: &str, size: f32) -> f32 {
        let scale = size / self.height;
        text.chars()
            .map(|ch| {
                self.glyphs
                    .get(&ch)
                    .map_or(size * 0.28, |glyph| glyph.advance as f32)
            })
            .sum::<f32>()
            * scale
    }

    /// Draws text with `baseline` as the baseline position. A soft shadow is
    /// drawn first so text reads over the dojo background.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        x: f32,
        baseline: f32,
        text: &str,
        size: f32,
        color: Color,
        align: Align,
        shadow: bool,
    ) {
        let width = self.measure(text, size);
        let pen = match align {
            Align::Left => x,
            Align::Center => x - width * 0.5,
            Align::Right => x - width,
        };
        let scale = size / self.height;
        if shadow {
            self.glyphs_at(
                canvas,
                pen + 2.0,
                baseline + 2.0,
                text,
                scale,
                Color([0, 0, 0, 165]),
            );
        }
        self.glyphs_at(canvas, pen, baseline, text, scale, color);
    }

    fn glyphs_at(
        &self,
        canvas: &mut Canvas,
        mut pen: f32,
        baseline: f32,
        text: &str,
        scale: f32,
        color: Color,
    ) {
        for ch in text.chars() {
            let Some(glyph) = self.glyphs.get(&ch) else {
                pen += self.height * scale * 0.28;
                continue;
            };
            if glyph.w > 0 && glyph.h > 0 {
                canvas.sprite(Sprite {
                    texture: self.texture,
                    destination: Rect::new(
                        pen + glyph.offset_x as f32 * scale,
                        baseline + glyph.offset_y as f32 * scale,
                        glyph.w as f32 * scale,
                        glyph.h as f32 * scale,
                    ),
                    source: Some(Rect::new(
                        glyph.x as f32,
                        glyph.y as f32,
                        glyph.w as f32,
                        glyph.h as f32,
                    )),
                    tint: color,
                    filter: Some(quasar_render_2d::Filter::Linear),
                });
            }
            pen += glyph.advance as f32 * scale;
        }
    }
}

#[derive(Debug, Clone)]
pub struct Art {
    pub fruits: TextureId,
    pub fruit_atlas: SpriteAtlas,
    pub bomb: TextureId,
    pub background: TextureId,
    pub logo: TextureId,
    pub dot: TextureId,
    pub streak: TextureId,
    pub spark: TextureId,
    pub smoke: TextureId,
    pub ring: TextureId,
    pub splat: TextureId,
    pub font: BitmapFont,
    pub root: PathBuf,
    pub images: BTreeMap<TextureId, Image>,
}

impl Art {
    /// Resolves `app/assets` first (Basalt package) and falls back to the
    /// checkout's `assets/` directory. All images are validated and cached by
    /// `quasar-assets`; decoded pixels are inserted into the renderer's
    /// immutable texture store once.
    pub fn load(paths: &GamePaths, textures: &mut Textures) -> Result<Self, String> {
        let root = asset_root(paths);
        let mut server = AssetServer::new();
        server.register(PngLoader);
        server
            .mount_basalt("", paths, "assets")
            .map_err(|error| format!("mount assets: {error}"))?;

        let mut images = BTreeMap::new();
        let load = |server: &mut AssetServer,
                    textures: &mut Textures,
                    images: &mut BTreeMap<TextureId, Image>,
                    name: &str| {
            let handle = server
                .load::<PngLoader>(name)
                .map_err(|error| format!("load {name}: {error}"))?;
            let image = (*handle).clone();
            let id = textures.insert(image.clone());
            images.insert(id, image);
            Ok::<TextureId, String>(id)
        };

        let fruits = load(&mut server, textures, &mut images, "fruits.png")?;
        let fruit_atlas =
            SpriteAtlas::grid(fruits, [FRUIT_ATLAS.0, FRUIT_ATLAS.1], [256, 256], 0, 0)
                .map_err(|error| format!("fruit atlas: {error}"))?;
        let bomb = load(&mut server, textures, &mut images, "bomb.png")?;
        let background = load(&mut server, textures, &mut images, "background.png")?;
        let logo = load(&mut server, textures, &mut images, "logo.png")?;
        let dot = load(&mut server, textures, &mut images, "dot.png")?;
        let streak = load(&mut server, textures, &mut images, "streak.png")?;
        let spark = load(&mut server, textures, &mut images, "spark.png")?;
        let smoke = load(&mut server, textures, &mut images, "smoke.png")?;
        let ring = load(&mut server, textures, &mut images, "ring.png")?;
        let splat = load(&mut server, textures, &mut images, "splat.png")?;
        let font_texture = load(&mut server, textures, &mut images, "font.png")?;
        let font_bytes =
            std::fs::read(root.join("font.json")).map_err(|error| format!("font.json: {error}"))?;
        let font_data: FontData =
            serde_json::from_slice(&font_bytes).map_err(|error| format!("font.json: {error}"))?;
        let font = BitmapFont::new(font_texture, font_data);

        Ok(Self {
            fruits,
            fruit_atlas,
            bomb,
            background,
            logo,
            dot,
            streak,
            spark,
            smoke,
            ring,
            splat,
            font,
            root,
            images,
        })
    }

    /// Load the same generated art through the collection's mounted game
    /// directory. This path never touches the filesystem directly.
    pub fn load_directory(root: &classicclassics_assets::Directory) -> Result<Self, String> {
        let mut textures = Textures::default();
        let mut images = BTreeMap::new();
        let load =
            |textures: &mut Textures, images: &mut BTreeMap<TextureId, Image>, name: &str| {
                let image = root
                    .join(name)
                    .image()
                    .map_err(|error| format!("{}: {error}", root.join(name).display()))?;
                let id = textures.insert(image.clone());
                images.insert(id, image);
                Ok::<TextureId, String>(id)
            };
        let fruits = load(&mut textures, &mut images, "fruits.png")?;
        let fruit_atlas =
            SpriteAtlas::grid(fruits, [FRUIT_ATLAS.0, FRUIT_ATLAS.1], [256, 256], 0, 0)
                .map_err(|error| format!("fruit atlas: {error}"))?;
        let bomb = load(&mut textures, &mut images, "bomb.png")?;
        let background = load(&mut textures, &mut images, "background.png")?;
        let logo = load(&mut textures, &mut images, "logo.png")?;
        let dot = load(&mut textures, &mut images, "dot.png")?;
        let streak = load(&mut textures, &mut images, "streak.png")?;
        let spark = load(&mut textures, &mut images, "spark.png")?;
        let smoke = load(&mut textures, &mut images, "smoke.png")?;
        let ring = load(&mut textures, &mut images, "ring.png")?;
        let splat = load(&mut textures, &mut images, "splat.png")?;
        let font_texture = load(&mut textures, &mut images, "font.png")?;
        let font_data: FontData = root
            .join("font.json")
            .json()
            .map_err(|error| format!("font.json: {error}"))?;
        let font = BitmapFont::new(font_texture, font_data);
        Ok(Self {
            fruits,
            fruit_atlas,
            bomb,
            background,
            logo,
            dot,
            streak,
            spark,
            smoke,
            ring,
            splat,
            font,
            root: PathBuf::new(),
            images,
        })
    }

    pub fn fruit_region(&self, row: u32, column: u32) -> Option<Rect> {
        self.fruit_atlas.region(row * 3 + column).ok()
    }
}

impl Art {
    #[allow(clippy::too_many_arguments)]
    pub fn text(
        &self,
        canvas: &mut Canvas,
        x: f32,
        baseline: f32,
        text: &str,
        size: f32,
        color: impl Into<Color>,
        align: Align,
    ) {
        self.font
            .draw(canvas, x, baseline, text, size, color.into(), align, true);
    }
}

pub fn asset_root(paths: &GamePaths) -> PathBuf {
    let packaged = paths.app.join("assets");
    if packaged.is_dir() {
        packaged
    } else {
        paths.root.join("assets")
    }
}

/// Audio files live beside the images; this exists so both loaders agree.
pub fn audio_root(paths: &GamePaths) -> PathBuf {
    asset_root(paths).join("audio")
}

pub fn resolve_dev_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// The checkout root is only a valid fallback when it still contains assets.
/// A packaged or relocated binary otherwise resolves paths from its own layout.
pub fn discovery_root() -> Option<&'static Path> {
    let root = resolve_dev_root();
    root.join("assets").is_dir().then_some(root)
}
