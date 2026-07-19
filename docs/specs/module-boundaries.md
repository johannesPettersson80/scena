# Module boundaries

Status: active architecture contract

The canonical ownership map is:

- `scene`: graph state, transforms, cameras, lights, imports, and revisions.
- `assets`: fetching, parsing, decode, cache, reload, and asset provenance.
- `geometry`: geometry descriptors and primitive construction.
- `material`: material descriptors and authored color/material state.
- `render`: preparation, backend resources, drawing, output, and readback.
- `animation`: clip sampling and animation state contracts.
- `controls`: reusable renderer-facing camera controls.
- `picking`: renderer-scene intersection and pick result contracts.
- `diagnostics`: typed errors, capability reports, and statistics.
- `platform`: native/browser host adapters only.
- `vocabulary`: stable public enumerations shared by schemas, the CLI, and API consumers.

No hidden asset fetch, shader compile, or first-time GPU upload inside `render()`
is permitted. Resource work is explicit in prepare or a separately named
output-preparation operation.

## Host-owned convenience facade exceptions

`HeadlessGltfViewer` and `InteractiveGltfViewer` are the v1.0 host-owned convenience
facade exceptions. They compose `Scene`, `Assets`, and `Renderer`; they do not
move ownership out of those modules. Mutable accessors remain explicit escape hatches
and must preserve lifecycle invalidation.

These are the only current host-owned convenience facade exceptions.

## Large module allowlist

The architecture scanner recognizes the historically large facade owners
`src/assets.rs` and `src/viewer.rs` only through an explicit reviewed policy.
New catch-all owners or size exemptions require a documented architecture
decision; file movement alone does not establish ownership.
