use crate::app::prelude::*;

pub(crate) const REQUIRED_Q08_PARITY_RESULTS: &[(&str, &str, &str)] = &[
    (
        "q08-required-parity/physical-glass-transmission-matches-cpu-and-gpu-across-volume-sweep.json",
        "physical_glass_transmission_matches_cpu_and_gpu_across_volume_sweep",
        "tests/transmission_parity.rs",
    ),
    (
        "q08-required-parity/close-camera-near-clip-matches-cpu-and-gpu-rendered-output.json",
        "close_camera_near_clip_matches_cpu_and_gpu_rendered_output",
        "tests/c13_depth_clipping_parity.rs",
    ),
    (
        "q08-required-parity/dynamic-transform-motion-matches-cpu-and-gpu-for-authored-animation-and-imports.json",
        "dynamic_transform_motion_matches_cpu_and_gpu_for_authored_animation_and_imports",
        "tests/dynamic_transform_parity.rs",
    ),
    (
        "q08-required-parity/z-up-imported-rotation-frame-matches-cpu-and-gpu-after-basis-conversion.json",
        "z_up_imported_rotation_frame_matches_cpu_and_gpu_after_basis_conversion",
        "tests/dynamic_transform_parity.rs",
    ),
    (
        "q08-required-parity/core-pbr-brdf-matches-cpu-and-gpu-across-metallic-roughness-sweep.json",
        "core_pbr_brdf_matches_cpu_and_gpu_across_metallic_roughness_sweep",
        "tests/pbr_brdf_parity.rs",
    ),
    (
        "q08-required-parity/pf08-adaptive-texture-bake-preserves-seams-perspective-and-material-identity-cpu-gpu.json",
        "pf08_adaptive_texture_bake_preserves_seams_perspective_and_material_identity_cpu_gpu",
        "tests/pf08_texture_bake_parity.rs",
    ),
];

pub(super) fn validate_q08_parity_results(
    output: &Path,
    expected_commit: &str,
) -> Result<(), String> {
    let root = repo_root()?;
    for (suffix, test_name, source) in REQUIRED_Q08_PARITY_RESULTS {
        let path = output.join(suffix);
        let value: Value = serde_json::from_str(
            &fs::read_to_string(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?,
        )
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        for (field, expected) in [
            ("schema", "scena.q08.required_cpu_gpu_parity.v1"),
            ("status", "passed"),
            ("proof_class", "physical-hardware-required"),
            ("test_name", *test_name),
            ("commit_sha", expected_commit),
        ] {
            if value.get(field).and_then(Value::as_str) != Some(expected) {
                return Err(format!(
                    "Q08 parity {test_name} {field} must be {expected:?}"
                ));
            }
        }
        if value.get("release_evidence").and_then(Value::as_bool) != Some(true)
            || value
                .get("assertions_executed")
                .and_then(Value::as_u64)
                .is_none_or(|count| count == 0)
        {
            return Err(format!(
                "Q08 parity {test_name} must carry release evidence and executed assertions"
            ));
        }
        let adapter = value
            .get("adapter")
            .ok_or_else(|| format!("Q08 parity {test_name} is missing adapter"))?;
        let device_type = adapter
            .get("device_type")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !matches!(device_type, "DiscreteGpu" | "IntegratedGpu" | "VirtualGpu") {
            return Err(format!(
                "Q08 parity {test_name} adapter is not physical hardware"
            ));
        }
        let identity = ["name", "driver", "driver_info"]
            .into_iter()
            .filter_map(|field| adapter.get(field).and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();
        if [
            "llvmpipe",
            "lavapipe",
            "swiftshader",
            "software",
            "basic render",
        ]
        .iter()
        .any(|marker| identity.contains(marker))
        {
            return Err(format!("Q08 parity {test_name} uses a software adapter"));
        }
        let source_sha = sha256_hex(&root.join(source)).map_err(|error| error.to_string())?;
        let source_bound = value
            .get("source_checksums")
            .and_then(Value::as_array)
            .is_some_and(|entries| {
                entries.iter().any(|entry| {
                    entry.get("path").and_then(Value::as_str) == Some(*source)
                        && entry.get("sha256").and_then(Value::as_str) == Some(&source_sha)
                })
            });
        if !source_bound {
            return Err(format!(
                "Q08 parity {test_name} does not bind source {source}"
            ));
        }
    }
    Ok(())
}
