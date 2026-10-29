//! Native outcome artwork. Player scores and menu choices remain live Rust UI.
use crate::scenes::Timeline;

pub struct Outcomes {
    game_over: Timeline,
    normal: Timeline,
    hard: Vec<Timeline>,
    pixels: Vec<u32>,
}
impl Outcomes {
    pub fn load(root: &(impl classicclassics_assets::AssetLocation + ?Sized)) -> Result<Self, String> {
        let root = root.open_assets()?.join("presentation/outcomes");
        let image = root.join("outcomes.png").image()?;
        if image.width() != 512 || image.height() % 8 != 0 {
            return Err("invalid outcome PNG atlas".into());
        }
        let count = image.width() * image.height() / 64;
        let read = |name: &str| -> Result<Timeline, String> {
            let timeline: Timeline = root.join(format!("{name}.json")).json()?;
            if timeline.width != 256
                || timeline.height != 240
                || timeline.tile_size != 8
                || timeline.duration == 0
                || timeline.tracks.len() != 960
            {
                return Err(format!("invalid {name} outcome dimensions"));
            }
            for track in &timeline.tracks {
                if track.first().is_none_or(|key| key[0] != 0)
                    || track
                        .iter()
                        .any(|key| key[0] >= u32::from(timeline.duration) || key[1] >= count)
                    || track.windows(2).any(|pair| pair[0][0] >= pair[1][0])
                {
                    return Err(format!("invalid {name} outcome keyframes"));
                }
            }
            Ok(timeline)
        };
        Ok(Self {
            game_over: read("game-over")?,
            normal: read("normal-ending")?,
            hard: (1..=4)
                .map(|n| read(&format!("hard-ending-{n}")))
                .collect::<Result<_, _>>()?,
            pixels: crate::scenes::rgb_pixels(&image),
        })
    }
    pub fn draw_icon(&self, player: usize, active: bool, x: usize, y: usize, output: &mut [u32]) {
        let cell = (9 + player * 2) * 32 + 12;
        let tile = self.game_over.tracks[cell][0][1] as usize;
        let source = tile / 64 * 8 * 512 + tile % 64 * 8;
        for row in 0..8 {
            for col in 0..8 {
                let color = self.pixels[source + row * 512 + col];
                output[(y + row) * 256 + x + col] = if color != 0 && !active {
                    0x585858
                } else {
                    color
                };
            }
        }
    }
    pub fn draw(&self, game_over: bool, hard: bool, players: usize, tick: u64, output: &mut [u32]) {
        let timeline = if game_over {
            &self.game_over
        } else if !hard {
            &self.normal
        } else {
            &self.hard[players.clamp(1, 4) - 1]
        };
        let tick = tick.min(u64::from(timeline.duration) - 1) as u32;
        for (cell, track) in timeline.tracks.iter().enumerate() {
            let tile = track[track.partition_point(|key| key[0] <= tick) - 1][1] as usize;
            let source = tile / 64 * 8 * 512 + tile % 64 * 8;
            let dest = cell / 32 * 8 * 256 + cell % 32 * 8;
            for row in 0..8 {
                output[dest + row * 256..dest + row * 256 + 8]
                    .copy_from_slice(&self.pixels[source + row * 512..source + row * 512 + 8]);
            }
        }
    }
}
