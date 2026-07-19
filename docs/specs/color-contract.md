# Color contract

Status: active rendering contract

glTF color textures and authored display colors use sRGB encoding. Lighting,
PBR math, interpolation, blending, and environment contribution operate in
linear space. Output conversion is applied exactly once according to the
target format and renderer color-management settings.

Asset decode records the semantic role of each texture so data textures such
as normals, metallic-roughness, occlusion, and transmission are never decoded
as sRGB color. CPU, WebGPU, and WebGL2 paths use the same transfer and alpha
semantics.

The asset-aware preparation entry point, `pub fn prepare_with_assets<F>`,
resolves material texture roles before rendering. No hidden color-space guess
or first-time asset fetch belongs in `render()`.
