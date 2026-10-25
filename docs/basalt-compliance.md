# Basalt compliance

This project targets the [Basalt Game Standard, version 1](https://github.com/HungLo2020/Basalt/blob/main/STANDARDS.md)
with catalog `schema_version` 2. The checklist below is copied from the standard
and marked for this project. Any deviation is called out explicitly.

**Layout and data**

- [x] The release archive contains only `app/` at its root, and no `data/` or
      `.basalt/`.
- [x] The game never writes to `app/` at runtime.
- [x] Progress and user content are stored in `data/save/`
      (`$BASALT_SAVE_DIR`).
- [x] Machine-specific settings are in `data/config/` (`$BASALT_CONFIG_DIR`);
      portable settings are in `data/config/shared/`
      (`$BASALT_SHARED_CONFIG_DIR`).
- [x] Rebuildable files belong in `data/cache/` (`$BASALT_CACHE_DIR`). The
      current build does not need any, but the directory is created only when a
      file is written there.
- [x] Nothing is stored outside the install root.
- [x] The game starts correctly with an empty or missing `data/`. Settings,
      controls and records have in-code defaults and are written on first run.

**Robustness**

- [x] Save files are written atomically by `quasar_storage::write_json`
      (temporary file in the same directory, `sync_all`, rename).
- [x] Synced data contains no absolute paths, machine identifiers or lock files.
- [x] Saves from older versions load. Format version 1 is accepted; unknown
      versions are preserved and defaults are used rather than overwriting.
- [x] The game works from any install path. `quasar_storage::GamePaths` reads
      the Basalt variables and falls back to paths relative to the install root.

**Packaging and launch**

- [x] Releases are GitHub Release `.zip` assets with predictable,
      platform-tagged names: `ClassicClassics-<version>-linux-x64.zip`.
- [x] The release bundles the executable, cover, icon and runtime notes. It
      relies only on a standard Linux desktop: glibc, a Vulkan 1.3 driver, and
      the desktop's X11/Wayland and audio stack. There is no Electron/JRE/Python
      runtime dependency.
- [x] There is no built-in updater.
- [x] The game starts without arguments, terminal input or network access.
- [x] The game exits 0 on a normal quit, non-zero on a fatal error.

**Artwork**

- [x] `app/basalt/cover.png` is a 900×1200 (3:4) opaque PNG under 2 MB.
- [x] `app/basalt/icon.png` is a 512×512 PNG with transparency.
- [x] The catalog entry's `artwork.cover_url` points at the same cover in this
      repository.

**Catalog**

- [x] Root [`ClassicClassics.json`](../ClassicClassics.json) uses
      `schema_version: 2` and `standard_version: 1`.
- [x] No `layout` or `actions` overrides are needed; the standard defaults are
      exactly what the game uses.

## Deliberate notes

- **No bundled runtime.** The executable is dynamically linked against the
  host's standard graphics, audio, X11/Wayland and C libraries, like other
  native Linux Quasar releases. A Vulkan 1.3 driver is required at runtime.
- **Audio is optional.** If no output device is available, the game runs
  silently. This is a graceful degradation, not a launch dependency.
- **Controller support** is delivered by Quasar's optional `gilrs` adapter and
  is compiled into release builds. Keyboard play is always available.
- **Catalog location.** Basalt's bundled catalog lives in `resources/games/`;
  this repository ships `ClassicClassics.json` at its root for the user to copy
  into `~/.config/basalt/games/` when trying the entry locally.
