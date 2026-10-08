# Tiny Factory

A small 2D factory game built with a portable Rust simulation, a headless CLI, and a Bevy 0.19 desktop frontend.

## Build

Install current stable Rust. On macOS, install Xcode command line tools. On Windows, install Visual Studio C++ build tools and Windows SDK. Then run:

```sh
cargo check -p factory-core -p factory-protocol -p factory-cli
cargo test -p factory-core -p factory-tests
cargo run -p factory-bevy
```

See [docs/cli.md](docs/cli.md) for headless commands and [AGENTS.md](AGENTS.md) for the development workflow.
