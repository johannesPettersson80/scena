#![cfg(not(target_arch = "wasm32"))]

use std::io::Cursor;

use scena::{AssetCatalogPreviewError, AssetCatalogV1, render_asset_catalog_preview_png};

#[test]
fn generated_asset_catalog_preview_is_deterministic_png() {
    let catalog: AssetCatalogV1 =
        serde_json::from_str(include_str!("assets/catalog/readiness_catalog.v1.json"))
            .expect("catalog fixture deserializes");
    let asset = catalog
        .assets
        .iter()
        .find(|asset| asset.id == "variant-triangle")
        .expect("generated-preview asset exists");

    let first = pollster::block_on(render_asset_catalog_preview_png(asset))
        .expect("first preview render succeeds");
    let second = pollster::block_on(render_asset_catalog_preview_png(asset))
        .expect("second preview render succeeds");

    assert_eq!(first.asset_id, "variant-triangle");
    assert_eq!(
        first.source,
        "tests/assets/gltf/material_variants_scene.gltf"
    );
    assert_eq!(first.width, 256);
    assert_eq!(first.height, 256);
    assert_eq!(first.png_fnv1a64, second.png_fnv1a64);
    assert_eq!(first.png_bytes, second.png_bytes);

    let decoded = decode_png_rgba8(&first.png_bytes);
    assert_eq!(decoded.width, 256);
    assert_eq!(decoded.height, 256);
    assert!(
        decoded
            .rgba8
            .chunks_exact(4)
            .any(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0),
        "generated catalog preview must contain visible asset pixels"
    );
}

#[test]
fn asset_catalog_preview_generation_rejects_image_only_entries() {
    let mut catalog: AssetCatalogV1 =
        serde_json::from_str(include_str!("assets/catalog/readiness_catalog.v1.json"))
            .expect("catalog fixture deserializes");
    let asset = catalog
        .assets
        .iter_mut()
        .find(|asset| asset.id == "variant-triangle")
        .expect("generated-preview asset exists");
    let preview = asset.preview.as_mut().expect("preview exists");
    preview.kind = "image".to_owned();
    preview.path = Some("previews/variant-triangle.png".to_owned());

    let error = pollster::block_on(render_asset_catalog_preview_png(asset))
        .expect_err("image previews are declared assets, not generated renders");

    assert_eq!(
        error,
        AssetCatalogPreviewError::UnsupportedPreviewKind {
            kind: "image".to_owned()
        }
    );
}

#[derive(Debug)]
struct DecodedPng {
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
}

fn decode_png_rgba8(bytes: &[u8]) -> DecodedPng {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().expect("PNG header reads");
    let mut buffer = vec![
        0;
        reader
            .output_buffer_size()
            .expect("PNG output buffer size is known")
    ];
    let info = reader.next_frame(&mut buffer).expect("PNG payload reads");
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    DecodedPng {
        width: info.width,
        height: info.height,
        rgba8: buffer[..info.buffer_size()].to_vec(),
    }
}
