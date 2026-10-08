# Verification record

Verified on Windows desktop on 2026-10-08 with Rust 1.99.0, Bevy 0.19.0, and Node.js 24.16.0. This host used the `stable-x86_64-pc-windows-gnu` Rust target, llvm-mingw, and `RUSTFLAGS="-C linker=rust-lld"`. The repository pins its Rust dependencies in `Cargo.lock`.

## Automated gates

- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace`: passed, including the Bevy frontend.
- `cargo test --workspace`: passed; 7 core tests and 9 integration tests, including normal-command production of ten mechanical parts.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo build -p factory-bevy`: passed and produced a runnable Windows executable.
- All three JSON scenarios passed: starting world (12 assertions), mining and belts (10), and crafting (4).
- `cargo tree -p factory-core` and `cargo tree -p factory-protocol` contained no Bevy packages.
- `node tools/render_preview.mjs` rendered a saved world to a deterministic 1536×1056 PNG.

## Live game checks

The Bevy executable launched with the generated pixel sprites. A conveyor placed from the GUI appeared in the world and deducted one stone. Right clicking it opened the inspector. Right clicking a crafter, pressing E, inserting ore with I, and changing the recipe with F3 updated the visible inventories and recipe. F5 saved a world; `factoryctl validate` accepted it. After another structure was placed, F9 restored the saved inventory and removed the later structure. A saved world with ten crafted mechanical parts launched, displayed `FACTORY GOAL COMPLETE`, and continued advancing ticks.

Screenshots from the running game: [factory line](images/factory-demo.png) and [completed objective](images/objective-complete.png). The corresponding headless gameplay tests verify depletion, timing, backpressure, contention, recipe input and output, conservation, deterministic replay, and save/load.

## Remaining platform checks

macOS launch was not exercised on this Windows host. Frame rate was not benchmarked, although the interactive Windows scene remained responsive during manual checks. Test macOS and measure frame timing on representative desktop hardware before claiming a measured 60 FPS target.
