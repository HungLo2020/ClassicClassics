//! Recovered stage texture changes and destructible boss background compositions.
use crate::renderer::Texture;
use serde::Deserialize;
#[derive(Deserialize)]
pub struct BossTilePatch {
    pub id: u8,
    pub width: usize,
    pub height: usize,
    pub offset_x: i32,
    pub offset_y: i32,
    pub tiles: Vec<u16>,
    pub collision: u8,
}
#[derive(Deserialize)]
struct PatchFile {
    patches: Vec<BossTilePatch>,
}
#[derive(Deserialize)]
struct PhaseSource {
    distance: f32,
    #[serde(default)]
    tick_offset: u64,
    tiles: String,
    sprites: String,
}
#[derive(Deserialize)]
struct PhasesFile {
    stages: Vec<Vec<PhaseSource>>,
}
pub struct TexturePhase {
    pub distance: f32,
    pub tick_offset: u64,
    pub tiles: Texture,
    pub sprites: Texture,
}
fn png(root: &(impl classicclassics_assets::AssetLocation + ?Sized), name: &str, expected: (u32, u32)) -> Result<Texture, String> {
        let root = root.open_assets()?;
    let img = root.join(name).image()
        .map_err(|e| e.to_string())?
        ;
    if (img.width(), img.height()) != expected {
        return Err(format!("Invalid world atlas dimensions: {name}"));
    }
    Texture::from_rgba(img.width() as usize, img.height() as usize, img.rgba().to_vec())
}
pub fn load(root: &(impl classicclassics_assets::AssetLocation + ?Sized)) -> Result<(Vec<Vec<TexturePhase>>, Vec<BossTilePatch>), String> {
        let root = root.open_assets()?;
    let source: PhasesFile = serde_json::from_slice(
        &root.join("texture_phases.json").read().map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if source.stages.len() != 6 {
        return Err("Texture phases must cover six stages".into());
    }
    let mut stages = Vec::new();
    for stage in source.stages {
        let mut phases = Vec::new();
        let mut previous = (0., 0);
        for phase in stage {
            if !phase.distance.is_finite() || (phase.distance, phase.tick_offset) < previous {
                return Err("Texture phases must be ordered".into());
            }
            previous = (phase.distance, phase.tick_offset);
            phases.push(TexturePhase {
                distance: phase.distance,
                tick_offset: phase.tick_offset,
                tiles: png(&root, &phase.tiles, (128, 512))?,
                sprites: png(&root, &phase.sprites, (128, 1024))?,
            });
        }
        stages.push(phases);
    }
    let patches: PatchFile = serde_json::from_slice(
        &root.join("boss_patches.json").read().map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if patches.patches.iter().enumerate().any(|(id, p)| {
        usize::from(p.id) != id
            || p.width.checked_mul(p.height) != Some(p.tiles.len())
            || p.width == 0
            || p.height == 0
            || p.collision > 3
            || p.tiles.iter().any(|&t| t > 255)
    }) {
        return Err("Invalid boss background patch".into());
    }
    Ok((stages, patches.patches))
}
