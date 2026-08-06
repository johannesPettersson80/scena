#![cfg(all(feature = "scene-host", feature = "material-library"))]

use std::collections::BTreeSet;
use std::io::{Cursor, Write};
use std::process::Command;

use slotmap::Key as _;

use scena::{
    Assets, MATERIAL_LIBRARY_CATALOG_SCHEMA_V1, PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V1,
    PhotoContourQualityMetricsV1, PhotoGroundingQualityMetricsV1, PhotoMaterialQualityMetricsV1,
    PhotoProjectedTextureDensityV1, PhotoQualityAnalysisReportV1, PhotographicMaterialCategoryV1,
    PhotographicMaterialPackMapRoleV1, PhotographicMaterialResolutionV1, PhotographicSurfaceKind,
    compile_photographic_material_archive, compile_photographic_material_archive_at_resolution,
    photographic_material_catalog_v1, photographic_material_catalog_v2,
    select_photographic_material_resolution,
};

#[test]
fn photographic_material_catalog_exposes_density_selected_1k_2k_and_4k_variants() {
    let catalog = photographic_material_catalog_v2();
    let entry = catalog
        .entries
        .iter()
        .find(|entry| entry.provider_asset_id == "Metal049A")
        .expect("fixture material is present");

    assert_eq!(
        entry
            .archive_variants
            .iter()
            .map(|variant| variant.resolution)
            .collect::<Vec<_>>(),
        vec![
            PhotographicMaterialResolutionV1::OneK,
            PhotographicMaterialResolutionV1::TwoK,
            PhotographicMaterialResolutionV1::FourK,
        ]
    );
    for variant in &entry.archive_variants {
        assert!(
            variant.archive_uri.ends_with(&format!(
                "_{}-JPG.zip",
                variant.resolution.ambientcg_token()
            )),
            "catalog URI must match its resolution: {variant:#?}"
        );
    }

    assert_eq!(
        select_photographic_material_resolution(1.0),
        Some(PhotographicMaterialResolutionV1::OneK)
    );
    assert_eq!(
        select_photographic_material_resolution(0.999),
        Some(PhotographicMaterialResolutionV1::TwoK)
    );
    assert_eq!(
        select_photographic_material_resolution(0.5),
        Some(PhotographicMaterialResolutionV1::TwoK)
    );
    assert_eq!(
        select_photographic_material_resolution(0.499),
        Some(PhotographicMaterialResolutionV1::FourK)
    );
    assert_eq!(select_photographic_material_resolution(f64::NAN), None);
}

#[test]
fn photographic_material_catalog_is_large_source_backed_and_product_focused() {
    let catalog = photographic_material_catalog_v1();

    assert_eq!(catalog.schema, MATERIAL_LIBRARY_CATALOG_SCHEMA_V1);
    assert!(
        catalog.entries.len() >= 300,
        "the built-in product/industrial surface catalog must expose the complete audited ambientCG families; count={}",
        catalog.entries.len()
    );
    assert!(
        catalog
            .entries
            .iter()
            .filter(|entry| entry.creation_method == "surface-photometric-stereo")
            .count()
            >= 16,
        "the catalog must include a substantial captured-material set, not only procedural surfaces"
    );

    let mut ids = BTreeSet::new();
    let categories = catalog
        .entries
        .iter()
        .map(|entry| entry.category)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        categories,
        BTreeSet::from([
            PhotographicMaterialCategoryV1::Fabric,
            PhotographicMaterialCategoryV1::Leather,
            PhotographicMaterialCategoryV1::Metal,
            PhotographicMaterialCategoryV1::Plastic,
            PhotographicMaterialCategoryV1::Rubber,
        ])
    );

    for entry in &catalog.entries {
        assert!(
            ids.insert(entry.id.as_str()),
            "catalog ids must be unique: {}",
            entry.id
        );
        assert_eq!(entry.provider, "ambientcg");
        assert_eq!(entry.license, "CC0-1.0");
        assert!(
            matches!(
                entry.creation_method.as_str(),
                "surface-fully-procedural"
                    | "surface-approximated"
                    | "surface-photogrammetry"
                    | "surface-photometric-stereo"
            ),
            "creation method must preserve ambientCG API provenance: {entry:#?}"
        );
        assert!(
            entry.source_page.starts_with("https://ambientcg.com/a/"),
            "source page must be explicit and HTTPS: {entry:#?}"
        );
        assert!(
            entry
                .archive_uri
                .starts_with("https://ambientcg.com/get?file=")
                && entry.archive_uri.ends_with("_1K-JPG.zip"),
            "archive must be the canonical 1K JPG PBR pack: {entry:#?}"
        );
        assert!(
            entry.recommended_tile_size_m.is_finite() && entry.recommended_tile_size_m > 0.0,
            "every entry needs a usable physical mapping recommendation: {entry:#?}"
        );
        assert!(
            entry.maps.iter().any(|map| map.as_str() == "base_color")
                && entry.maps.iter().any(|map| map.as_str() == "normal_gl")
                && entry.maps.iter().any(|map| map.as_str() == "roughness"),
            "every catalog material needs the minimum PBR map set: {entry:#?}"
        );
    }

    let smooth_aluminium = catalog
        .entries
        .iter()
        .find(|entry| entry.provider_asset_id == "Metal050A")
        .expect("Metal050A is in the curated catalog");
    assert_eq!(
        smooth_aluminium.surface_kind,
        PhotographicSurfaceKind::PolishedMetal,
        "Metal050A's measured 0.055 mean roughness is polished, not satin"
    );
}

#[test]
fn materials_list_cli_filters_without_network_access() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "materials",
            "list",
            "--category",
            "metal",
            "--query",
            "clean",
        ])
        .output()
        .expect("scena materials list runs");
    assert!(
        output.status.success(),
        "materials list failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("materials list emits JSON");
    assert_eq!(report["schema"], "scena.material_library_catalog.v1");
    let entries = report["entries"].as_array().expect("catalog entries");
    assert!(
        entries.len() >= 4,
        "the clean-metal query should still offer useful choice: {report:#}"
    );
    assert!(
        entries.iter().all(|entry| entry["category"] == "metal"
            && (entry["label"]
                .as_str()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .contains("clean")
                || entry["tags"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|tag| tag == "clean"))),
        "the CLI must apply both filters deterministically: {report:#}"
    );
}

#[test]
fn material_archive_compiler_locks_source_and_emits_canonical_pbr_maps() {
    let entry = photographic_material_catalog_v1()
        .entries
        .into_iter()
        .find(|entry| entry.provider_asset_id == "Metal049A")
        .expect("fixture catalog entry");
    let archive = material_archive_fixture();
    let output_dir = unique_temp_dir("compile");

    let pack = compile_photographic_material_archive(&entry, &archive, &output_dir)
        .expect("valid ambientCG-style material archive compiles");

    assert_eq!(pack.schema, PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V1);
    assert_eq!(pack.id, entry.id);
    assert_eq!(pack.source.archive_bytes, archive.len() as u64);
    assert_eq!(pack.source.archive_sha256.len(), 64);
    assert_eq!(pack.source.license, "CC0-1.0");
    assert_eq!(pack.maps.len(), 3);
    for role in [
        PhotographicMaterialPackMapRoleV1::BaseColor,
        PhotographicMaterialPackMapRoleV1::NormalGl,
        PhotographicMaterialPackMapRoleV1::OcclusionRoughnessMetallic,
    ] {
        let map = pack
            .maps
            .iter()
            .find(|map| map.role == role)
            .unwrap_or_else(|| panic!("compiled pack is missing {role:?}: {pack:#?}"));
        assert_eq!(map.sha256.len(), 64);
        assert!(output_dir.join(&map.path).is_file());
    }

    let orm = image::open(output_dir.join("occlusion-roughness-metallic.png"))
        .expect("packed ORM opens")
        .to_rgba8();
    assert_eq!(orm.dimensions(), (2, 2));
    assert_eq!(
        orm.get_pixel(0, 0).0,
        [255, 64, 192, 255],
        "ORM must pack AO=white, roughness=G, metalness=B"
    );

    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(output_dir.join("scena-material-pack.json"))
            .expect("material pack manifest exists"),
    )
    .expect("material pack manifest is JSON");
    assert_eq!(manifest["schema"], PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V1);
    assert!(
        !output_dir
            .parent()
            .expect("output has parent")
            .join("escape.png")
            .exists(),
        "ZIP paths must never be extracted outside the pack"
    );

    let assets = Assets::new();
    let loaded = pollster::block_on(
        assets.load_photographic_material_pack(output_dir.join("scena-material-pack.json")),
    )
    .expect("compiled material pack loads through Assets");
    assert_eq!(
        loaded.pack().source.archive_sha256,
        pack.source.archive_sha256
    );
    assert!(assets.contains_material(loaded.material()));
    let material = assets
        .try_material(loaded.material())
        .expect("loaded pack material resolves");
    assert_eq!(
        material.photographic_surface_tile_size_m(),
        Some(entry.recommended_tile_size_m)
    );
    assert!(material.base_color_texture().is_some());
    assert!(material.normal_texture().is_some());
    assert!(material.metallic_roughness_texture().is_some());
    assert_eq!(
        material.metallic_roughness_texture(),
        material.occlusion_texture(),
        "the packed ORM texture must serve both glTF material slots"
    );

    std::fs::remove_dir_all(output_dir).expect("test output cleans up");
}

#[test]
fn materials_import_and_cached_fetch_cli_compile_and_reuse_source_locked_pack() {
    let root = unique_temp_dir("import-cli");
    std::fs::create_dir_all(&root).expect("test root creates");
    let archive_path = root.join("Metal049A_1K-JPG.zip");
    std::fs::write(&archive_path, material_archive_fixture()).expect("fixture archive writes");
    let output_dir = root.join("pack");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "materials",
            "import",
            "ambientcg-metal049a",
            archive_path.to_str().expect("archive path is UTF-8"),
            "--out",
            output_dir.to_str().expect("output path is UTF-8"),
        ])
        .output()
        .expect("scena materials import runs");
    assert!(
        output.status.success(),
        "materials import failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let pack: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("materials import emits JSON");
    assert_eq!(pack["schema"], "scena.photographic_material_pack.v1");
    assert_eq!(pack["id"], "ambientcg-metal049a");
    assert!(output_dir.join("scena-material-pack.json").is_file());

    let cached_fetch = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "materials",
            "fetch",
            "ambientcg-metal049a",
            "--out",
            output_dir.to_str().expect("output path is UTF-8"),
        ])
        .env("HTTPS_PROXY", "http://127.0.0.1:9")
        .env("https_proxy", "http://127.0.0.1:9")
        .env("ALL_PROXY", "http://127.0.0.1:9")
        .env("all_proxy", "http://127.0.0.1:9")
        .env("NO_PROXY", "")
        .env("no_proxy", "")
        .output()
        .expect("scena materials fetch cache probe runs");
    assert!(
        cached_fetch.status.success(),
        "a validated existing pack must satisfy fetch without network: stdout={} stderr={}",
        String::from_utf8_lossy(&cached_fetch.stdout),
        String::from_utf8_lossy(&cached_fetch.stderr)
    );
    let cached_pack: serde_json::Value =
        serde_json::from_slice(&cached_fetch.stdout).expect("materials fetch emits JSON");
    assert_eq!(cached_pack["schema"], "scena.photographic_material_pack.v1");
    assert_eq!(
        cached_pack["source"]["archive_sha256"],
        pack["source"]["archive_sha256"]
    );

    let rejected_dir = root.join("rejected");
    let rejected = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "materials",
            "import",
            "ambientcg-metal049a",
            archive_path.to_str().expect("archive path is UTF-8"),
            "--out",
            rejected_dir.to_str().expect("output path is UTF-8"),
            "--expect-sha256",
            "0000000000000000000000000000000000000000000000000000000000000000",
        ])
        .output()
        .expect("scena materials import checksum guard runs");
    assert!(!rejected.status.success());
    assert!(
        !rejected_dir.exists(),
        "checksum mismatch must fail before publishing a material pack"
    );

    std::fs::remove_dir_all(root).expect("test root cleans up");
}

#[test]
fn materials_import_resolution_emits_v2_pack_in_resolution_specific_cache() {
    let root = unique_temp_dir("resolution-import-cli");
    std::fs::create_dir_all(&root).expect("test root creates");
    let archive_path = root.join("Metal049A_2K-JPG.zip");
    std::fs::write(
        &archive_path,
        material_archive_fixture_at_resolution(
            PhotographicMaterialResolutionV1::TwoK,
            [180, 184, 190],
        ),
    )
    .expect("2K fixture archive writes");
    let cache_root = root.join("cache");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "materials",
            "import",
            "ambientcg-metal049a",
            archive_path.to_str().expect("archive path is UTF-8"),
            "--resolution",
            "2k",
        ])
        .env("XDG_CACHE_HOME", &cache_root)
        .output()
        .expect("resolution-aware materials import runs");
    assert!(
        output.status.success(),
        "2K materials import failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let pack: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("materials import emits JSON");
    assert_eq!(pack["schema"], "scena.photographic_material_pack.v2");
    assert_eq!(pack["resolution"], "2k");
    assert!(
        pack["maps"].as_array().is_some_and(|maps| {
            maps.iter()
                .all(|map| map["width"] == 2048 && map["height"] == 2048)
        }),
        "a 2K pack must contain actual 2048px maps: {pack:#}"
    );

    let manifest_path =
        cache_root.join("scena/materials/ambientcg-metal049a/2k/scena-material-pack.json");
    assert!(
        manifest_path.is_file(),
        "resolution is part of the default material cache key"
    );
    let assets = Assets::new();
    let loaded =
        pollster::block_on(assets.load_photographic_material_pack(manifest_path.as_path()))
            .expect("v2 material pack loads through Assets");
    assert_eq!(loaded.resolution(), PhotographicMaterialResolutionV1::TwoK);

    let cached_fetch = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "materials",
            "fetch",
            "ambientcg-metal049a",
            "--resolution",
            "2k",
        ])
        .env("XDG_CACHE_HOME", &cache_root)
        .env("HTTP_PROXY", "http://127.0.0.1:9")
        .env("http_proxy", "http://127.0.0.1:9")
        .env("HTTPS_PROXY", "http://127.0.0.1:9")
        .env("https_proxy", "http://127.0.0.1:9")
        .env("ALL_PROXY", "http://127.0.0.1:9")
        .env("all_proxy", "http://127.0.0.1:9")
        .env("NO_PROXY", "")
        .env("no_proxy", "")
        .output()
        .expect("resolution-aware cached materials fetch runs");
    assert!(
        cached_fetch.status.success(),
        "cached 2K fetch failed: stdout={} stderr={}",
        String::from_utf8_lossy(&cached_fetch.stdout),
        String::from_utf8_lossy(&cached_fetch.stderr)
    );
    let cached_pack: serde_json::Value =
        serde_json::from_slice(&cached_fetch.stdout).expect("cached materials fetch emits JSON");
    assert_eq!(cached_pack["schema"], "scena.photographic_material_pack.v2");
    assert_eq!(cached_pack["resolution"], "2k");

    std::fs::remove_dir_all(root).expect("test root cleans up");
}

#[test]
fn visible_texture_density_rebinds_a_1k_material_to_the_required_2k_variant() {
    let root = unique_temp_dir("visible-density-selection");
    std::fs::create_dir_all(&root).expect("test root creates");
    let entry = photographic_material_catalog_v2()
        .entries
        .into_iter()
        .find(|entry| entry.provider_asset_id == "Metal049A")
        .expect("fixture material is present");
    let family = root.join("material");
    let one_k = family.join("1k");
    let two_k = family.join("2k");
    compile_photographic_material_archive_at_resolution(
        &entry,
        PhotographicMaterialResolutionV1::OneK,
        &material_archive_fixture_at_resolution(
            PhotographicMaterialResolutionV1::OneK,
            [170, 174, 180],
        ),
        &one_k,
    )
    .expect("1K fixture pack compiles");
    compile_photographic_material_archive_at_resolution(
        &entry,
        PhotographicMaterialResolutionV1::TwoK,
        &material_archive_fixture_at_resolution(
            PhotographicMaterialResolutionV1::TwoK,
            [176, 180, 186],
        ),
        &two_k,
    )
    .expect("2K fixture pack compiles");

    let recipe_path = root.join("scene.recipe.json");
    let recipe = serde_json::json!({
        "schema": "scena.scene_recipe.v1",
        "geometries": [{
            "id": "body_geo",
            "primitive": { "kind": "box", "size": [0.4, 0.2, 0.3] }
        }],
        "materials": [{
            "id": "body_mat",
            "material_pack": {
                "uri": "material/1k/scena-material-pack.json",
                "tile_size_m": 0.25
            }
        }],
        "nodes": [{
            "id": "body",
            "geometry": "body_geo",
            "material": "body_mat"
        }]
    });
    let recipe_text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");
    std::fs::write(&recipe_path, &recipe_text).expect("recipe writes");
    let policy = scena::RecipeBuildPolicy::testing()
        .with_allowed_root(root.canonicalize().expect("test root canonicalizes"));
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        recipe_path.to_string_lossy(),
        &recipe_text,
        policy,
    ))
    .unwrap_or_else(|manifest| panic!("material scene builds: {manifest:#?}"));
    let mut host = build.host;
    let before = host.scene().inspect_with_assets(host.assets()).draw_list()[0].material();
    assert_eq!(
        host.assets()
            .try_texture(
                host.assets()
                    .try_material(before)
                    .expect("1K material resolves")
                    .base_color_texture()
                    .expect("1K material has a base-color map")
            )
            .expect("1K texture resolves")
            .decoded_dimensions(),
        Some((1024, 1024))
    );

    let analysis = density_analysis(before.data().as_ffi(), 0.75);
    let selection = pollster::block_on(
        host.select_photographic_material_resolutions(&analysis, 64 * 1024 * 1024),
    )
    .expect("visible-density material selection succeeds");
    assert_eq!(selection.selections.len(), 1, "{selection:#?}");
    assert_eq!(
        selection.selections[0].selected_resolution,
        PhotographicMaterialResolutionV1::TwoK
    );
    assert!(selection.selections[0].changed);

    let after = host.scene().inspect_with_assets(host.assets()).draw_list()[0].material();
    assert_ne!(after, before);
    assert_eq!(
        host.assets()
            .try_texture(
                host.assets()
                    .try_material(after)
                    .expect("2K material resolves")
                    .base_color_texture()
                    .expect("2K material has a base-color map")
            )
            .expect("2K texture resolves")
            .decoded_dimensions(),
        Some((2048, 2048))
    );

    std::fs::remove_dir_all(root).expect("test root cleans up");
}

#[test]
fn scene_recipe_material_pack_builds_and_checks_source_lock() {
    let root = unique_temp_dir("recipe");
    std::fs::create_dir_all(&root).expect("test root creates");
    let entry = photographic_material_catalog_v1()
        .entries
        .into_iter()
        .find(|entry| entry.provider_asset_id == "Metal049A")
        .expect("fixture catalog entry");
    let pack_dir = root.join("pack");
    let pack =
        compile_photographic_material_archive(&entry, &material_archive_fixture(), &pack_dir)
            .expect("fixture material pack compiles");
    let recipe_path = root.join("scene.recipe.json");
    let recipe = serde_json::json!({
        "schema": "scena.scene_recipe.v1",
        "geometries": [{
            "id": "body_geo",
            "primitive": { "kind": "box", "size": [0.2, 0.1, 0.16] }
        }],
        "materials": [{
            "id": "body_mat",
            "material_pack": {
                "uri": "pack/scena-material-pack.json",
                "expected_archive_sha256": pack.source.archive_sha256
            }
        }],
        "nodes": [{
            "id": "body",
            "geometry": "body_geo",
            "material": "body_mat"
        }]
    });
    let recipe_text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");
    std::fs::write(&recipe_path, &recipe_text).expect("recipe writes");

    let validation = scena::validate_scene_recipe_json(&recipe_text);
    assert!(
        validation.ok,
        "material-pack recipe validates: {validation:#?}"
    );
    let policy = scena::RecipeBuildPolicy::testing()
        .with_allowed_root(root.canonicalize().expect("test root canonicalizes"));
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        recipe_path.to_string_lossy(),
        &recipe_text,
        policy.clone(),
    ))
    .unwrap_or_else(|manifest| panic!("material-pack recipe builds: {manifest:#?}"));
    assert!(build.manifest.ok, "{:#?}", build.manifest);
    assert!(
        build
            .manifest
            .materials
            .iter()
            .any(|material| material.id == "body_mat"),
        "the pack-backed material must be present in the build manifest"
    );

    let mut bad_recipe = recipe;
    bad_recipe["materials"][0]["material_pack"]["expected_archive_sha256"] =
        serde_json::Value::String(
            "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        );
    let bad_text = serde_json::to_string_pretty(&bad_recipe).expect("bad recipe serializes");
    let failure = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        recipe_path.to_string_lossy(),
        &bad_text,
        policy,
    ))
    .expect_err("a mismatched material source lock must fail");
    assert!(
        failure
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "material_pack_source_sha256_mismatch"),
        "source lock mismatch must be explicit: {failure:#?}"
    );

    std::fs::remove_dir_all(root).expect("test root cleans up");
}

#[test]
fn imported_scene_can_use_source_locked_material_pack() {
    let root = unique_temp_dir("import-recipe");
    std::fs::create_dir_all(&root).expect("test root creates");
    let entry = photographic_material_catalog_v1()
        .entries
        .into_iter()
        .find(|entry| entry.provider_asset_id == "Metal049A")
        .expect("fixture catalog entry");
    let pack_dir = root.join("pack");
    let pack =
        compile_photographic_material_archive(&entry, &material_archive_fixture(), &pack_dir)
            .expect("fixture material pack compiles");
    std::fs::copy(
        "tests/assets/gltf/cad_plate_drawing_scene.gltf",
        root.join("part.gltf"),
    )
    .expect("self-contained glTF fixture copies");
    let recipe_path = root.join("scene.recipe.json");
    let recipe = serde_json::json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "part",
            "uri": "part.gltf",
            "material": {
                "material_pack": {
                    "uri": "pack/scena-material-pack.json",
                    "expected_archive_sha256": pack.source.archive_sha256,
                    "tile_size_m": 0.18
                },
                "base_color": "#AAB7C5",
                "normal_scale": 0.35,
                "occlusion_strength": 0.60,
                "double_sided": true
            }
        }]
    });
    let recipe_text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");
    std::fs::write(&recipe_path, &recipe_text).expect("recipe writes");

    let validation = scena::validate_scene_recipe_json(&recipe_text);
    assert!(
        validation.ok,
        "import material pack recipe validates: {validation:#?}"
    );
    let policy = scena::RecipeBuildPolicy::testing()
        .with_allowed_root(root.canonicalize().expect("test root canonicalizes"));
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        recipe_path.to_string_lossy(),
        &recipe_text,
        policy,
    ))
    .unwrap_or_else(|manifest| panic!("import material pack recipe builds: {manifest:#?}"));
    let inspection = build.host.scene().inspect_with_assets(build.host.assets());
    assert!(
        !inspection.draw_list().is_empty(),
        "imported fixture must remain drawable"
    );
    for draw in inspection.draw_list() {
        let material = build
            .host
            .assets()
            .try_material(draw.material())
            .expect("import pack material resolves");
        assert!(material.base_color_texture().is_some());
        assert!(material.normal_texture().is_some());
        assert!(material.metallic_roughness_texture().is_some());
        assert_eq!(material.photographic_surface_tile_size_m(), Some(0.18));
        assert_eq!(material.normal_scale(), 0.35);
        assert_eq!(material.occlusion_strength(), 0.60);
        assert!(material.double_sided());
    }

    std::fs::remove_dir_all(root).expect("test root cleans up");
}

#[test]
fn imported_scene_can_bind_material_packs_by_locked_source_material_identity() {
    let root = unique_temp_dir("import-material-bindings");
    std::fs::create_dir_all(&root).expect("test root creates");
    let entry = photographic_material_catalog_v1()
        .entries
        .into_iter()
        .find(|entry| entry.provider_asset_id == "Metal049A")
        .expect("fixture catalog entry");
    let pack_dir = root.join("pack");
    let pack =
        compile_photographic_material_archive(&entry, &material_archive_fixture(), &pack_dir)
            .expect("fixture material pack compiles");
    std::fs::copy("tests/assets/gltf/drive_unit.glb", root.join("part.glb"))
        .expect("self-contained multi-material GLB fixture copies");
    let recipe_path = root.join("scene.recipe.json");
    let recipe = serde_json::json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "part",
            "uri": "part.glb",
            "material_bindings": [
                {
                    "source_material": { "index": 0, "name": "baseplate steel" },
                    "material": {
                        "material_pack": {
                            "uri": "pack/scena-material-pack.json",
                            "expected_archive_sha256": pack.source.archive_sha256,
                            "tile_size_m": 0.18
                        }
                    }
                },
                {
                    "source_material": { "index": 3, "name": "navy powder coat" },
                    "material": {
                        "material_pack": {
                            "uri": "pack/scena-material-pack.json",
                            "expected_archive_sha256": pack.source.archive_sha256,
                            "tile_size_m": 0.27
                        },
                        "base_color": "#18345A"
                    }
                }
            ]
        }]
    });
    let recipe_text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");
    std::fs::write(&recipe_path, &recipe_text).expect("recipe writes");

    let validation = scena::validate_scene_recipe_json(&recipe_text);
    assert!(
        validation.ok,
        "source-material binding recipe validates: {validation:#?}"
    );
    let policy = scena::RecipeBuildPolicy::testing()
        .with_allowed_root(root.canonicalize().expect("test root canonicalizes"));
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        recipe_path.to_string_lossy(),
        &recipe_text,
        policy.clone(),
    ))
    .unwrap_or_else(|manifest| panic!("source-material binding recipe builds: {manifest:#?}"));
    let inspection = build.host.scene().inspect_with_assets(build.host.assets());
    let mut first_binding_draws = 0;
    let mut second_binding_draws = 0;
    let mut untouched_source_draws = 0;
    for draw in inspection.draw_list() {
        let material = build
            .host
            .assets()
            .try_material(draw.material())
            .expect("draw material resolves");
        match material.photographic_surface_tile_size_m() {
            Some(tile_size) if (tile_size - 0.18).abs() < f32::EPSILON => {
                first_binding_draws += 1;
            }
            Some(tile_size) if (tile_size - 0.27).abs() < f32::EPSILON => {
                second_binding_draws += 1;
            }
            _ if build
                .host
                .assets()
                .material_source(draw.material())
                .is_some_and(|source| {
                    source.kind() == scena::AssetMaterialSourceKind::SourceMaterial
                }) =>
            {
                untouched_source_draws += 1;
            }
            _ => {}
        }
    }
    assert!(first_binding_draws > 0, "material index 0 must be rebound");
    assert!(second_binding_draws > 0, "material index 3 must be rebound");
    assert!(
        untouched_source_draws > 0,
        "source materials without a binding must remain untouched"
    );

    let mut mismatched = recipe;
    mismatched["imports"][0]["material_bindings"][0]["source_material"]["name"] =
        serde_json::Value::String("wrong source name".to_string());
    let mismatched_text =
        serde_json::to_string_pretty(&mismatched).expect("mismatched recipe serializes");
    let failure = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        recipe_path.to_string_lossy(),
        &mismatched_text,
        policy,
    ))
    .expect_err("a source material identity mismatch must fail closed");
    assert!(
        failure
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "source_material_identity_mismatch"),
        "source material mismatch must be explicit: {failure:#?}"
    );

    std::fs::remove_dir_all(root).expect("test root cleans up");
}

fn material_archive_fixture() -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut archive = zip::ZipWriter::new(cursor);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in [
        (
            "nested/Metal049A_1K-JPG_Color.png",
            solid_rgb_png([180, 184, 190]),
        ),
        (
            "nested/Metal049A_1K-JPG_NormalGL.png",
            solid_rgb_png([128, 128, 255]),
        ),
        (
            "nested/Metal049A_1K-JPG_Roughness.png",
            solid_rgb_png([64, 64, 64]),
        ),
        (
            "nested/Metal049A_1K-JPG_Metalness.png",
            solid_rgb_png([192, 192, 192]),
        ),
        ("../escape.png", solid_rgb_png([255, 0, 0])),
    ] {
        archive
            .start_file(name, options)
            .expect("fixture ZIP starts file");
        archive.write_all(&bytes).expect("fixture ZIP writes file");
    }
    archive.finish().expect("fixture ZIP finishes").into_inner()
}

fn material_archive_fixture_at_resolution(
    resolution: PhotographicMaterialResolutionV1,
    base_color: [u8; 3],
) -> Vec<u8> {
    let dimension = resolution.dimension_px();
    let token = resolution.ambientcg_token();
    let cursor = Cursor::new(Vec::new());
    let mut archive = zip::ZipWriter::new(cursor);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, color) in [
        (format!("Metal049A_{token}-JPG_Color.png"), base_color),
        (
            format!("Metal049A_{token}-JPG_NormalGL.png"),
            [128, 128, 255],
        ),
        (format!("Metal049A_{token}-JPG_Roughness.png"), [64, 64, 64]),
        (
            format!("Metal049A_{token}-JPG_Metalness.png"),
            [192, 192, 192],
        ),
    ] {
        archive
            .start_file(name, options)
            .expect("resolution fixture ZIP starts file");
        archive
            .write_all(&solid_rgb_png_with_dimension(color, dimension))
            .expect("resolution fixture ZIP writes file");
    }
    archive
        .finish()
        .expect("resolution fixture ZIP finishes")
        .into_inner()
}

fn density_analysis(
    material_handle: u64,
    texels_per_pixel_p50: f64,
) -> PhotoQualityAnalysisReportV1 {
    PhotoQualityAnalysisReportV1 {
        schema: scena::PHOTO_QUALITY_ANALYSIS_SCHEMA_V1.to_owned(),
        mode: "report_only".to_owned(),
        identity_source: "same_pass_beauty_semantic".to_owned(),
        materials: vec![PhotoMaterialQualityMetricsV1 {
            material_handle,
            material_kind: "pbr_metallic_roughness".to_owned(),
            material_class: "smooth_metal".to_owned(),
            material_class_basis: "effective_surface".to_owned(),
            metallic_factor: 1.0,
            roughness_factor: 0.2,
            effective_metallic_mean: 1.0,
            effective_roughness_mean: 0.2,
            surface_texture_min_dimension_px: Some(1024),
            surface_tile_size_m: Some(0.25),
            sample_count: 1_000,
            interior_sample_count: 900,
            reflection_structure_rms_srgb8: Some(4.0),
            luminance_p99_srgb8: 220.0,
            near_white_fraction: 0.001,
            clipped_fraction: 0.0,
            projected_texture_density: Some(PhotoProjectedTextureDensityV1 {
                method: "beauty_identity_linear_depth_physical_tile".to_owned(),
                sample_count: 1_000,
                texels_per_pixel_p10: texels_per_pixel_p50 * 0.9,
                texels_per_pixel_p50,
                texels_per_pixel_p90: texels_per_pixel_p50 * 1.1,
            }),
        }],
        grounding: PhotoGroundingQualityMetricsV1 {
            method: "same_pass_subject_support_boundary".to_owned(),
            boundary_sample_count: 0,
            contact_shadow_delta_mean_srgb8: None,
            attached_fraction: None,
            contact_shadow_confirmed: false,
        },
        contour: PhotoContourQualityMetricsV1 {
            method: "semantic_silhouette_row_extents".to_owned(),
            boundary_sample_count: 0,
            curved_turn_diversity: None,
        },
        unavailable_metrics: Vec::new(),
    }
}

fn solid_rgb_png(color: [u8; 3]) -> Vec<u8> {
    solid_rgb_png_with_dimension(color, 2)
}

fn solid_rgb_png_with_dimension(color: [u8; 3], dimension: u32) -> Vec<u8> {
    let image = image::RgbImage::from_pixel(dimension, dimension, image::Rgb(color));
    let mut bytes = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .expect("fixture PNG encodes");
    bytes.into_inner()
}

fn unique_temp_dir(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "scena-material-library-{label}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    path
}
