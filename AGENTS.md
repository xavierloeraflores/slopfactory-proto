# Tiny Factory agent guide

The workspace has five crates. `factory-protocol` defines portable commands, IDs, and snapshots. `factory-core` owns all canonical state, generation, rules, ticks, and saves. `factory-cli` runs headlessly. `factory-bevy` maps input to commands and snapshots to visuals. `factory-tests` contains deterministic scenarios.

Never add Bevy to `factory-core` or `factory-protocol`, and never save Bevy entities. Add items and recipes in protocol definitions and core recipe tables, then update the renderer's item-to-sprite mapping. Add structures by extending protocol commands and snapshots and implementing rules in core before drawing them in Bevy. To change rendering, edit only `factory-bevy` and assets.

Build and run with `cargo run -p factory-bevy`. Run headless scenarios with `cargo run -p factory-cli -- scenario --file scenarios/<name>.json --json`. Inspect a save with `cargo run -p factory-cli -- summary --world world.json --json` or `inspect --world world.json --x 0 --y 0 --json`. When tests fail, reproduce with the CLI and compare state hashes, ticks, and machine inventories. Keep save format version 1 stable unless a migration is added.

Standard verification:

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo tree -p factory-core
cargo tree -p factory-protocol
```

The last two dependency trees must contain no Bevy package. For visual checks, run the game and inspect the HUD, machine flow, and objective. Capture a desktop screenshot if a graphics environment is available. `node tools/render_preview.mjs world.json preview.png 32 32` produces a deterministic headless scene preview from a save; it does not show the Bevy UI or animations. Headless scenarios do not need a graphics environment.
