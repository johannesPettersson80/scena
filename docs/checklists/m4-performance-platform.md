# M4 performance and platform acceptance

Status: active evidence index

The `m4_performance_platform` test surface produces typed platform evidence.
Browser smoke is recorded in `m4-platform-browser-smoke.json`; WASM size is
recorded in `m4-wasm-size.json`, including `brotli_q11_bytes`. Doctor registers
the contract as `ARCH-M4-PLATFORM`.

These files are evidence only when their provenance and source commit match the
release bundle. Performance conclusions require measured distributions rather
than the presence of an artifact name.
