use std::fs;

#[test]
fn production_asset_profile_enables_compressed_asset_decoders_without_default_bloat() {
    let manifest = fs::read_to_string("Cargo.toml").expect("workspace manifest is readable");

    assert!(
        manifest.contains("default = []"),
        "production asset work must not silently expand default features"
    );
    assert!(
        manifest.contains("production-assets = [\"ktx2\", \"meshopt\"]"),
        "Cargo.toml must expose a named production-assets feature for compressed glTF assets"
    );
}
