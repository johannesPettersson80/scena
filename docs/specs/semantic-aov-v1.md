# Semantic AOV v1 contract

Status: accepted cross-backend contract for FR06. CPU, native/headless GPU,
WebGPU, and WebGL2 implementations exist; release acceptance still requires
the recorded native and real-hardware browser parity lanes described below.

## Ownership and lifecycle

- The renderer owns rasterization from its already prepared scene state. AOV
  capture never fetches assets, traverses an unprepared scene as a substitute,
  or hides prepare/render/GPU work.
- `SceneHost` maps renderer-local `NodeKey`/`InstanceId` attribution to public
  host identity. A host handle is valid only for the lifetime/generation of the
  host that issued it. Reports call this `runtime_scoped`; callers must use
  recipe/build-manifest IDs for persistence.
- CPU v1 is available on deterministic `headless` and requires a current
  successful `prepare()`. GPU hosts opt in before prepare with
  `set_semantic_aov_capture_enabled(true)` (or
  `RendererOptions::with_semantic_aov_capture(true)`), which lifecycle-owns
  three RGBA8 targets, depth, pipelines, and readback resources. Capture never
  allocates those resources inside render.
- Native/headless GPU uses explicit texture-to-buffer readback. Browser WebGPU
  maps the same buffers asynchronously. WebGL2 renders each byte-oriented AOV
  texture through an sRGB-compensated surface blit and reads the preserved
  canvas because WebGL buffer mapping is not a portable contract.

## Identity and palette

- One ID represents a visible scene node plus an optional authored instance.
  Primitive/triangle identity is intentionally not encoded in v1: all opaque
  triangles for the same `(node, instance)` share one palette entry.
- Palette index `0`, RGBA `[0, 0, 0, 0]`, means background/no attributed hit.
  Hit entries use deterministic, collision-free 24-bit palette indices encoded
  little-endian into RGB with alpha `255`. The legend records both index and
  exact RGBA.
- Legend order is deterministic by internal node key then optional instance ID,
  independent of visibility/triangle traversal order. Host handles in the
  legend are runtime-scoped, never persistence identifiers.
- Recipe-facing output enriches each runtime legend entry with the persistent
  authored node/instance ID from the build manifest when one exists. Imported
  nodes use the persistent import ID plus source node path.

## Visibility semantics

- AOVs cover prepared opaque and alpha-masked triangle geometry. For alpha
  masks, fragments discarded by the prepared material coverage remain
  background. Nearest surviving opaque fragment wins using the same camera,
  clipping planes, section box, front-face/double-sided rule, and occlusion
  ordering as normal geometry rendering.
- Alpha-blended/OIT geometry and physical transmission are excluded in v1.
  They never overwrite opaque attribution and are counted explicitly.
- Strokes (line/wireframe/edge), labels, particles, helpers, and screen-space
  overlays are excluded/background and counted explicitly by available owner.
  No excluded surface is silently attributed to an underlying helper node.
- CPU v1 uses a single center sample per output pixel. It does not apply MSAA,
  supersampling, reconstruction, FXAA, post-processing, or color-edge resolve.
  Therefore semantic edges can differ from anti-aliased beauty output.

## Depth and normals

- The raw depth buffer stores positive linear camera/view distance in scene meters
  (`f32`). Background is positive infinity. Perspective interpolation
  is reciprocal-depth correct; orthographic interpolation is linear.
- The portable depth PNG reserves `0` for background and maps the active
  camera near/far interval linearly to unsigned 16-bit values `1..65535`.
  The report records near, far, units, background value, and mapping.
- Raw normals are normalized world-space geometric vertex normals. Background
  is `[0, 0, 0]`. The portable normal PNG encodes `normal * 0.5 + 0.5` in RGB,
  with alpha `255` for hits and `[0, 0, 0, 0]` for background.
- GPU targets pack the 24-bit ID and normalized near/far linear depth into
  RGBA8 so the exact contract is portable across WebGPU and WebGL2 MRT limits;
  the public capture decodes both back to the raw `u32`/`f32` representation.

## Determinism and evidence

- Identical prepared state, camera, viewport, and backend must produce
  byte-identical repeated ID/depth/normal captures and legend JSON.
- Acceptance fixtures cover overlapping opaque nodes, transparent foreground
  exclusion, depth occlusion, distinct authored instances, background, and
  world-normal encoding. The native focused test compares GPU center truth to
  CPU. `npm run browser:fr06-semantic-aov` compares WebGPU and WebGL2 masks,
  identity, depth, and normals and writes
  `scena.fr06_semantic_aov_browser_proof.v1`. Software adapters are functional
  proof only; the GPU checklist item closes only with native and browser output
  from the required real-GPU lane.
