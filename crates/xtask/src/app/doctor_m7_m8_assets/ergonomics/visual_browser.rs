use crate::app::prelude::*;

pub(crate) fn check_m7_visual_browser_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/render_nodes.rs",
        &[
            "self.world_transform(node_key)",
            "map(|transform| (renderable, transform))",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/render_nodes.rs",
        &["Some((renderable, node.transform))"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene.rs",
        &[
            ".world_transform(key)",
            "map(|transform| (key, mesh, transform))",
            "self.world_transform(node_key)",
            "map(|transform| (node_key, instance_set, transform))",
            "map(|transform| (node_key, label, label_desc, transform))",
            "map(|transform| (node_key, light_key, light, transform))",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene.rs",
        &[
            "Some((key, mesh, node.transform))",
            "map(|instance_set| (node_key, instance_set, node.transform))",
            "map(|label_desc| (node_key, label, label_desc, node.transform))",
            "map(|light| (node_key, light_key, light, node.transform))",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "tests/m7_visual_proof.rs",
        &["Primitive::unlit_triangle()", "add_renderable("],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "tests/assets/gltf/connector_zup_scene.gltf",
        &["z-up-mount", "0.70710677"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "tests/m7_visual_proof.rs",
        &[
            "m7_headless_visual_artifacts_cover_ergonomics_workflows",
            "target/gate-artifacts/m7-visual",
            "m7-first-render",
            "m7-first-glb",
            "m7-camera-frame",
            "m7-picking-selection",
            "m7-helpers",
            "m7-labels",
            "m7-controls",
            "m7-layers-helper-on-top",
            "m7-static-batching",
            "m7-anchor-alignment",
            "m7-connector-before",
            "m7-connector-after",
            "connector before/after proof",
            "m7-coordinate-units",
            "m7-industrial-static-scene",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/browser_probe/workflows/ergonomics.rs",
        &[
            "camera-framing",
            "anchor-alignment",
            "connector-before",
            "connector-after",
            "ConnectorFrame::new",
            "ConnectOptions::default",
            "coordinate-units",
            "static-batching",
            "layers-helper-on-top",
            "beginner-diagnostics",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &[
            "camera-framing",
            "anchor-alignment",
            "connector-before",
            "connector-after",
            "connector before/after workflow",
            "coordinate-units",
            "static-batching",
            "layers-helper-on-top",
            "beginner-diagnostics",
        ],
    );
}
