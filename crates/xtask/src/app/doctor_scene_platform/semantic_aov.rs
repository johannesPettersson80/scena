use crate::app::prelude::*;

pub(crate) fn check_fr06_semantic_aov_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "FR06-SEMANTIC-AOV";
    let required: &[(&str, &[&str])] = &[
        (
            "docs/specs/semantic-aov-v1.md",
            &[
                "node plus an optional authored instance",
                "Palette index `0`",
                "runtime-scoped",
                "Alpha-blended/OIT geometry and physical transmission are excluded",
                "single center sample",
                "linear camera/view distance in scene meters",
                "world-space geometric vertex normals",
                "native/headless GPU",
                "WebGPU, and WebGL2",
            ],
        ),
        (
            "src/render/prepare/types.rs",
            &[
                "source_instance: Option<InstanceId>",
                "semantic_opaque: bool",
                "semantic_alpha_cutoff: Option<f32>",
                "fn with_source_instance",
                "fn with_semantic_material",
                "fn without_semantic_attribution",
            ],
        ),
        (
            "src/render/prepare.rs",
            &["instance: Some(instance.id())", "instance: None"],
        ),
        (
            "src/render/semantic_aov.rs",
            &[
                "self.prepared_state(scene)?",
                "Backend::Headless",
                "MAX_PALETTE_INDEX",
                "primitive.semantic_opaque()",
                "primitive.source_instance()",
                "f32::INFINITY",
                "cpu_geometry::perspective_weights",
                "cpu_geometry::point_is_clipped(world, clipping_planes, section_box)",
                "x as f32 + 0.5",
                "y as f32 + 0.5",
            ],
        ),
        (
            "src/render/cpu_geometry.rs",
            &[
                "camera.interpolation_weights(projected, affine)",
                "pub(super) fn point_is_clipped(",
                "signed_distance < -CLIPPING_BOUNDARY_TOLERANCE",
                "section.clips(position)",
            ],
        ),
        (
            "src/render/settings.rs",
            &[
                "with_semantic_aov_capture",
                "semantic_aov_capture_enabled",
                "set_semantic_aov_capture_enabled",
                "mark_output_resources_changed",
            ],
        ),
        (
            "src/render/gpu/semantic_aov.rs",
            &[
                "wgpu::TextureFormat::Rgba8Unorm",
                "wgpu::TextureFormat::Depth32Float",
                "wgpu::TextureUsages::TEXTURE_BINDING",
                "entry_point: Some(\"fs_semantic\")",
                "semantic.reversed_z",
                "webgl2_readback",
            ],
        ),
        (
            "src/render/gpu/semantic_aov/capture.rs",
            &[
                "scena.semantic_aov.encoder",
                "super::webgl2::capture",
                "js_sys::Promise",
                "semantic AOV readback failed",
            ],
        ),
        ("src/render/gpu/semantic_aov.rs", &["blit_srgb.wgsl"]),
        (
            "src/render/gpu/semantic_aov/webgl2.rs",
            &[
                "read_webgl2_canvas_rgba8",
                "get_current_texture",
                "surface_output.present()",
            ],
        ),
        (
            "src/render/gpu/output_shader.wgsl",
            &[
                "@location(15) instance_semantic_id",
                "fn fs_semantic",
                "encode_semantic_depth",
            ],
        ),
        (
            "src/render/gpu/output_shader_texture_2d.wgsl",
            &[
                "@location(15) instance_semantic_id",
                "fn fs_semantic",
                "encode_semantic_depth",
            ],
        ),
        (
            "src/render/gpu/instancing.rs",
            &[
                "encode_instance_draw_state_with_semantics",
                "semantic_ids",
                "palette_rgba_f32",
            ],
        ),
        (
            "src/render/gpu/vertices.rs",
            &[
                "encode_draw_batches_indexed_with_semantics",
                "semantic_eligible",
                "semantic_id",
            ],
        ),
        (
            "src/scene_host/semantic_aov.rs",
            &[
                "scena.semantic_aov_capture.v1",
                "pub fn capture_semantic_aovs",
                "identity_scope: \"runtime_scoped\".to_owned()",
                "linear_camera_distance_scene_meters",
                "normal_space: \"world\"",
                "pub fn id_rgba8",
                "pub fn depth_u16",
                "pub fn normal_rgba8",
                "pub fn capture_semantic_aovs_gpu",
                "pub async fn capture_semantic_aovs_gpu_async",
            ],
        ),
        (
            "src/scene_host/wasm.rs",
            &[
                "setSemanticAovCaptureEnabled",
                "captureSemanticAovs",
                "capture_semantic_aovs_gpu_async",
                "idIndices",
                "depthMeters",
                "worldNormals",
            ],
        ),
        (
            "src/scene/recipe/types/build_manifest.rs",
            &[
                "pub instances: Vec<SceneRecipeBuildInstanceV1>",
                "pub struct SceneRecipeBuildInstanceV1",
                "pub identity_scope: String",
            ],
        ),
        (
            "src/scene_host/recipe/authoring/extras.rs",
            &[
                "SceneRecipeBuildInstanceV1",
                "set_id: recipe.id.clone()",
                "id: instance.id.clone()",
                "instance_id: instance_id.as_u64()",
                "identity_scope: \"runtime_scoped\"",
            ],
        ),
        (
            "src/bin/scena/recipe/semantic_aov.rs",
            &[
                "scena.semantic_aov_result.v1",
                "host.prepare()",
                ".capture_semantic_aovs()",
                "write_png_rgba8",
                "write_png_gray16",
                "recipe_instance",
                "runtime_only",
                "excluded_and_counted",
                "\"sample_pattern\": capture.sample_pattern",
            ],
        ),
        (
            "src/bin/scena/help.rs",
            &["recipe aov <recipe.json>", "scena.semantic_aov_result.v1"],
        ),
        (
            "src/schema_catalog.rs",
            &[
                "scena.semantic_aov_result.v1",
                "tests/assets/stable-contracts/semantic_aov_result.v1.json",
            ],
        ),
        (
            "docs/schema-contracts.md",
            &[
                "### `scena.semantic_aov_result.v1`",
                "Runtime handles are explicitly not persistence identifiers",
                "Alpha-blended/transmissive geometry",
            ],
        ),
        (
            "tests/browser/fr06_semantic_aov.js",
            &[
                "scena.fr06_semantic_aov_browser_proof.v1",
                "webgpu,webgl2",
                "deterministic_repeat",
                "identity_agreement_on_common_hits",
                "max_depth_error_meters",
                "min_normal_dot",
                "evaluateRequiredHardwareAdapter",
                "SCENA_REQUIRE_PARITY",
                "hardware_evidence",
                "page.on(\"response\"",
                "page.on(\"requestfailed\"",
                "unexpected HTTP failures",
            ],
        ),
        (
            "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
            &[
                "\"baseColorTexture\"",
                "khronos/WaterBottle/WaterBottle_baseColor.png",
            ],
        ),
        (
            "tests/release/windows_complete_hardware_proof_validation.js",
            &[
                "scena.windows_complete_hardware_proof.v1",
                "scena.fr06_semantic_aov_browser_proof.v1",
                "scena.fr06.native_semantic_aov_proof.v1",
                "FR06 browser mask agreement is below 0.98",
                "FR06 browser identity agreement is below 0.995",
                "FR06 browser depth error exceeds 0.005 meters",
                "FR06 browser normal agreement is below 0.98",
                "native FR06 artifact is not release evidence",
                "missing visual artifact",
            ],
        ),
        (
            "scripts/run_windows_complete_hardware_proof.ps1",
            &[
                "browser:fr06-semantic-aov",
                "scena-fr06-native-hardware-proof.exe",
                "SCENA_REQUIRE_HARDWARE_GPU",
                "fr06_headless_gpu_semantic_aov_matches_cpu_center_truth",
                "windows_complete_hardware_proof_validation.js",
            ],
        ),
        (
            "package.json",
            &[
                "browser:fr06-semantic-aov",
                "tests/browser/fr06_semantic_aov.js",
            ],
        ),
        (
            ".github/workflows/ci.yml",
            &["npm run browser:fr06-semantic-aov"],
        ),
        (
            ".github/workflows/release.yml",
            &["npm run browser:fr06-semantic-aov"],
        ),
        (
            ".github/workflows/hardware-gpu.yml",
            &[
                "SCENA_REQUIRE_HARDWARE_GPU: \"1\"",
                "SCENA_REQUIRE_PARITY: \"1\"",
                "cargo test --features scene-host --test fr06_semantic_aov",
                "npm run browser:fr06-semantic-aov",
            ],
        ),
    ];
    for (relative, needles) in required {
        require_contains(root, findings, RULE, relative, needles);
    }
    const FR06_TEXTURE: &str = "tests/assets/gltf/khronos/WaterBottle/WaterBottle_baseColor.png";
    if !root.join(FR06_TEXTURE).is_file() {
        findings.push(Finding::new(
            RULE,
            format!("FR06 fixture texture is missing: {FR06_TEXTURE}"),
        ));
    }
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/fr06_semantic_aov.rs",
        &[
            "fr06_cpu_semantic_aov_proves_occlusion_transparency_and_instance_identity",
            "fr06_recipe_aov_cli_writes_portable_images_and_persistent_legend",
            "fr06_headless_gpu_semantic_aov_matches_cpu_center_truth",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "tests/fr06_semantic_aov.rs",
        &[
            "transparent foreground is excluded",
            "both authored instances are distinct",
            "unchanged prepared state is byte deterministic",
            "persistent_identity",
            "SCENA_REQUIRE_HARDWARE_GPU",
            "assert_required_native_hardware_adapter",
            "scena.fr06.native_semantic_aov_proof.v1",
            "release_evidence",
            "SCENA_HARDWARE_PROOF_ROOT",
            "native-semantic-aov-proof.json",
            "SCENA_HARDWARE_PROOF_COMMAND",
        ],
    );
}
