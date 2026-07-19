# M1 geometry and materials acceptance

Status: active evidence index

- [x] Geometry/material contracts are exercised by the native M1 suite.
- [x] `m1_browser_rendered_output` records Rust/WASM browser rendered-output proof.
- [x] The browser lighting/clipping command is
  `node tests/browser/m2_browser_lighting_clipping_smoke.js` and writes
  `m2-browser-lighting-clipping-smoke.json`.
- [x] Doctor owns these browser assertions through `VISUAL-BROWSER-M2`.

Checkboxes name implemented test surfaces. Release acceptance still requires
the exact-commit typed artifacts defined by the release gates.
