use classicclassics_core::{ArcadeGame, GameLoadContext, Input, RomEntry};
use classicclassics_emulators::{EmulatorGame, NesSystem};
use std::path::PathBuf;

/// Minimal iNES image: one 16 KiB PRG bank whose reset vector jumps to itself.
fn fixture_rom() -> Vec<u8> {
    let mut data = vec![0u8; 16 + 0x4000 + 0x2000];
    data[..4].copy_from_slice(b"NES\x1a");
    data[4] = 1;
    data[5] = 1;
    data[16..19].copy_from_slice(&[0x4c, 0x00, 0x80]);
    let reset = 16 + 0x3ffc;
    data[reset..reset + 2].copy_from_slice(&[0x00, 0x80]);
    data
}

#[test]
fn cartridge_starts_saves_state_and_leaves_the_machine() {
    let directory = tempfile::tempdir().unwrap();
    let path: PathBuf = directory.path().join("fixture.nes");
    std::fs::write(&path, fixture_rom()).unwrap();
    let entry = RomEntry {
        system: "nes".into(),
        name: "FIXTURE".into(),
        size: 16 + 0x4000 + 0x2000,
        path,
        id: 42,
        builtin: false,
    };
    let context = GameLoadContext {
        roms: vec![entry.clone()],
        initial_rom: Some(entry),
        save_dir: Some(directory.path().join("save")),
        save_enabled: true,
        ..Default::default()
    };
    let mut game = <EmulatorGame<NesSystem> as ArcadeGame>::load_with(&context).unwrap();
    assert!(game.summary().contains("running=true"));
    for _ in 0..12 {
        game.tick(Input::default(), &mut Vec::new());
    }
    game.save();
    assert!(
        directory
            .path()
            .join("save/nes/000000000000002a.state")
            .is_file()
    );
    assert!(!game.back_to_picker());
    assert!(game.summary().contains("running=false"));
    game.picker.selected = 0;
    game.tick(
        Input {
            confirm: true,
            ..Default::default()
        },
        &mut Vec::new(),
    );
    assert!(game.summary().contains("running=true"));
}
