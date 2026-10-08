# Portability boundary

`factory-core` and `factory-protocol` cannot depend on Bevy. All gameplay actions are portable commands, and all game rules run within the core. The frontend advances the core at 20 ticks per second and reads render snapshots. Saved IDs and sprite names are plain data; renderer entities and handles never enter a save. Run `cargo tree -p factory-core` and `cargo tree -p factory-protocol` to audit dependencies.

The `debug-api` feature exposes test setup methods on the core. The CLI and scenario crate enable it; the Bevy frontend uses production gameplay commands only. It does not add cheat variants to `GameCommand`.
