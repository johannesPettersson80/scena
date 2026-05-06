# scena

`scena` is a Rust-native scene-graph renderer intended to become an easy,
production-safe replacement for Three.js in Rust applications.

The initial charter is in [`docs/RFC-rust-3d-renderer.md`](docs/RFC-rust-3d-renderer.md).
Implementation contracts live in [`docs/specs/`](docs/specs/), especially the
[`public API`](docs/specs/public-api.md), [`module boundaries`](docs/specs/module-boundaries.md),
and [`release gates`](docs/specs/release-gates.md). Milestone checklists live in
[`docs/checklists/`](docs/checklists/), and accepted decisions live in
[`docs/decisions/`](docs/decisions/).

Non-goals are explicit: this is not a game engine, not a simulation engine, not robotics
logic, not PLC/domain logic, and not physics. `scena` owns rendering, scene graph,
assets, authoring helpers, diagnostics, and testable output.

Initial local checks:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -p xtask -- doctor --full
```

The repo doctor is the source-derived guardrail for contract drift and known silent-failure
families. See [`docs/specs/doctor-contract.md`](docs/specs/doctor-contract.md).
