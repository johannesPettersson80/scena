use crate::app::prelude::*;

pub(crate) fn check_m3a_render_view_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/render/prepare/labels.rs",
        &[
            "pub(super) fn append_label_primitives",
            "scene.label_nodes()",
            "LabelBillboard::ScreenAligned",
            "Primitive::triangle",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/render.rs",
        &[
            "mod offscreen;",
            "hover_style: InteractionStyle",
            "selection_style: InteractionStyle",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/render/offscreen.rs",
        &[
            "pub struct OffscreenTarget",
            "pub struct PixelReadback",
            "pub fn offscreen",
            "pub fn read_pixels",
            "pub fn into_rgba8",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/render/settings.rs",
        &["pub fn set_hover_style", "pub fn set_selection_style"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/view.rs",
        &[
            "pub fn camera_node",
            "pub fn frame(&mut self, camera: CameraKey, bounds: Aabb)",
            "pub fn frame_all",
            "pub fn frame_node",
            "pub fn look_at(&mut self, camera: CameraKey, target: NodeKey)",
            "DepthRange::fit_sphere",
            "set_node_transform_and_mark_changed",
        ],
    );
}
