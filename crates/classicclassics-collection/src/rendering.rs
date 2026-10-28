//! Shared scene resources. Headless and native hosts upload the same images;
//! only the native host owns GPU dynamic-texture handles.
use crate::{GameKind, Result};
pub use classicclassics_core::imported::PixelFrame;
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
    fn artwork(&self) -> &CatalogArt;
}

pub struct SceneAssets {
    pub textures: Textures,
    pub artwork: CatalogArt,
    frame: Option<(TextureId, Image)>,
}
impl SceneAssets {
    pub fn new() -> Result<Self> {
        let mut assets = Self {
            textures: Textures::default(),
            artwork: CatalogArt::default(),
            frame: None,
        };
        for kind in GameKind::ALL {
            if let Some(bytes) = kind.cover() {
                let image = Image::decode_png(bytes)?;
                let id = assets.textures.insert(image);
                assets.artwork.0.insert(kind.number(), id);
            }
        }
        Ok(assets)
    }
}
impl SceneTarget for SceneAssets {
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
