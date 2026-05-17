# Public Demo Showcase Checklist

Updated: 2026-05-17

Goal: the public Cloudflare demo must screenshot as a product-quality renderer
showcase for authored connector mating in Rust, not as a browser test harness.

## Local Visual Verdict

- [x] Connector snap is the default first render.
- [x] The tagline is:
  "Three.js ergonomics, Rust types, running in your browser. Drop a model or snap authored connectors."
- [x] Connector snap remains first/sidebar-featured.
- [x] `Replay snap` is visible for connector mode.
- [x] Replay moves the drive unit toward the load unit along the solved mate axis.
- [x] The first connector frame is assembled and lit, not a mid-replay scattered frame.
- [x] Khronos samples remain secondary compatibility checks.
- [x] Weak/random public samples are not present.

## Code And Diagnostics

- [x] Code panel is a static synced display, not an editor.
- [x] Connector mode shows `scene.mate(&drive, "shaft", &load, "hub")?;`.
- [x] Khronos/drop mode shows matching `load_scene(...)`, instantiate, camera/frame/orbit code.
- [x] Code panel is verified against every sidebar mode by `scripts/probe_cloudflare_demo.js`.
- [x] Frame, asset bytes, load phase, and orbit are under collapsed Diagnostics.
- [x] Diagnostics are closed by default on desktop and mobile.
- [x] Public demo console is quiet without `?perf=1` / `?timing=1`.

## README Hero GIF

- [x] Connector snap GIF exists at `docs/assets/readme/connector-snap.gif`.
- [x] README places the GIF before the static screenshots.
- [x] Static screenshots remain below as supporting proof.
- [x] Representative coalesced GIF frames were inspected:
  - `target/gate-artifacts/readme-gif/regenerated-000.png`
  - `target/gate-artifacts/readme-gif/regenerated-040.png`
  - `target/gate-artifacts/readme-gif/regenerated-065.png`
  - `target/gate-artifacts/readme-gif/regenerated-079.png`
- [ ] Follow-up: `cargo run --example connector_snap_hero` produced frame files but did not exit cleanly in this checkout and was stopped; the GIF overlay step completed from the generated frame set.

## Local Browser Proof

Build:

```bash
wasm-pack build --release --target web --out-dir demo/pkg . --features demo-page
```

Probe:

```bash
node scripts/probe_cloudflare_demo.js http://127.0.0.1:18104/index.html
```

Verified locally on 2026-05-17 with headless Chromium `/usr/bin/chromium`.

Artifacts:

- `target/gate-artifacts/cloudflare-demo/connector-snap-page.png`
- `target/gate-artifacts/cloudflare-demo/connector-snap-canvas.png`
- `target/gate-artifacts/cloudflare-demo/connector-snap-replay-page.png`
- `target/gate-artifacts/cloudflare-demo/drive-unit-page.png`
- `target/gate-artifacts/cloudflare-demo/load-unit-page.png`
- `target/gate-artifacts/cloudflare-demo/water-bottle-page.png`
- `target/gate-artifacts/cloudflare-demo/water-bottle-canvas.png`
- `target/gate-artifacts/cloudflare-demo/toy-car-page.png`
- `target/gate-artifacts/cloudflare-demo/connector-snap-mobile-page.png`

## Local Gates

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo run --example mate_two_parts`
- [x] `wasm-pack build --release --target web --out-dir demo/pkg . --features demo-page`
- [x] `node scripts/probe_cloudflare_demo.js http://127.0.0.1:18104/index.html`
- [x] `cargo test`
- [x] `cargo run -p xtask -- doctor --full`
- [x] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`

## Production Follow-Through

- [ ] Commit the demo/README fixes.
- [ ] Push branch.
- [ ] Confirm Cloudflare preview deployment.
- [ ] Confirm production alias `https://scena-demo.pages.dev/` serves corrected files.
- [ ] Run probe against the production alias.
- [ ] Capture production desktop screenshot.
- [ ] Capture production mobile screenshot.
- [ ] Confirm production console has no red errors.
- [ ] Keep repo clean after commit/push except any explicitly user-owned pre-existing work.
- [ ] Monitor GitHub CI for the pushed commit.
- [ ] Confirm CI size gate on GitHub; local `wasm-pack` is not final size proof.
