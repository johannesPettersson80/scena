use super::AssetPath;

pub(crate) const AGENT_TEMPLATE_MATERIAL_VARIANTS_URI: &str =
    "scena://bundled/agent-template/material_variants_scene.gltf";
pub(crate) const AGENT_TEMPLATE_ANIMATED_TRIANGLE_URI: &str =
    "scena://bundled/agent-template/animated_triangle_scene.glb";
pub(crate) const AGENT_TEMPLATE_CAD_PLATE_URI: &str =
    "scena://bundled/agent-template/cad_plate_drawing_scene.gltf";

const MATERIAL_VARIANTS_BYTES: &[u8] =
    include_bytes!("../../tests/assets/gltf/material_variants_scene.gltf");
const ANIMATED_TRIANGLE_BYTES: &[u8] =
    include_bytes!("../../tests/assets/gltf/animated_triangle_scene.glb");
const CAD_PLATE_BYTES: &[u8] =
    include_bytes!("../../tests/assets/gltf/cad_plate_drawing_scene.gltf");

pub(crate) fn bundled_scene_bytes(path: &AssetPath) -> Option<&'static [u8]> {
    match path.as_str() {
        AGENT_TEMPLATE_MATERIAL_VARIANTS_URI => Some(MATERIAL_VARIANTS_BYTES),
        AGENT_TEMPLATE_ANIMATED_TRIANGLE_URI => Some(ANIMATED_TRIANGLE_BYTES),
        AGENT_TEMPLATE_CAD_PLATE_URI => Some(CAD_PLATE_BYTES),
        _ => None,
    }
}

pub(crate) fn is_bundled_scene_uri(uri: &str) -> bool {
    matches!(
        uri,
        AGENT_TEMPLATE_MATERIAL_VARIANTS_URI
            | AGENT_TEMPLATE_ANIMATED_TRIANGLE_URI
            | AGENT_TEMPLATE_CAD_PLATE_URI
    )
}
