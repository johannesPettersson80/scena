use serde_json::Value;

use scena::{AssetPath, Color, SceneHostAssetImportReportV1, SceneHostCore, Transform, Vec3};

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
        AssetPath::from("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    ))?;
    let import_report: SceneHostAssetImportReportV1 = serde_json::from_str(&import_json)?;
    let mesh = host.node_handle(import_report.import, "ColoredTriangle")?;

    host.set_node_tint(mesh, Some(Color::from_linear_rgba(0.1, 0.8, 0.2, 0.45)))?;
    host.set_node_annotation("mesh-label", mesh, [0.0, 0.0, 0.0])?;
    host.frame_node(mesh)?;
    host.prepare()?;
    host.render()?;

    print_json("capability_report", host.capabilities_json()?)?;
    print_json("scene_host_asset_import", import_json)?;
    print_json("scene_inspection", host.inspect_json()?)?;
    print_json("annotation_projection", host.annotation_projections_json()?)?;
    print_json("capture", host.capture_json()?)?;

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
