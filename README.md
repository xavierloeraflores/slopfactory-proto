# Tiny Factory

A small 2D factory game built with a portable Rust simulation, a headless CLI, and a Bevy 0.19 desktop frontend. The world is 64×64 tiles. Mine iron, copper, and stone; route items on conveyors; craft ten mechanical parts.

## Build

Install current stable Rust. On macOS, install Xcode command line tools. On Windows, install Visual Studio C++ build tools and Windows SDK. This repository was also tested on Windows with the GNU Rust target and the portable llvm-mingw toolchain. Then run:

```sh
cargo check --workspace
cargo test --workspace
cargo run -p factory-bevy
```

## Headless quick start

```sh
cargo run -p factory-cli --bin factoryctl -- new --seed 42 --output world.json
cargo run -p factory-cli --bin factoryctl -- summary --world world.json --json
cargo run -p factory-cli --bin factoryctl -- move --world world.json --direction east
cargo run -p factory-cli --bin factoryctl -- harvest --world world.json --x 35 --y 31
cargo run -p factory-cli --bin factoryctl -- step --world world.json --ticks 100
cargo run -p factory-cli --bin factoryctl -- validate --world world.json
cargo run -p factory-cli --bin factoryctl -- scenario --file scenarios/crafting-test.json --json
```

The CLI executable is named `factoryctl`. See [docs/cli.md](docs/cli.md) for all commands and scenario semantics. [AGENTS.md](AGENTS.md) has the verification workflow; [docs/architecture.md](docs/architecture.md) describes the engine boundary.
