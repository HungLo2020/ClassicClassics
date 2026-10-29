//! Shared scene resources. Headless and native hosts upload the same images;
//! only the native host owns GPU dynamic-texture handles.
use crate::{GameKind, Result};
pub use classicclassics_core::imported::PixelFrame;
pub use classicclassics_core::tiled::DrawFrame;
use quasar_render_2d::{Image, TextureId, Textures};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct CatalogArt(BTreeMap<&'static str, TextureId>);
impl CatalogArt {
    pub fn get(&self, kind: GameKind) -> Option<TextureId> {
        self.0.get(kind.number()).copied()
    }
}

pub trait SceneTarget {
    fn upload_frame(&mut self, frame: &PixelFrame) -> Result<TextureId>;
    fn draw_frame(
        &mut self,
        frame: &classicclassics_core::tiled::DrawFrame,
    ) -> Result<quasar_render_2d::Canvas>;
    fn clear_draw_frame(&mut self);
    fn artwork(&self) -> &CatalogArt;
}

pub struct SceneAssets {
    pub textures: Textures,
    pub artwork: CatalogArt,
    frame: Option<(TextureId, Image)>,
    drawn: BTreeMap<TextureId, TextureId>,
}
impl SceneAssets {
    pub fn new() -> Result<Self> {
        Self::with_assets(&mut crate::assets::GameAssets::default())
    }
    pub fn with_assets(resources: &mut crate::assets::GameAssets) -> Result<Self> {
        let catalog = resources.catalog()?;
        let mut assets = Self {
            textures: Textures::default(),
            artwork: CatalogArt::default(),
            frame: None,
            drawn: Default::default(),
        };
        for kind in GameKind::ALL {
            if kind.cover().is_some() {
                let image = catalog.join(format!("{}.png", kind.number())).image()?;
                let id = assets.textures.insert(image);
                assets.artwork.0.insert(kind.number(), id);
            }
        }
        Ok(assets)
    }
}
impl SceneAssets {
    pub fn clear_drawn(&mut self) {
        for (_, id) in std::mem::take(&mut self.drawn) {
            self.textures.remove(id);
        }
    }
    pub fn import_frame(
        &mut self,
        frame: &classicclassics_core::tiled::DrawFrame,
    ) -> Result<quasar_render_2d::Canvas> {
        if let Some((id, _)) = self.frame.take() {
            self.textures.remove(id);
        }
        self.drawn.retain(|local, host| {
            if frame.images.contains_key(local) {
                true
            } else {
                self.textures.remove(*host);
                false
            }
        });
        for (id, image) in &frame.images {
            self.drawn
                .entry(*id)
                .or_insert_with(|| self.textures.insert(image.clone()));
        }
        Ok(frame.fit(|id| self.drawn.get(&id).copied().unwrap_or(id)))
    }
}
impl SceneTarget for SceneAssets {
    fn clear_draw_frame(&mut self) {
        self.clear_drawn();
    }
    fn draw_frame(
        &mut self,
        frame: &classicclassics_core::tiled::DrawFrame,
    ) -> Result<quasar_render_2d::Canvas> {
        self.import_frame(frame)
    }
    fn artwork(&self) -> &CatalogArt {
        &self.artwork
    }
    fn upload_frame(&mut self, frame: &PixelFrame) -> Result<TextureId> {
        if let Some((id, image)) = &self.frame
            && std::ptr::eq(image.rgba().as_ptr(), frame.image.rgba().as_ptr())
        {
            return Ok(*id);
        }
        if let Some((id, _)) = self.frame.take() {
            self.textures.remove(id);
        }
        let id = self.textures.insert(frame.image.clone());
        self.frame = Some((id, frame.image.clone()));
        Ok(id)
    }
}
