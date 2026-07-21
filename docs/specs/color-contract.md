# Color contract

Status: active rendering contract

glTF color textures and authored display colors use sRGB encoding. Lighting,
PBR math, interpolation, blending, and environment contribution operate in
linear space. Output conversion is applied exactly once according to the
target format and renderer color-management settings.

The attachment format, rather than the presence of a post-processing pass,
selects that final conversion:

- shaders write linear RGB to `Rgba8UnormSrgb`/`Bgra8UnormSrgb`, and the
  attachment performs the linear-to-sRGB transfer;
- shaders explicitly encode linear RGB before writing an sRGB-byte output
  contract to `Rgba8Unorm`/`Bgra8Unorm`;
- alpha is linear in both cases;
- renderer-owned RGBA8 readback always contains sRGB display bytes for sRGB
  output, regardless of whether the storage attachment itself is `*Srgb`.

The SDR post chain uses `Rgba8UnormSrgb` intermediates. Their physical storage
is encoded, while shader sampling automatically returns linear values, so
blending, bloom, FXAA, SSAO, reflections, and depth-of-field operate before the
single final transfer. This does not add HDR headroom; an HDR scene target is a
separate renderer project.

Background clears, mesh output, labels, and strokes follow the same target
rule. Toggling post-processing must not reinterpret an otherwise identical
RGBA8 result. The browser `color-transfer-no-post` and `color-transfer-post`
workflows pin linear `0.18` to sRGB byte `118` (within two byte values) on
WebGPU and WebGL2.

Asset decode records the semantic role of each texture so data textures such
as normals, metallic-roughness, occlusion, and transmission are never decoded
as sRGB color. CPU, WebGPU, and WebGL2 paths use the same transfer and alpha
semantics.

The asset-aware preparation entry point, `pub fn prepare_with_assets<F>`,
resolves material texture roles before rendering. No hidden color-space guess
or first-time asset fetch belongs in `render()`.
