# scena

`scena` is a Rust-native scene-graph renderer intended to become an easy,
production-safe replacement for Three.js in Rust applications.

The initial charter is in [`docs/RFC-rust-3d-renderer.md`](docs/RFC-rust-3d-renderer.md).

Non-goals are explicit: this is not a game engine, not a simulation engine, not robotics
logic, not PLC/domain logic, and not physics. `scena` owns rendering, scene graph,
assets, authoring helpers, diagnostics, and testable output.

Initial local checks:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
