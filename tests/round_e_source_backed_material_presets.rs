use scena::{Assets, TextureColorSpace};

#[test]
fn source_backed_material_presets_load_texture_backed_surfaces() {
    pollster::block_on(async {
        let assets = Assets::new();
        let presets = assets.material_presets();

        let satin = presets.satin().await.expect("satin preset loads");
        let leather = presets.leather().await.expect("leather preset loads");
        let rubber = presets.rubber().await.expect("rubber preset loads");

        for (name, material) in [("satin", satin), ("leather", leather), ("rubber", rubber)] {
            let desc = assets
                .try_material(material)
                .unwrap_or_else(|error| panic!("{name} material handle resolves: {error:?}"));
            let base_color = desc
                .base_color_texture()
                .unwrap_or_else(|| panic!("{name} preset must bind a base-color texture"));
            let normal = desc
                .normal_texture()
                .unwrap_or_else(|| panic!("{name} preset must bind a normal texture"));
            let roughness = desc
                .metallic_roughness_texture()
                .unwrap_or_else(|| panic!("{name} preset must bind an ORM/roughness texture"));

            assert_eq!(
                assets
                    .try_texture(base_color)
                    .expect("base texture resolves")
                    .color_space(),
                TextureColorSpace::Srgb
            );
            for handle in [normal, roughness] {
                let texture = assets
                    .try_texture(handle)
                    .unwrap_or_else(|error| panic!("{name} texture handle resolves: {error:?}"));
                assert_eq!(texture.color_space(), TextureColorSpace::Linear);
                assert!(
                    texture.has_decoded_pixels(),
                    "{name} texture must be decoded from bundled source bytes"
                );
            }
        }
    });
}

#[test]
fn source_backed_leather_preset_uses_warm_leather_tint() {
    pollster::block_on(async {
        let assets = Assets::new();
        let leather = assets
            .material_presets()
            .leather()
            .await
            .expect("leather preset loads");
        let desc = assets
            .try_material(leather)
            .expect("leather material handle resolves");
        let color = desc.base_color();

        assert!(
            color.r > color.g && color.g > color.b,
            "source-backed leather must multiply its pale texture with a warm brown leather tint, got {color:?}"
        );
        assert!(
            color.a >= 0.99,
            "source-backed leather tint must remain opaque, got {color:?}"
        );
    });
}
