use serde_json::Value;

use scena::{
    AssetPath, Color, SceneHostAssetImportReportV1, SceneHostCameraState, SceneHostCore, Transform,
    Vec3, VisualPatchLabelTargetV1, VisualPatchLabelV1, VisualPatchMaterialVariantV1,
    VisualPatchSelectionV1, VisualPatchTransformV1, VisualPatchV1, VisualPatchVisibilityV1,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut host = SceneHostCore::headless(128, 128)?;
    let root = host.root_handle();
    let group = host.add_empty(
        Some(root),
        Transform::at(Vec3::new(-0.25, 0.0, 0.0)),
        Some("group:left"),
    )?;

    let import_json = pollster::block_on(host.instantiate_url_under_with_report_json(
        group,
        AssetPath::from("tests/assets/gltf/material_variants_scene.gltf"),
    ))?;
    let import_report: SceneHostAssetImportReportV1 = serde_json::from_str(&import_json)?;
    let mesh = host.node_handle(import_report.import, "VariantTriangle")?;
    let hidden_marker = host.add_empty(Some(root), Transform::IDENTITY, Some("hidden-marker"))?;

    let patch_result = host.apply_patch(&VisualPatchV1 {
        transforms: vec![VisualPatchTransformV1 {
            node: group,
            transform: Transform::at(Vec3::new(-0.2, 0.0, 0.0)),
        }],
        tints: vec![scena::VisualPatchTintV1 {
            node: mesh,
            tint: Some(Color::from_linear_rgba(0.1, 0.8, 0.2, 0.45)),
        }],
        visibility: vec![VisualPatchVisibilityV1 {
            node: hidden_marker,
            visible: false,
        }],
        camera: Some(SceneHostCameraState {
            target: Vec3::new(0.0, 0.0, 0.0),
            distance: 3.0,
            yaw_radians: 0.7,
            pitch_radians: 0.4,
        }),
        selection: Some(VisualPatchSelectionV1 { node: Some(mesh) }),
        material_variants: vec![VisualPatchMaterialVariantV1 {
            import: import_report.import,
            variant: Some("noon".to_owned()),
        }],
        labels: vec![VisualPatchLabelV1 {
            id: "mesh-label".to_owned(),
            target: VisualPatchLabelTargetV1::Node {
                node: mesh,
                local_offset: [0.0, 0.0, 0.0],
            },
        }],
        metadata: Some(serde_json::json!({ "example": "scene_host_contracts" })),
        echo_metadata: true,
        ..VisualPatchV1::default()
    })?;
    host.frame_node(mesh)?;
    host.prepare()?;
    host.render()?;
    let _ = host.pick(64.0, 64.0)?;

    print_json("capability_report", host.capabilities_json()?)?;
    print_json("scene_host_asset_import", import_json)?;
    println!(
        "visual_patch_result\n{}",
        serde_json::to_string_pretty(&patch_result)?
    );
    print_json("scene_inspection", host.inspect_json()?)?;
    print_json("annotation_projection", host.annotation_projections_json()?)?;
    print_json("capture", host.capture_json()?)?;
    print_json("host_event", host.drain_events_json()?)?;

    let asset = pollster::block_on(
        host.assets()
            .load_scene("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )?;
    println!(
        "asset_provenance\n{}",
        serde_json::to_string_pretty(asset.provenance())?
    );

    Ok(())
}

fn print_json(label: &str, json_text: impl AsRef<str>) -> Result<(), serde_json::Error> {
    let value: Value = serde_json::from_str(json_text.as_ref())?;
    println!("{label}\n{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
