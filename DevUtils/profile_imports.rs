//! Deterministic CPU profile after setup, excluding window/device startup.
use classicclassics_collection::{
    GameKind, Records, Session, rendering::SceneAssets, settings::Settings,
};
use std::time::Instant;
fn main() {
    for name in ["micromages", "jackal", "missile-command", "river-raid"] {
        let mut game = Session::new(Records::default(), Settings::default());
        game.launch(GameKind::from_alias(name).unwrap());
        for _ in 0..1200 {
            game.tick(game.autoplay());
        }
        let mut assets = SceneAssets::new().unwrap();
        let mut samples = Vec::new();
        let mut quads = 0;
        for _ in 0..120 {
            for _ in 0..2 {
                game.tick(game.autoplay());
            }
            let start = Instant::now();
            let canvas = game.render(&mut assets).unwrap();
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
            quads += canvas.quads().len();
            std::hint::black_box(canvas);
        }
        samples.sort_by(f64::total_cmp);
        println!(
            "{name} scene_mean_ms={:.3} p95_ms={:.3} max_ms={:.3} mean_quads={}",
            samples.iter().sum::<f64>() / samples.len() as f64,
            samples[samples.len() * 95 / 100],
            samples.last().unwrap(),
            quads / samples.len()
        );
    }
}
