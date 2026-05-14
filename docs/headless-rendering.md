# Headless rendering

Headless rendering is useful when you need image output without a visible
window.

Use it for:

- CI snapshots,
- generated documentation images,
- regression tests,
- server-side previews,
- deterministic visual checks.

## Example

Run the bundled example:

```bash
cargo run --example headless_ci
```

The example shows the complete lifecycle: create scene, prepare renderer, render
the frame, and write output.

## Basic pattern

```rust
let mut renderer = scena::Renderer::headless(1280, 720)?;
renderer.prepare_with_assets(&mut scene, &assets)?;
renderer.render_active(&scene)?;
```

The exact readback or file-writing helper depends on the output workflow. Start
with `examples/headless_ci.rs` for a runnable reference.

## Why use headless rendering

Headless output is deterministic and easy to automate. It is well suited for:

- checking that an imported asset is visible,
- verifying camera framing,
- detecting blank renders,
- comparing generated image artifacts,
- producing images for documentation.

## CPU and GPU paths

`Renderer::headless` is the deterministic headless path. GPU-capable headless
paths are used when adapter availability and platform support allow it.

Applications should use capability reports and renderer metadata when they need
to distinguish CPU, native GPU, WebGPU, and WebGL2 output.

## Common mistakes

- Rendering before calling `prepare()`.
- Forgetting to set an active camera.
- Placing the model outside the camera frustum.
- Loading an asset but not instantiating it into the scene.
- Mutating the scene after preparation without preparing again.

See [Troubleshooting](troubleshooting.md).
