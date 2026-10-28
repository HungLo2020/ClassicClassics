//! Grounded subpixels must not move a sprite relative to the tile grid.
use classicclassics_micromages::{
    assets::{Assets, default_root},
    game::Game,
    physics::Vec2,
    video::Video,
};
use std::{collections::BTreeSet, sync::Arc};

fn game() -> Game {
    let mut game = Game::new(Arc::new(Assets::load(&default_root()).unwrap()));
    game.start(0, 1, false).unwrap();
    let world = game.world.as_mut().unwrap();
    world.enemies.clear();
    world.props.clear();
    world.particles.clear();
    world.players[0].health = 1;
    world.camera.position.y -= 0.5;
    game
}

#[test]
fn grounded_sprite_does_not_shift_with_fractional_position_after_camera_scroll() {
    let mut game = game();
    let assets = Assets::load(&default_root()).unwrap();
    let mut video = Video::new();
    let player = &mut game.world.as_mut().unwrap().players[0];
    player.body.grounded = true;
    player.body.velocity = Vec2::default();
    player.body.position.y = player.body.position.y.floor() + 0.25;
    let first = video.render(&game, &assets).to_vec();
    game.world.as_mut().unwrap().players[0].body.position.y += 0.5;
    let second = video.render(&game, &assets);
    let changed_pixels = first.iter().zip(second).filter(|(a, b)| a != b).count();
    assert_eq!(
        changed_pixels, 0,
        "subpixel changes must not bob the grounded sprite against stationary terrain"
    );
}

#[test]
fn settled_ground_contacts_stay_on_one_rendered_row_after_camera_scroll() {
    let mut game = game();
    for _ in 0..120 {
        game.update([0; 4]);
    }
    let mut old_rows = BTreeSet::new();
    let mut world_rows = BTreeSet::new();
    let mut rendered_rows = BTreeSet::new();
    for _ in 0..64 {
        game.update([0; 4]);
        let world = game.world.as_ref().unwrap();
        let player = &world.players[0];
        assert!(player.body.grounded);
        world_rows.insert(player.body.center().y.floor() as i32);
        rendered_rows.insert(world.camera.pixel_screen(player.body.center()).y as i32);
        old_rows.insert(world.camera.screen(player.body.center()).y.floor() as i32);
    }
    assert_eq!(
        world_rows.len(),
        1,
        "collision position must remain on the same ground row"
    );
    assert_eq!(rendered_rows.len(), 1);
    assert!(
        old_rows.len() > 1,
        "the reproduction must actually exercise the old one-pixel jitter"
    );
}
