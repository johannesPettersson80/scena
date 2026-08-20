use super::*;

#[test]
fn bundled_studio_environment_does_not_overbake_its_128x64_source() {
    let assets = Assets::new();
    let handle = assets
        .bundled_studio_environment()
        .expect("bundled studio HDR decodes");
    let environment = assets
        .try_environment(handle)
        .expect("bundled studio environment remains available");

    assert_eq!(environment.source_dimensions(), Some((128, 64)));
    assert_eq!(
        environment.cubemap_resolution(),
        64,
        "the bundled low-resolution source must retain its explicit 64-face bake"
    );
}

#[test]
fn bundled_final_studio_environment_uses_checked_2k_source_and_512_faces() {
    let assets = Assets::new();
    let handle = assets
        .bundled_final_studio_environment()
        .expect("bundled final studio HDR decodes");
    let environment = assets
        .try_environment(handle)
        .expect("bundled final studio environment remains available");

    assert_eq!(environment.source_dimensions(), Some((2048, 1024)));
    assert_eq!(
        environment.source_sha256(),
        Some("6e677b7421f4a14f0844dece04243c4ab3f4bf1a05bf4bb79e29368b3ecc7746")
    );
    assert_eq!(
        include_str!("../../tests/assets/environment/polyhaven/studio_small_08_2k.provenance.json"),
        concat!(
            "{\n",
            "  \"asset\": \"Studio Small 08\",\n",
            "  \"source_url\": \"https://polyhaven.com/a/studio_small_08\",\n",
            "  \"download_url\": \"https://dl.polyhaven.org/file/ph-assets/HDRIs/hdr/2k/studio_small_08_2k.hdr\",\n",
            "  \"license\": \"CC0-1.0\",\n",
            "  \"sha256\": \"6e677b7421f4a14f0844dece04243c4ab3f4bf1a05bf4bb79e29368b3ecc7746\",\n",
            "  \"size_bytes\": 5930381\n",
            "}\n"
        )
    );
    assert_eq!(
        environment.cubemap_resolution(),
        512,
        "final product stills require enough directional resolution for smooth metal",
    );
}

#[test]
fn pf04_snapshot_cache_replacement_preserves_old_view_and_exposes_fresh_content() {
    let assets = Assets::new();
    let path = AssetPath::from("memory://snapshot-cache.png");
    let descriptor = TextureDesc::new_with_bytes(
        path,
        TextureColorSpace::Srgb,
        TextureSamplerDesc::default(),
        TextureSourceFormat::Png,
        None,
    )
    .expect("deferred PNG descriptor creates");
    assert!(!descriptor.has_decoded_pixels());
    let handle = assets.storage().textures.insert(Arc::new(descriptor));
    let old = assets
        .texture_snapshot(handle)
        .expect("old snapshot resolves");

    let source = include_bytes!("../../tests/assets/gltf/khronos/TextureTransformTest/Correct.png");
    Arc::make_mut(
        assets
            .storage()
            .textures
            .get_mut(handle)
            .expect("cached texture remains live"),
    )
    .decode_missing_pixels_from_bytes(Some(source))
    .expect("cached texture decodes replacement pixels");
    let fresh = assets
        .texture_snapshot(handle)
        .expect("fresh snapshot resolves");

    assert!(!old.has_decoded_pixels());
    assert!(fresh.has_decoded_pixels());
    assert!(!Arc::ptr_eq(&old, &fresh));
}
