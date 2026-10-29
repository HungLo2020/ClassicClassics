//! Editable native cinematic artwork and timelines recovered from the original intro.
use serde::Deserialize;

pub const INTRO_TICKS: u16 = 948;
pub const LOADING_TICKS: u16 = 298;

#[derive(Deserialize)]
pub struct Timeline {
    pub width: usize,
    pub height: usize,
    pub tile_size: usize,
    pub duration: u16,
    pub looping: bool,
    /// One track per screen cell; each key is [tick, artwork tile].
    pub tracks: Vec<Vec<[u32; 2]>>,
}
pub struct Presentation {
    pub intro: Timeline,
    pub title: Timeline,
    pixels: Vec<u32>,
    width: usize,
    loading: Vec<u32>,
}
impl Presentation {
    pub fn load(root: &(impl classicclassics_assets::AssetLocation + ?Sized)) -> Result<Self, String> {
        let root = root.open_assets()?;
        let image = root.join("presentation/cinematic.png").image()?;
        if image.width() != 512 || image.height() % 8 != 0 {
            return Err("cinematic.png must be an atlas with width 512 and 8-pixel rows".into());
        }
        let pixels = rgb_pixels(&image);
        let read = |name: &str| -> Result<Timeline, String> {
            let timeline: Timeline = root.join(format!("presentation/{name}.json")).json()?;
            if timeline.width != 256
                || timeline.height != 240
                || timeline.tile_size != 8
                || timeline.duration == 0
                || timeline.tracks.len() != 960
            {
                return Err(format!("invalid {name} cinematic dimensions"));
            }
            let count = image.width() * image.height() / 64;
            for track in &timeline.tracks {
                if track.first().is_none_or(|k| k[0] != 0)
                    || track
                        .iter()
                        .any(|k| k[0] >= u32::from(timeline.duration) || k[1] >= count)
                    || track.windows(2).any(|w| w[0][0] >= w[1][0])
                {
                    return Err(format!("invalid {name} cinematic keyframes"));
                }
            }
            Ok(timeline)
        };
        let intro = read("intro")?;
        if intro.duration != INTRO_TICKS {
            return Err("intro timing does not match screen transition".into());
        }
        let loading_image = root.join("presentation/loading.png").image()?;
        if loading_image.width() != 256 || loading_image.height() != 240 {
            return Err("loading.png must be 256x240".into());
        }
        let loading = rgb_pixels(&loading_image);
        Ok(Self {
            intro,
            title: read("title")?,
            pixels,
            width: image.width() as usize,
            loading,
        })
    }
    pub fn draw_loading(&self, output: &mut [u32]) {
        output.copy_from_slice(&self.loading);
    }
    pub fn draw(&self, intro: bool, tick: u64, output: &mut [u32]) {
        let timeline = if intro { &self.intro } else { &self.title };
        let tick = if timeline.looping {
            tick % u64::from(timeline.duration)
        } else {
            tick.min(u64::from(timeline.duration) - 1)
        } as u32;
        for (cell, track) in timeline.tracks.iter().enumerate() {
            let key = track.partition_point(|key| key[0] <= tick) - 1;
            let tile = track[key][1] as usize;
            let source = tile / 64 * 8 * self.width + tile % 64 * 8;
            let dest = cell / 32 * 8 * 256 + cell % 32 * 8;
            for row in 0..8 {
                output[dest + row * 256..dest + row * 256 + 8].copy_from_slice(
                    &self.pixels[source + row * self.width..source + row * self.width + 8],
                );
            }
        }
    }
}

pub(crate) fn rgb_pixels(image: &quasar_render_2d::Image) -> Vec<u32> {
    image.rgba().chunks_exact(4).map(|c| u32::from(c[0]) << 16 | u32::from(c[1]) << 8 | u32::from(c[2])).collect()
}
