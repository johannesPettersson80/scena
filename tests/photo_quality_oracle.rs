#![cfg(feature = "scene-host")]

use scena::{
    CaptureProjection, PhotoQualityAnalysisInputV1, SceneHostSemanticAovLegendEntryV1,
    analyze_photo_quality,
};
use sha2::{Digest, Sha256};
use std::fs;

const WIDTH: u32 = 64;
const HEIGHT: u32 = 64;
const SUBJECT_ID: u32 = 1;
const SUPPORT_ID: u32 = 2;

#[test]
fn photo_quality_oracle_controls_separate_known_failures_by_twenty_percent() {
    let legend = [
        SceneHostSemanticAovLegendEntryV1 {
            palette_index: SUBJECT_ID,
            rgba8: [1, 0, 0, 255],
            node_handle: 10,
            material_handle: Some(100),
            material_kind: Some("pbr_metallic_roughness".to_owned()),
            metallic_factor: Some(1.0),
            roughness_factor: Some(0.12),
            effective_metallic_mean: Some(1.0),
            effective_roughness_mean: Some(0.12),
            surface_texture_min_dimension_px: None,
            surface_tile_size_m: None,
            instance_handle: None,
            instance_id: None,
        },
        SceneHostSemanticAovLegendEntryV1 {
            palette_index: SUPPORT_ID,
            rgba8: [2, 0, 0, 255],
            node_handle: 20,
            material_handle: Some(200),
            material_kind: Some("pbr_metallic_roughness".to_owned()),
            metallic_factor: Some(0.0),
            roughness_factor: Some(0.9),
            effective_metallic_mean: Some(0.0),
            effective_roughness_mean: Some(0.9),
            surface_texture_min_dimension_px: None,
            surface_tile_size_m: None,
            instance_handle: None,
            instance_id: None,
        },
    ];

    let structured = control_frame(Shape::Circle, true, true);
    let flat = control_frame(Shape::Circle, false, true);
    let detached = control_frame(Shape::Circle, true, false);
    let faceted = control_frame(Shape::Octagon, true, true);
    let depth_meters = vec![1.0; (WIDTH * HEIGHT) as usize];
    let projection = Some(CaptureProjection::Perspective {
        vertical_fov_radians: 1.0,
        aspect: WIDTH as f32 / HEIGHT as f32,
        near: 0.1,
        far: 10.0,
    });

    let analyze = |control: &(Vec<u8>, Vec<u32>)| {
        analyze_photo_quality(PhotoQualityAnalysisInputV1 {
            width: WIDTH,
            height: HEIGHT,
            rgba8: &control.0,
            beauty_id_indices: &control.1,
            depth_meters: &depth_meters,
            projection,
            legend: &legend,
            subject_handles: &[10],
            support_handles: &[20],
        })
        .expect("synthetic same-pass control is valid")
    };
    let structured_report = analyze(&structured);
    let flat_report = analyze(&flat);
    let detached_report = analyze(&detached);
    let faceted_report = analyze(&faceted);
    let mut rough_metal_legend = legend.clone();
    rough_metal_legend[0].roughness_factor = Some(0.8);
    rough_metal_legend[0].effective_roughness_mean = Some(0.8);
    let rough_metal_report = analyze_photo_quality(PhotoQualityAnalysisInputV1 {
        width: WIDTH,
        height: HEIGHT,
        rgba8: &structured.0,
        beauty_id_indices: &structured.1,
        depth_meters: &depth_meters,
        projection,
        legend: &rough_metal_legend,
        subject_handles: &[10],
        support_handles: &[20],
    })
    .expect("rough metallic same-pass control is valid");

    assert_eq!(structured_report.mode, "report_only");
    assert_eq!(
        structured_report.identity_source,
        "same_pass_beauty_semantic"
    );

    let structured_reflection = structured_report.materials[0]
        .reflection_structure_rms_srgb8
        .expect("smooth metal reflection structure is measured");
    let flat_reflection = flat_report.materials[0]
        .reflection_structure_rms_srgb8
        .expect("flat smooth metal control is measured");
    assert!(
        structured_reflection >= flat_reflection * 1.20,
        "structured metal must separate from flat metal by at least 20%: structured={structured_reflection} flat={flat_reflection}",
    );
    assert!(
        rough_metal_report.materials[0].interior_sample_count > 0,
        "rough metallic surfaces still need interior samples for reflection-content calibration"
    );
    assert!(
        rough_metal_report.materials[0]
            .reflection_structure_rms_srgb8
            .is_some(),
        "rough metallic reflection structure must not silently report unavailable"
    );

    let attached = structured_report
        .grounding
        .contact_shadow_delta_mean_srgb8
        .expect("attached control has a measured support boundary");
    let absent = detached_report
        .grounding
        .contact_shadow_delta_mean_srgb8
        .expect("detached-shadow control has the same support boundary");
    assert!(
        attached >= absent * 1.20 + 1.0,
        "attached contact must separate from absent darkening by at least 20%: attached={attached} absent={absent}",
    );
    assert!(
        structured_report.grounding.contact_shadow_confirmed,
        "the attached same-pass control must confirm contact"
    );
    assert!(
        !detached_report.grounding.contact_shadow_confirmed,
        "the detached same-pass control must not confirm contact"
    );

    let smooth_contour = structured_report
        .contour
        .curved_turn_diversity
        .expect("circle contour is measurable");
    let polygon_contour = faceted_report
        .contour
        .curved_turn_diversity
        .expect("octagon contour is measurable");
    assert!(
        smooth_contour >= polygon_contour * 1.20,
        "circle must separate from faceted contour by at least 20%: circle={smooth_contour} octagon={polygon_contour}",
    );
    eprintln!(
        "[photo-quality-controls] reflection structured={structured_reflection:.6} \
         flat={flat_reflection:.6}; grounding attached={attached:.6} absent={absent:.6}; \
         contour smooth={smooth_contour:.6} faceted={polygon_contour:.6}"
    );
}

#[test]
fn photo_quality_reports_effective_material_and_projected_texture_density() {
    let legend = [SceneHostSemanticAovLegendEntryV1 {
        palette_index: SUBJECT_ID,
        rgba8: [1, 0, 0, 255],
        node_handle: 10,
        material_handle: Some(100),
        material_kind: Some("pbr_metallic_roughness".to_owned()),
        metallic_factor: Some(1.0),
        roughness_factor: Some(1.0),
        effective_metallic_mean: Some(0.92),
        effective_roughness_mean: Some(0.24),
        surface_texture_min_dimension_px: Some(1024),
        surface_tile_size_m: Some(0.5),
        instance_handle: None,
        instance_id: None,
    }];
    let mut control = control_frame(Shape::Circle, true, true);
    let mut highlighted = 0;
    for (index, id) in control.1.iter().copied().enumerate() {
        if id != SUBJECT_ID || highlighted == 32 {
            continue;
        }
        control.0[index * 4..index * 4 + 4].copy_from_slice(&[255, 255, 255, 255]);
        highlighted += 1;
    }
    let depth_meters = vec![1.0; (WIDTH * HEIGHT) as usize];
    let report = analyze_photo_quality(PhotoQualityAnalysisInputV1 {
        width: WIDTH,
        height: HEIGHT,
        rgba8: &control.0,
        beauty_id_indices: &control.1,
        depth_meters: &depth_meters,
        projection: Some(CaptureProjection::Perspective {
            vertical_fov_radians: 1.0,
            aspect: WIDTH as f32 / HEIGHT as f32,
            near: 0.1,
            far: 10.0,
        }),
        legend: &legend,
        subject_handles: &[10],
        support_handles: &[],
    })
    .expect("synthetic textured-material control is valid");
    let report = serde_json::to_value(report).expect("photo quality report serializes");
    let material = &report["materials"][0];

    assert_eq!(
        material["material_class_basis"], "effective_surface",
        "texture-backed material classification must use effective surface values"
    );
    assert_eq!(material["material_class"], "smooth_metal");
    let effective_metallic = material["effective_metallic_mean"]
        .as_f64()
        .expect("effective metallic mean");
    let effective_roughness = material["effective_roughness_mean"]
        .as_f64()
        .expect("effective roughness mean");
    assert!((effective_metallic - 0.92).abs() <= 1.0e-6);
    assert!((effective_roughness - 0.24).abs() <= 1.0e-6);
    assert_eq!(
        material["projected_texture_density"]["method"],
        "beauty_identity_linear_depth_physical_tile",
        "texture density must be measured in the visible beauty material region"
    );
    assert!(material["luminance_p99_srgb8"].as_f64().unwrap() >= 254.9);
    assert!(material["near_white_fraction"].as_f64().unwrap() > 0.0);
    assert!(material["clipped_fraction"].as_f64().unwrap() > 0.0);
    let density = material["projected_texture_density"]["texels_per_pixel_p50"]
        .as_f64()
        .expect("projected density median");
    assert!(
        (34.0..=36.0).contains(&density),
        "1024 px per 0.5 m at 1 m depth and 1 radian vertical FOV should be about 35 texels/pixel, got {density}"
    );
}

#[test]
fn final_photo_policy_admits_only_four_subject_calibrated_grounding() {
    let policy: serde_json::Value = serde_json::from_slice(
        &fs::read("tests/assets/photo/final/photo_final_policy_v1.json")
            .expect("tracked final-photo policy exists"),
    )
    .expect("final-photo policy is JSON");
    assert_eq!(policy["schema"], "scena.photo_final_policy.v1");
    assert_eq!(policy["mode"], "selective_blocking");
    assert_eq!(policy["admission"]["min_relative_separation"], 0.20);
    assert_eq!(
        policy["metrics"]["contact_shadow_delta_mean_srgb8"]["status"],
        "four_subject_calibrated"
    );
    assert_eq!(
        policy["metrics"]["contact_shadow_delta_mean_srgb8"]["blocking"],
        true
    );
    assert_eq!(
        policy["metrics"]["contact_shadow_delta_mean_srgb8"]["threshold"],
        serde_json::json!({
            "min": 4.0,
            "min_boundary_samples": 32,
            "min_attached_fraction": 0.20
        })
    );
    assert_eq!(
        policy["calibration"]["contact_shadow_delta_mean_srgb8"]["native_positive_srgb8"],
        serde_json::json!({
            "dark_metal_speaker": 33.061,
            "colored_travel_mug": 15.407,
            "valve_manifold": 22.701,
            "demo_hero": 20.188
        })
    );
    assert_eq!(
        policy["calibration"]["contact_shadow_delta_mean_srgb8"]["detached_control_srgb8"],
        0.0
    );
    let metrics = policy["metrics"].as_object().expect("metrics object");
    assert!(
        metrics.iter().all(|(name, metric)| {
            name == "contact_shadow_delta_mean_srgb8" || metric["blocking"] == false
        }),
        "only calibrated same-pass grounding may block final photos",
    );
    assert_eq!(
        policy["admission_decisions"]["reflection_structure_rms_srgb8"],
        "report_only_rewards_noise_and_coherent_artifacts"
    );
    assert_eq!(
        policy["admission_decisions"]["curved_turn_diversity"],
        "report_only_rejects_valid_non_curved_silhouettes"
    );
}

#[test]
fn frozen_subjects_have_reproducible_final_recipes_and_correct_speaker_finish() {
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read("tests/assets/photo/final/fixture_manifest.json")
            .expect("tracked final-photo fixture manifest exists"),
    )
    .expect("fixture manifest is JSON");
    let subjects = manifest["subjects"]
        .as_array()
        .expect("fixture subjects are an array");
    assert_eq!(subjects.len(), 4);

    for subject in subjects {
        let id = subject["id"].as_str().expect("fixture id");
        let path = format!("tests/assets/photo/final/recipes/{id}.recipe.json");
        let recipe: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap_or_else(|error| {
                panic!("final recipe '{path}' must be tracked and readable: {error}")
            }))
            .unwrap_or_else(|error| panic!("final recipe '{path}' must be JSON: {error}"));
        assert_eq!(recipe["schema"], "scena.scene_recipe.v1", "{path}");
        assert_eq!(recipe["photo"]["quality"], "final", "{path}");
        assert_eq!(recipe["photo"]["intent"], "camera_behavior", "{path}");
        assert!(
            recipe.get("capture").is_none(),
            "{path} must use the native final default instead of pinning a low-resolution preview"
        );
        for forbidden in ["cameras", "lights", "render"] {
            assert!(
                recipe.get(forbidden).is_none(),
                "{path} must not hide per-shot {forbidden} tuning"
            );
        }
        for binding in recipe["imports"][0]["material_bindings"]
            .as_array()
            .expect("final material bindings")
        {
            for field in ["normal_scale", "occlusion_strength"] {
                let value = binding["material"][field]
                    .as_f64()
                    .unwrap_or_else(|| panic!("{path} binding must pin {field}"));
                assert!(
                    (0.0..=1.0).contains(&value),
                    "{path} binding {field} must be normalized, got {value}"
                );
            }
        }
    }

    let speaker: serde_json::Value = serde_json::from_slice(
        &fs::read("tests/assets/photo/final/recipes/dark_metal_speaker.recipe.json")
            .expect("speaker final recipe exists"),
    )
    .expect("speaker final recipe is JSON");
    let body = speaker["imports"][0]["material_bindings"]
        .as_array()
        .expect("speaker material bindings")
        .iter()
        .find(|binding| binding["source_material"]["index"] == 1)
        .expect("speaker body binding");
    assert!(
        body["material"].get("material_pack").is_none(),
        "speaker body must not use a directional brushed-steel texture for satin anodized aluminium"
    );
    assert_eq!(body["material"]["base_color"], "#46515C");
    assert_eq!(body["material"]["metallic"], 0.92);
    assert_eq!(body["material"]["roughness"], 0.36);

    let valve: serde_json::Value = serde_json::from_slice(
        &fs::read("tests/assets/photo/final/recipes/valve_manifold.recipe.json")
            .expect("valve final recipe exists"),
    )
    .expect("valve final recipe is JSON");
    let cast_body = valve["imports"][0]["material_bindings"]
        .as_array()
        .expect("valve material bindings")
        .iter()
        .find(|binding| binding["source_material"]["index"] == 4)
        .expect("valve cast body binding");
    assert!(
        cast_body["material"]["material_pack"]["uri"]
            .as_str()
            .is_some_and(|uri| uri.contains("metal027")),
        "the cast body must use a clean dark industrial finish instead of visibly scratched Metal052B"
    );
}

#[test]
fn final_photo_recipes_correct_only_the_four_proven_asset_content_defects() {
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read("tests/assets/photo/final/fixture_manifest.json")
            .expect("final-photo fixture manifest exists"),
    )
    .expect("final-photo fixture manifest parses");
    for subject in manifest["subjects"].as_array().expect("fixture subjects") {
        let path = subject["asset_uri"].as_str().expect("subject asset path");
        let actual =
            Sha256::digest(fs::read(path).unwrap_or_else(|error| {
                panic!("source GLB '{path}' must remain readable: {error}")
            }))
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(
            actual,
            subject["asset_sha256"].as_str().expect("pinned GLB digest"),
            "row-3 recipe corrections must not mutate source GLB '{path}'"
        );
    }

    let recipe = |id: &str| -> serde_json::Value {
        let path = format!("tests/assets/photo/final/recipes/{id}.recipe.json");
        serde_json::from_slice(
            &fs::read(&path)
                .unwrap_or_else(|error| panic!("final recipe '{path}' must read: {error}")),
        )
        .unwrap_or_else(|error| panic!("final recipe '{path}' must parse: {error}"))
    };
    let hidden_import_paths = |recipe: &serde_json::Value| -> Vec<String> {
        recipe["named_states"]
            .as_array()
            .expect("recipe must declare its bounded content-correction state")
            .iter()
            .find(|state| state["id"] == "content_corrections" && state["active"] == true)
            .expect("content-correction state must be active")["visibility"]
            .as_array()
            .expect("content-correction visibility")
            .iter()
            .filter(|entry| entry["visible"] == false)
            .map(|entry| {
                assert_eq!(entry["target"]["kind"], "import_node");
                entry["target"]["path"]
                    .as_str()
                    .expect("hidden import-node path")
                    .to_owned()
            })
            .collect()
    };
    let node_ids = |recipe: &serde_json::Value| -> Vec<String> {
        recipe["nodes"]
            .as_array()
            .expect("authored correction nodes")
            .iter()
            .map(|node| node["id"].as_str().expect("authored node id").to_owned())
            .collect()
    };

    let speaker = recipe("dark_metal_speaker");
    let speaker_scale = speaker["imports"][0]["transform"]["scale"]
        .as_array()
        .expect("speaker scale");
    let diameter = 0.0956_f64;
    let stretched_height = 0.1122 * speaker_scale[1].as_f64().expect("speaker y scale");
    assert!(
        ((stretched_height / diameter) - 1.8).abs() <= 0.05,
        "speaker height-to-diameter ratio must stay approximately 1.8"
    );
    let hidden = hidden_import_paths(&speaker);
    for ring in 0..4 {
        assert!(
            hidden.contains(&format!("/audio_monitor_root/grille_ring_{ring}")),
            "speaker must hide burner-like grille ring {ring}"
        );
    }
    let nodes = node_ids(&speaker);
    for id in [
        "speaker_wrap_grille",
        "speaker_status_led",
        "speaker_knob_index",
    ] {
        assert!(
            nodes.iter().any(|candidate| candidate == id),
            "speaker must author {id}"
        );
    }

    let valve = recipe("valve_manifold");
    let hidden = hidden_import_paths(&valve);
    for path in [
        "/valve_manifold_root/riser_0",
        "/valve_manifold_root/riser_1",
        "/valve_manifold_root/riser_cap_0",
        "/valve_manifold_root/riser_cap_1",
        "/valve_manifold_root/riser_nut_0",
        "/valve_manifold_root/riser_nut_1",
        "/valve_manifold_root/valve_body",
    ] {
        assert!(
            hidden.iter().any(|candidate| candidate == path),
            "valve must hide {path}"
        );
    }
    let nodes = node_ids(&valve);
    for id in [
        "valve_cast_body",
        "valve_bonnet",
        "valve_gland",
        "valve_hub",
        "valve_flange_mate_left",
        "valve_flange_mate_right",
        "valve_saddle_left",
        "valve_saddle_right",
    ] {
        assert!(
            nodes.iter().any(|candidate| candidate == id),
            "valve must author {id}"
        );
    }
    let pipe_binding = valve["imports"][0]["material_bindings"]
        .as_array()
        .expect("valve bindings")
        .iter()
        .find(|binding| binding["source_material"]["index"] == 2)
        .expect("valve pipe binding");
    assert!(
        pipe_binding["material"]["material_pack"]["uri"]
            .as_str()
            .is_some_and(|uri| uri.contains("metal009")),
        "visible valve metal must use the steel finish, not chrome"
    );

    let hero = recipe("demo_hero");
    let hero_bindings = hero["imports"][0]["material_bindings"]
        .as_array()
        .expect("hero bindings");
    let flywheel = hero_bindings
        .iter()
        .find(|binding| binding["source_material"]["index"] == 9)
        .expect("hero flywheel binding");
    assert!(
        flywheel["material"]["material_pack"]["uri"]
            .as_str()
            .is_some_and(|uri| uri.contains("metal052b")),
        "hero flywheel must use its manifest-declared worn steel"
    );
    assert_eq!(
        hero_bindings
            .iter()
            .filter(|binding| binding["source_material"]["index"] == 5)
            .count(),
        1,
        "one baseplate-steel binding must unify both imported bedplates"
    );
    let baseplate = hero_bindings
        .iter()
        .find(|binding| binding["source_material"]["index"] == 5)
        .expect("hero baseplate binding");
    assert_eq!(
        baseplate["material"]["preset"], "brushed_steel",
        "hero baseplate brushing must stay restrained instead of reading as printed laminate"
    );
    assert!(
        baseplate["material"]["roughness"]
            .as_f64()
            .is_some_and(|roughness| roughness >= 0.45),
        "hero baseplate preset must remain restrained rather than mirror bright"
    );
    let powder_coat = hero_bindings
        .iter()
        .find(|binding| binding["source_material"]["index"] == 7)
        .expect("hero powder-coat binding");
    assert_eq!(
        powder_coat["material"]["preset"], "satin",
        "the navy component must use a tinted dielectric satin coating, not molded plastic or black-texture tinting"
    );
    assert_eq!(powder_coat["material"]["base_color"], "#244F7F");
    let gearbox = hero_bindings
        .iter()
        .find(|binding| binding["source_material"]["index"] == 11)
        .expect("hero gearbox binding");
    assert!(
        gearbox["material"]["material_pack"]["uri"]
            .as_str()
            .is_some_and(|uri| uri.contains("metal050a")),
        "the satin gearbox must not repeat the baseplate's strong brushed-steel pattern"
    );
    assert!(
        gearbox["material"]["normal_scale"]
            .as_f64()
            .is_some_and(|strength| strength <= 0.10),
        "gearbox satin variation must remain restrained"
    );
    let hero_glb = gltf::Gltf::from_slice(
        &fs::read("demo/samples/connector-snap/connector_snap_assembly.glb")
            .expect("hero source GLB reads for bedplate identity proof"),
    )
    .expect("hero source GLB parses for bedplate identity proof");
    let bedplates = hero_glb
        .nodes()
        .filter(|node| matches!(node.name(), Some("drive baseplate" | "load baseplate")))
        .map(|node| {
            let material_indices = node
                .mesh()
                .expect("bedplate node has a mesh")
                .primitives()
                .map(|primitive| {
                    primitive
                        .material()
                        .index()
                        .expect("bedplate primitive has a source material")
                })
                .collect::<Vec<_>>();
            (
                node.name().expect("bedplate name").to_owned(),
                material_indices,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        bedplates,
        vec![
            ("load baseplate".to_owned(), vec![5]),
            ("drive baseplate".to_owned(), vec![5]),
        ],
        "both source bedplates must resolve through the one Metal009 binding; \
        any remaining brightness difference is illumination, not a split finish"
    );
    let hero_validation = scena::validate_scene_recipe_value(hero.clone());
    assert!(
        hero_validation.ok,
        "the bounded hero material corrections must remain valid import-binding authoring: {hero_validation:#?}"
    );

    let mug = recipe("colored_travel_mug");
    assert!(
        hidden_import_paths(&mug)
            .iter()
            .any(|candidate| candidate == "/travel_mug_root/body"),
        "mug must replace the ribbed full-height body"
    );
    let grip = mug["imports"][0]["material_bindings"]
        .as_array()
        .expect("mug bindings")
        .iter()
        .find(|binding| binding["source_material"]["index"] == 2)
        .expect("mug grip binding");
    assert_eq!(grip["material"]["base_color"], "#25292D");
    let nodes = node_ids(&mug);
    for id in ["mug_smooth_body", "mug_sip_hinge", "mug_thumb_tab"] {
        assert!(
            nodes.iter().any(|candidate| candidate == id),
            "mug must author {id}"
        );
    }
}

#[test]
fn molded_rubber_bindings_use_the_smooth_scena_rubber_preset() {
    for (recipe_path, source_indices) in [
        (
            "tests/assets/photo/final/recipes/dark_metal_speaker.recipe.json",
            &[2_usize][..],
        ),
        (
            "tests/assets/photo/final/recipes/demo_hero.recipe.json",
            &[6_usize, 12_usize][..],
        ),
    ] {
        let recipe: serde_json::Value = serde_json::from_slice(
            &fs::read(recipe_path)
                .unwrap_or_else(|error| panic!("failed to read '{recipe_path}': {error}")),
        )
        .unwrap_or_else(|error| panic!("failed to parse '{recipe_path}': {error}"));
        let bindings = recipe["imports"][0]["material_bindings"]
            .as_array()
            .expect("final material bindings");

        for source_index in source_indices {
            let binding = bindings
                .iter()
                .find(|binding| binding["source_material"]["index"] == *source_index)
                .unwrap_or_else(|| {
                    panic!("{recipe_path} must bind source material {source_index}")
                });
            let material = &binding["material"];
            assert_eq!(
                material["preset"], "rubber",
                "{recipe_path} source material {source_index} must use Scena's smooth molded-rubber preset"
            );
            assert!(
                material.get("material_pack").is_none(),
                "{recipe_path} source material {source_index} must not use a crumb-rubber texture pack"
            );
            assert_eq!(material["roughness"], 0.86);
            assert_eq!(material["normal_scale"], 0.0);
        }
    }

    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read("tests/assets/photo/final/fixture_manifest.json")
            .expect("final-photo fixture manifest exists"),
    )
    .expect("fixture manifest is JSON");
    for (subject_id, source_indices) in [
        ("dark_metal_speaker", &[2_usize][..]),
        ("demo_hero", &[6_usize, 12_usize][..]),
    ] {
        let subject = manifest["subjects"]
            .as_array()
            .expect("fixture subjects")
            .iter()
            .find(|subject| subject["id"] == subject_id)
            .unwrap_or_else(|| panic!("fixture manifest must include {subject_id}"));
        let material_intents = subject["material_intents"]
            .as_array()
            .expect("fixture material intents");
        for source_index in source_indices {
            let intent = material_intents
                .iter()
                .find(|intent| intent["source_index"] == *source_index)
                .unwrap_or_else(|| {
                    panic!("{subject_id} must declare source material {source_index}")
                });
            assert_eq!(intent["preset"], "rubber");
            assert!(intent.get("catalog_id").is_none());
        }
    }
}

#[derive(Clone, Copy)]
enum Shape {
    Circle,
    Octagon,
}

fn control_frame(
    shape: Shape,
    structured_metal: bool,
    contact_shadow: bool,
) -> (Vec<u8>, Vec<u32>) {
    let mut rgba8 = vec![0_u8; (WIDTH * HEIGHT * 4) as usize];
    let mut ids = vec![0_u32; (WIDTH * HEIGHT) as usize];
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let index = (y * WIDTH + x) as usize;
            let dx = x as i32 - 32;
            let dy = y as i32 - 30;
            let silhouette = match shape {
                Shape::Circle => dx * dx + dy * dy <= 18 * 18,
                Shape::Octagon => dx.abs() <= 18 && dy.abs() <= 18 && dx.abs() + dy.abs() <= 25,
            };
            // Product stills normally meet the receiver through a finite foot,
            // not the single mathematical tangent pixel of a circle. Give the
            // grounding control enough same-pass boundary samples to exercise
            // the confidence floor used by the real oracle.
            let subject = silhouette || ((45..=48).contains(&y) && dx.abs() <= 18);
            let value = if subject {
                ids[index] = SUBJECT_ID;
                if structured_metal {
                    let softbox = 92.0 * (-((x as f64 - 23.0) / 5.0).powi(2)).exp();
                    (72.0 + softbox).round().clamp(0.0, 255.0) as u8
                } else {
                    112
                }
            } else if y >= 48 {
                ids[index] = SUPPORT_ID;
                if contact_shadow && y <= 53 && x.abs_diff(32) <= 19 {
                    104_u8.saturating_add(((y - 48) * 14) as u8)
                } else {
                    184
                }
            } else {
                205
            };
            let offset = index * 4;
            rgba8[offset..offset + 4].copy_from_slice(&[value, value, value, 255]);
        }
    }
    (rgba8, ids)
}
