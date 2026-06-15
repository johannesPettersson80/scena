use scena::{
    AssetPath, Color, PRODUCT_OPTIONS_SCHEMA_V1, ProductOptionGroupV1, ProductOptionV1,
    ProductOptionsV1, SceneHostCameraState, SceneHostCore, Transform, Vec3,
    VisualPatchMaterialVariantV1, VisualPatchTintV1, VisualPatchV1, VisualPatchVisibilityV1,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut host = SceneHostCore::headless(160, 120)?;
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/material_variants_scene.gltf",
    )))?;
    let mesh = host.node_handle(import, "VariantTriangle")?;
    let accessory = host.add_empty(
        Some(host.root_handle()),
        Transform::at(Vec3::new(0.4, 0.0, 0.0)),
        Some("accessory"),
    )?;

    host.store_product_options(ProductOptionsV1 {
        schema: PRODUCT_OPTIONS_SCHEMA_V1.to_owned(),
        groups: vec![
            ProductOptionGroupV1 {
                id: "finish".to_owned(),
                label: "Finish".to_owned(),
                active: None,
                options: vec![ProductOptionV1 {
                    id: "noon-green".to_owned(),
                    label: "Noon green".to_owned(),
                    patch: VisualPatchV1 {
                        camera: Some(SceneHostCameraState {
                            target: Vec3::ZERO,
                            distance: 3.0,
                            yaw_radians: 0.7,
                            pitch_radians: 0.2,
                        }),
                        material_variants: vec![VisualPatchMaterialVariantV1 {
                            import,
                            variant: Some("noon".to_owned()),
                        }],
                        tints: vec![VisualPatchTintV1 {
                            node: mesh,
                            tint: Some(Color::from_linear_rgba(0.2, 0.8, 0.4, 1.0)),
                        }],
                        ..VisualPatchV1::default()
                    },
                    metadata: None,
                }],
            },
            ProductOptionGroupV1 {
                id: "accessory".to_owned(),
                label: "Accessory".to_owned(),
                active: None,
                options: vec![ProductOptionV1 {
                    id: "hidden".to_owned(),
                    label: "Hidden".to_owned(),
                    patch: VisualPatchV1 {
                        visibility: vec![VisualPatchVisibilityV1 {
                            node: accessory,
                            visible: false,
                        }],
                        ..VisualPatchV1::default()
                    },
                    metadata: None,
                }],
            },
        ],
    })?;

    let finish_result = host.apply_product_option_json("finish", "noon-green")?;
    let accessory_result = host.apply_product_option_json("accessory", "hidden")?;
    println!("{finish_result}");
    println!("{accessory_result}");
    println!("{}", host.product_options_json()?);

    Ok(())
}
