#![cfg(feature = "scene-host")]

use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn fr06_cpu_semantic_aov_proves_occlusion_transparency_and_instance_identity() {
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "colors": {
            "opaque": "#4A90E2",
            "transparent": "#F05050"
        },
        "geometries": [{
            "id": "box_geo",
            "primitive": { "kind": "box", "size": [0.16, 0.16, 0.04] }
        }],
        "materials": [
            { "id": "opaque_mat", "kind": "unlit", "base_color": "opaque", "double_sided": true },
            {
                "id": "transparent_mat",
                "kind": "unlit",
                "base_color": "transparent",
                "alpha_mode": { "kind": "blend" },
                "double_sided": true
            }
        ],
        "nodes": [
            {
                "id": "back",
                "geometry": "box_geo",
                "material": "opaque_mat",
                "transform": { "kind": "trs", "translation": [0.0, 0.0, -0.08] }
            },
            {
                "id": "front",
                "geometry": "box_geo",
                "material": "opaque_mat"
            },
            {
                "id": "transparent_foreground",
                "geometry": "box_geo",
                "material": "transparent_mat",
                "transform": { "kind": "trs", "translation": [0.0, 0.0, 0.08] }
            }
        ],
        "instance_sets": [{
            "id": "pair",
            "geometry": "box_geo",
            "material": "opaque_mat",
            "instances": [
                { "id": "left", "transform": { "kind": "trs", "translation": [-0.22, 0.0, 0.0] } },
                { "id": "right", "transform": { "kind": "trs", "translation": [0.22, 0.0, 0.0] } }
            ]
        }],
        "labels": [{
            "id": "excluded_label",
            "text": "AOV",
            "color": "opaque",
            "size_px": 14.0,
            "transform": { "kind": "trs", "translation": [0.0, 0.2, 0.0] }
        }],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "fov_degrees": 34.0,
            "transform": { "kind": "look_at", "eye": [0.0, 0.0, 1.0], "target": [0.0, 0.0, 0.0], "up": [0.0, 1.0, 0.0] }
        }],
        "capture": { "width": 120, "height": 90 }
    });
    let text = serde_json::to_string(&recipe).expect("FR06 recipe serializes");
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "memory://fr06-semantic-aov.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("FR06 recipe builds");
    let front_handle = build
        .manifest
        .nodes
        .iter()
        .find(|node| node.id == "front")
        .expect("front manifest entry")
        .handle;
    assert_eq!(build.manifest.instances.len(), 2);
    assert!(build.manifest.instances.iter().any(|instance| {
        instance.set_id == "pair"
            && instance.id == "left"
            && instance.identity_scope == "runtime_scoped"
    }));
    let mut host = build.host;
    host.prepare().expect("FR06 scene prepares");

    let aov = host
        .capture_semantic_aovs()
        .expect("CPU semantic AOV capture succeeds");

    assert_eq!(aov.schema, "scena.semantic_aov_capture.v1");
    assert_eq!((aov.width, aov.height), (120, 90));
    assert_eq!(aov.identity_scope, "runtime_scoped");
    assert_eq!(aov.sample_pattern, "single_center_sample");
    assert_eq!(aov.depth_convention, "linear_camera_distance_scene_meters");
    assert_eq!(aov.normal_space, "world");
    assert_eq!(aov.id_indices.len(), 120 * 90);
    assert_eq!(aov.depth_meters.len(), 120 * 90);
    assert_eq!(aov.world_normals.len(), 120 * 90);
    assert_eq!(aov.id_rgba8().len(), 120 * 90 * 4);
    assert_eq!(aov.normal_rgba8().len(), 120 * 90 * 4);

    let center = 45 * 120 + 60;
    let center_palette = aov.id_indices[center];
    assert_ne!(center_palette, 0, "center must hit opaque geometry");
    let center_entry = aov
        .legend
        .iter()
        .find(|entry| entry.palette_index == center_palette)
        .expect("center palette has a legend entry");
    assert_eq!(
        center_entry.node_handle, front_handle,
        "transparent foreground is excluded and front opaque node occludes back node"
    );
    assert!(aov.depth_meters[center].is_finite());
    assert!(
        aov.world_normals[center]
            .iter()
            .all(|value| value.is_finite())
    );

    let instance_entries = aov
        .legend
        .iter()
        .filter(|entry| entry.instance_id.is_some())
        .collect::<Vec<_>>();
    assert_eq!(
        instance_entries.len(),
        2,
        "both authored instances are distinct"
    );
    assert_ne!(
        instance_entries[0].palette_index,
        instance_entries[1].palette_index
    );
    assert!(aov.exclusions.transparent_triangle_count > 0);
    assert!(aov.exclusions.label_quad_count > 0);

    let again = host
        .capture_semantic_aovs()
        .expect("second CPU semantic AOV capture succeeds");
    assert_eq!(aov, again, "unchanged prepared state is byte deterministic");
}

#[test]
fn fr06_recipe_aov_cli_writes_portable_images_and_persistent_legend() {
    let root = PathBuf::from("target/gate-artifacts/fr06-semantic-aov");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("FR06 artifact directory creates");
    let recipe_path = root.join("semantic.recipe.json");
    let output_dir = root.join("aov-output");
    fs::write(
        &recipe_path,
        serde_json::to_vec_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": { "blue": "#4A90E2" },
            "geometries": [{
                "id": "box_geo",
                "primitive": { "kind": "box", "size": [0.16, 0.16, 0.04] }
            }],
            "materials": [{
                "id": "box_mat", "kind": "unlit", "base_color": "blue", "double_sided": true
            }],
            "nodes": [{ "id": "body", "geometry": "box_geo", "material": "box_mat" }],
            "instance_sets": [{
                "id": "pair",
                "geometry": "box_geo",
                "material": "box_mat",
                "instances": [
                    { "id": "left", "transform": { "kind": "trs", "translation": [-0.24, 0.0, 0.0] } },
                    { "id": "right", "transform": { "kind": "trs", "translation": [0.24, 0.0, 0.0] } }
                ]
            }],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 1.0], "target": [0.0, 0.0, 0.0] }
            }],
            "capture": { "width": 96, "height": 72 }
        }))
        .expect("FR06 CLI recipe serializes"),
    )
    .expect("FR06 CLI recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "aov",
            path(&recipe_path),
            "--out-dir",
            path(&output_dir),
            "--passes",
            "id,depth,normal",
        ])
        .output()
        .expect("FR06 AOV command runs");
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("FR06 AOV stdout is JSON");
    assert_eq!(report["schema"], "scena.semantic_aov_result.v1");
    assert_eq!(report["ok"], true);
    assert_eq!(report["semantics"]["identity_scope"], "runtime_scoped");
    assert_eq!(report["semantics"]["depth"]["units"], "scene_meters");
    assert_eq!(report["semantics"]["normal"]["coordinate_space"], "world");
    for pass in ["id", "depth", "normal"] {
        let png = PathBuf::from(
            report["images"][pass]["png"]
                .as_str()
                .expect("pass PNG path"),
        );
        assert!(fs::metadata(png).expect("pass PNG exists").len() > 0);
    }
    let legend = report["legend"].as_array().expect("AOV legend array");
    assert!(legend.iter().any(|entry| {
        entry["persistent_identity"]["kind"] == "recipe_node"
            && entry["persistent_identity"]["node_id"] == "body"
    }));
    for expected in ["left", "right"] {
        assert!(legend.iter().any(|entry| {
            entry["persistent_identity"]["kind"] == "recipe_instance"
                && entry["persistent_identity"]["set_id"] == "pair"
                && entry["persistent_identity"]["instance_id"] == expected
        }));
    }
    let report_path = output_dir.join("semantic-aov-result.json");
    assert!(
        fs::metadata(report_path)
            .expect("saved report exists")
            .len()
            > 0
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn fr06_headless_gpu_semantic_aov_matches_cpu_center_truth() {
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "blue": "#4A90E2", "red": "#F05050" },
        "geometries": [{
            "id": "box_geo",
            "primitive": { "kind": "box", "size": [0.28, 0.28, 0.05] }
        }],
        "materials": [
            { "id": "opaque", "kind": "unlit", "base_color": "blue", "double_sided": true },
            {
                "id": "transparent",
                "kind": "unlit",
                "base_color": "red",
                "alpha_mode": { "kind": "blend" },
                "double_sided": true
            }
        ],
        "nodes": [
            { "id": "body", "geometry": "box_geo", "material": "opaque" },
            {
                "id": "transparent_foreground",
                "geometry": "box_geo",
                "material": "transparent",
                "transform": { "kind": "trs", "translation": [0.0, 0.0, 0.1] }
            }
        ],
        "instance_sets": [{
            "id": "pair",
            "geometry": "box_geo",
            "material": "opaque",
            "instances": [
                { "id": "left", "transform": { "kind": "trs", "translation": [-0.38, 0.0, 0.0] } },
                { "id": "right", "transform": { "kind": "trs", "translation": [0.38, 0.0, 0.0] } }
            ]
        }],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "fov_degrees": 34.0,
            "transform": { "kind": "look_at", "eye": [0.0, 0.0, 1.0], "target": [0.0, 0.0, 0.0], "up": [0.0, 1.0, 0.0] }
        }],
        "capture": { "width": 96, "height": 72 }
    });
    let text = serde_json::to_string(&recipe).expect("FR06 GPU recipe serializes");
    let cpu_build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "memory://fr06-gpu-parity.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("FR06 CPU truth recipe builds");
    let gpu_build = pollster::block_on(scena::SceneHostCore::build_recipe_json_gpu(
        "memory://fr06-gpu-parity.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("FR06 required headless GPU adapter constructs");

    let mut cpu = cpu_build.host;
    cpu.prepare().expect("FR06 CPU truth prepares");
    let cpu_aov = cpu
        .capture_semantic_aovs()
        .expect("FR06 CPU truth capture succeeds");

    let mut gpu = gpu_build.host;
    assert_required_native_hardware_adapter(&gpu);
    gpu.set_semantic_aov_capture_enabled(true);
    gpu.prepare()
        .expect("FR06 GPU scene prepares AOV resources");
    let gpu_aov = gpu
        .capture_semantic_aovs_gpu()
        .expect("FR06 headless GPU AOV capture succeeds");

    assert_eq!((gpu_aov.width, gpu_aov.height), (96, 72));
    assert_eq!(gpu_aov.legend.len(), cpu_aov.legend.len());
    assert_eq!(
        gpu_aov.exclusions.transparent_triangle_count,
        cpu_aov.exclusions.transparent_triangle_count,
    );
    let center = 36 * 96 + 48;
    assert_ne!(gpu_aov.id_indices[center], 0, "GPU center hits body");
    assert_eq!(
        gpu_aov.id_indices[center], cpu_aov.id_indices[center],
        "GPU and CPU agree on the unambiguous center identity",
    );
    assert!(gpu_aov.depth_meters[center].is_finite());
    assert!(
        (gpu_aov.depth_meters[center] - cpu_aov.depth_meters[center]).abs() <= 0.001,
        "24-bit GPU linear-depth encoding stays within one millimeter of CPU truth: gpu={} cpu={}",
        gpu_aov.depth_meters[center],
        cpu_aov.depth_meters[center],
    );
    for component in 0..3 {
        assert!(
            (gpu_aov.world_normals[center][component] - cpu_aov.world_normals[center][component])
                .abs()
                <= 0.01,
            "RGBA8 GPU normal stays within quantization tolerance of CPU truth",
        );
    }

    let capability_report: serde_json::Value = serde_json::from_str(
        &gpu.capabilities_json()
            .expect("FR06 native GPU capabilities serialize"),
    )
    .expect("FR06 native GPU capabilities are JSON");
    let gpu_hit_count = gpu_aov.id_indices.iter().filter(|id| **id != 0).count();
    let gpu_finite_depth_count = gpu_aov
        .depth_meters
        .iter()
        .zip(&gpu_aov.id_indices)
        .filter(|(depth, id)| **id != 0 && depth.is_finite())
        .count();
    let artifact = json!({
        "schema": "scena.fr06.native_semantic_aov_proof.v1",
        "status": "passed",
        "release_evidence": std::env::var_os("SCENA_REQUIRE_HARDWARE_GPU").is_some(),
        "required_hardware": std::env::var_os("SCENA_REQUIRE_HARDWARE_GPU").is_some(),
        "capability_report": capability_report,
        "fixture": {
            "width": gpu_aov.width,
            "height": gpu_aov.height,
            "transparent_triangle_count": gpu_aov.exclusions.transparent_triangle_count,
            "legend_entries": gpu_aov.legend.len(),
        },
        "coverage": {
            "gpu_hit_count": gpu_hit_count,
            "gpu_finite_depth_count": gpu_finite_depth_count,
        },
        "center": {
            "index": center,
            "cpu_id": cpu_aov.id_indices[center],
            "gpu_id": gpu_aov.id_indices[center],
            "cpu_depth_meters": cpu_aov.depth_meters[center],
            "gpu_depth_meters": gpu_aov.depth_meters[center],
            "cpu_world_normal": cpu_aov.world_normals[center],
            "gpu_world_normal": gpu_aov.world_normals[center],
        },
        "tolerances": {
            "max_depth_error_meters": 0.001,
            "max_normal_component_error": 0.01,
        },
        "command": std::env::var("SCENA_HARDWARE_PROOF_COMMAND").unwrap_or_else(|_| {
            "cargo test --features scene-host --test fr06_semantic_aov fr06_headless_gpu_semantic_aov_matches_cpu_center_truth -- --exact --nocapture".to_owned()
        }),
    });
    let artifact_root = std::env::var_os("SCENA_HARDWARE_PROOF_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let artifact_path = artifact_root
        .join("target/gate-artifacts/fr06-semantic-aov/native/native-semantic-aov-proof.json");
    fs::create_dir_all(
        artifact_path
            .parent()
            .expect("FR06 native artifact path has a parent"),
    )
    .expect("FR06 native artifact directory creates");
    fs::write(
        artifact_path,
        serde_json::to_vec_pretty(&artifact).expect("FR06 native artifact serializes"),
    )
    .expect("FR06 native artifact writes");
}

#[cfg(not(target_arch = "wasm32"))]
fn assert_required_native_hardware_adapter(host: &scena::SceneHostCore) {
    if std::env::var_os("SCENA_REQUIRE_HARDWARE_GPU").is_some() {
        let report: serde_json::Value = serde_json::from_str(
            &host
                .capabilities_json()
                .expect("FR06 required native GPU capabilities serialize"),
        )
        .expect("FR06 required native GPU capabilities are JSON");
        let adapter = &report["adapter"];
        let device_type = adapter["device_type"]
            .as_str()
            .expect("FR06 required native GPU reports device_type");
        let identity = format!(
            "{} {} {} {}",
            adapter["name"].as_str().unwrap_or_default(),
            device_type,
            adapter["driver"].as_str().unwrap_or_default(),
            adapter["driver_info"].as_str().unwrap_or_default(),
        )
        .to_ascii_lowercase();
        assert!(
            matches!(device_type, "DiscreteGpu" | "IntegratedGpu" | "VirtualGpu"),
            "FR06 required native lane needs a hardware GPU device type, report={report:#}"
        );
        for marker in [
            "swiftshader",
            "llvmpipe",
            "lavapipe",
            "software rasterizer",
            "microsoft basic render",
        ] {
            assert!(
                !identity.contains(marker),
                "FR06 required native lane rejects software adapter marker {marker}, report={report:#}"
            );
        }
    }
}

fn path(path: &Path) -> &str {
    path.to_str().expect("fixture path is UTF-8")
}
