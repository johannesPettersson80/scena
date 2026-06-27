use crate::app::prelude::*;

pub(crate) fn check_m3a_render_view_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/render/prepare/labels.rs",
        &[
            "pub(super) fn prepare_label_atlas",
            "scene.label_nodes()",
            "LabelBillboard::ScreenAligned",
            "PreparedLabelAtlas",
            "PreparedLabelQuad",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/render.rs",
        &["mod offscreen;", "pub struct Renderer"],
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
