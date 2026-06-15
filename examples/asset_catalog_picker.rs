use std::path::PathBuf;

use serde_json::json;

use scena::{AssetCatalogV1, AssetPath, Assets, SceneHostCore, render_asset_catalog_preview_png};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/gate-artifacts/asset-catalog-picker"));
    std::fs::create_dir_all(&out_dir)?;

    let catalog: AssetCatalogV1 = serde_json::from_str(include_str!(
        "../tests/assets/catalog/readiness_catalog.v1.json"
    ))?;
    let readiness = pollster::block_on(Assets::new().validate_asset_catalog(&catalog));
    let selected_report = readiness
        .assets
        .iter()
        .find(|asset| asset.ok)
        .ok_or("catalog did not contain a ready asset")?;
    let selected = catalog
        .assets
        .iter()
        .find(|asset| asset.id == selected_report.id)
        .ok_or("readiness report referenced an unknown asset")?;

    let preview = pollster::block_on(render_asset_catalog_preview_png(selected))?;
    let preview_path = out_dir.join(format!("{}.preview.png", preview.asset_id));
    std::fs::write(&preview_path, &preview.png_bytes)?;

    let mut host = SceneHostCore::headless(256, 256)?;
    pollster::block_on(
        host.instantiate_url_with_report_json(AssetPath::from(selected.source.clone())),
    )?;
    host.frame_all()?;
    host.prepare()?;
    host.render()?;
    let capture_path = out_dir.join(format!("{}.scenehost.png", preview.asset_id));
    std::fs::write(&capture_path, host.capture_png_bytes()?)?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "selected_asset": selected.id,
            "source": selected.source,
            "preview_png": preview_path,
            "preview_fnv1a64": preview.png_fnv1a64,
            "scenehost_png": capture_path,
        }))?
    );
    Ok(())
}
