use crate::app::prelude::*;

/// Calibration parity sweeps that MUST be paired with an external ground-truth
/// oracle (closed-form analytic results or reference-implementation values). CPU↔GPU
/// parity proves the two backends agree, not that they are correct — both can
/// be uniformly wrong. Each entry: (parity test, oracle file, oracle tests).
const PAIRED_CALIBRATION_SWEEPS: &[(&str, &str, &[&str])] = &[
    (
        "tests/pbr_brdf_parity.rs",
        "src/render/pbr_brdf.rs",
        &[
            "fresnel_f0_is_dielectric_ior_exact_and_metal_keeps_base",
            "specular_ggx_conserves_energy_under_white_furnace",
        ],
    ),
    (
        "tests/transmission_parity.rs",
        "src/render/physical_transmission.rs",
        &[
            "volume_transmittance_satisfies_beer_lambert_closed_form",
            "refract_vec3_matches_snells_law",
        ],
    ),
];

/// Parity sweeps that compare behavior or backends rather than calibrate against
/// closed-form math, so they legitimately need no analytic oracle. Listed so a
/// new parity sweep cannot silently skip the calibration requirement.
const NON_CALIBRATION_SWEEPS: &[&str] = &[
    "tests/c13_depth_clipping_parity.rs",
    "tests/c14_gltf_semantic_parity.rs",
    "tests/dynamic_transform_parity.rs",
    "tests/m6_browser_renderer_parity.rs",
    "tests/m6_browser_webgpu_readback.rs",
    "tests/pf08_texture_bake_parity.rs",
];

/// Meta-rule: every `*_parity` sweep must either be paired with an external
/// ground-truth oracle or be explicitly allowlisted as non-calibration. This is
/// the standing guard against "parity instead of calibration" — it fails when a
/// paired oracle is removed, or when a new parity sweep is added with neither an
/// oracle nor an allowlist entry.
pub(crate) fn check_calibration_oracles_pair_parity_sweeps(
    root: &Path,
    findings: &mut Vec<Finding>,
) {
    // 1. Every registered calibration sweep must still carry its oracle.
    for (_, oracle_file, oracle_tests) in PAIRED_CALIBRATION_SWEEPS {
        require_contains(
            root,
            findings,
            "ARCH-CALIBRATION-ORACLE",
            oracle_file,
            oracle_tests,
        );
    }

    // 2. Audit every parity sweep on disk; flag any that is neither paired nor
    //    allowlisted.
    let tests_dir = root.join("tests");
    let Ok(entries) = std::fs::read_dir(&tests_dir) else {
        findings.push(Finding::new(
            "ARCH-CALIBRATION-ORACLE",
            "could not read tests/ to audit parity sweeps for paired oracles",
        ));
        return;
    };
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if !(file_name.contains("parity") && file_name.ends_with(".rs")) {
            continue;
        }
        let rel = format!("tests/{file_name}");
        let registered = PAIRED_CALIBRATION_SWEEPS
            .iter()
            .any(|(parity, _, _)| *parity == rel);
        let allowlisted = NON_CALIBRATION_SWEEPS.contains(&rel.as_str());
        if !registered && !allowlisted {
            findings.push(Finding::new(
                "ARCH-CALIBRATION-ORACLE",
                format!(
                    "parity sweep {rel} is not paired with an external ground-truth oracle, nor \
                     allowlisted as non-calibration. CPU↔GPU parity is not calibration: add an \
                     analytic/reference oracle and register the pairing in calibration_oracles.rs, \
                     or allowlist it as a behavioral/backend sweep."
                ),
            ));
        }
    }
}
