use crate::app::prelude::*;

pub(crate) fn check_m3a_gltf_scene_contracts(root: &Path, findings: &mut Vec<Finding>) {
    // Stage C2: GLB framing now comes from the `gltf` crate, so scena
    // no longer maintains its own glb.rs / accessor.rs / read.rs
    // modules. The doctor contract instead pins that the new
    // gltf-crate-backed parser modules exist and reference the crate.
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/external.rs",
        &[
            "pub(super) fn external_buffer_paths",
            "resolve_relative_path",
            "!uri.starts_with(\"data:\")",
            "::gltf::Gltf",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/meshes.rs",
        &[
            "parse_meshes",
            "primitive.reader",
            "read_positions",
            "Semantic::Positions",
            "normalize_i16_vec3",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/materials.rs",
        &[
            "parse_materials",
            "validate_material_texture_indices",
            "TextureTransform::new",
            "TextureColorSpace::Srgb",
            "KHR_materials_unlit",
            "KHR_materials_emissive_strength",
            "KHR_texture_transform",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/textures.rs",
        &[
            "parse_textures",
            "ImageSource::Uri",
            "ImageSource::View",
            "from_gltf_sampler",
            "WrappingMode",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/material.rs",
        &[
            "pub struct TextureTransform",
            "pub const fn base_color_texture_transform",
            "pub const fn with_base_color_texture_transform",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/geometry.rs",
        &["try_new_with_vertex_colors", "pub fn vertex_colors"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene.rs",
        &[
            "mod import;",
            "mod instances;",
            "mod labels;",
            "mod materials;",
            "mod picking;",
            "mod view;",
            "ImportOptions",
            "InstanceSetKey",
            "LabelKey",
            "SceneImport",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/materials.rs",
        &[
            "pub fn set_mesh_material",
            "NodeIsNotMesh",
            "structure_revision",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/picking.rs",
        &[
            "pub fn pick(",
            "pickable_renderables",
            "pick_scene",
            "pub fn interaction(&self)",
            "pub fn interaction_mut(&mut self)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/instances.rs",
        &[
            "pub struct InstanceId",
            "pub enum InstanceCullingPolicy",
            "CpuBoundingBoxFallback",
            "pub struct InstanceSet",
            "pub fn add_instance_set",
            "pub fn reserve_instances",
            "pub fn push_instance",
            "pub fn remove_instance",
            "pub fn clear_instances",
            "pub fn instances(&self)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/labels.rs",
        &[
            "pub struct LabelDesc",
            "pub enum LabelRasterization",
            "pub enum LabelBillboard",
            "pub fn sdf",
            "pub fn msdf",
            "pub fn add_label",
            "pub fn set_label_text",
            "LabelNotFound",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/picking.rs",
        &[
            "pub struct CursorPosition",
            "pub struct Viewport",
            "pub enum HitTarget",
            "pub struct Hit",
            "pub struct InteractionContext",
            "pub struct InteractionStyle",
            "set_hover",
            "set_primary_selection",
            "pub(crate) const fn revision",
            "pub(crate) fn pick_scene",
            "HitTarget::Node",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/render/prepare.rs",
        &[
            "scene.instance_set_nodes()",
            "labels::append_label_primitives",
            "compose_transform",
            "instance_set.geometry()",
            "instance_set.material()",
        ],
    );
}
