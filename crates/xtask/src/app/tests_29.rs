use crate::app::prelude::*;

#[test]
fn q05_doctor_rejects_sampled_hash_and_missing_effect_masks() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q05-effect-footprints");
    let _ = fs::remove_dir_all(&fixture_root);
    for directory in ["tests/visual/fixtures", "tests/visual/references", "tests"] {
        fs::create_dir_all(fixture_root.join(directory)).expect("Q05 fixture directory");
    }
    fs::write(
        fixture_root.join("tests/visual/fixtures/m2-headless-core.toml"),
        "[suite]\nreference_mode = \"sampled-rgba\"\n[[fixture]]\nname = \"direct-lights-pbr\"\n",
    )
    .expect("weak fixture metadata writes");
    fs::write(
        fixture_root.join("tests/visual/references/m2-headless-core.toml"),
        "[suite]\nreference_mode = \"sampled-rgba\"\n[[reference]]\nname = \"direct-lights-pbr\"\ncenter_rgba = [1, 2, 3, 255]\nrgba_hash = \"fnv1a64:weak\"\n",
    )
    .expect("weak reference metadata writes");
    fs::write(
        fixture_root.join("tests/m2_visual_proof.rs"),
        "fn proof() { assert!(nonblack_pixel_count(frame) > 0); }\n",
    )
    .expect("weak visual proof writes");
    let mut findings = Vec::new();

    check_q05_effect_footprint_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "Q05-EFFECT-FOOTPRINTS"),
        "old sampled/hash metadata without paired spatial masks must fail closed: {findings:?}"
    );

    let effects = [
        "direct-lights-pbr",
        "shadowed-directional-light",
        "ibl-environment",
        "anti-aliasing-on-off",
        "bloom-on-off",
        "ssao-contact-on-off",
        "oit-overlap-order-invariance",
        "clipping-half-space",
    ];
    let mut fixture_metadata =
        "[suite]\nreference_mode = \"local-structure-v2\"\nmax_abs_diff = 3\n".to_owned();
    let mut reference_metadata =
        "[suite]\nreference_mode = \"local-structure-v2\"\nmax_abs_diff = 3\n".to_owned();
    for effect in effects {
        fixture_metadata.push_str(&format!(
            "[[fixture]]\nname = \"{effect}\"\nproof_class = \"paired-effect-footprint\"\npair = \"off-left-on-right\"\nspatial_mask = [0, 0, 1, 1]\n"
        ));
        reference_metadata.push_str(&format!(
            "[[reference]]\nname = \"{effect}\"\nmax_abs_diff = 3\ntop_left_mean_rgba = [0, 0, 0, 255]\ntop_right_mean_rgba = [0, 0, 0, 255]\nbottom_left_mean_rgba = [0, 0, 0, 255]\nbottom_right_mean_rgba = [0, 0, 0, 255]\nquadrant_nonblack = [0, 0, 0, 0]\n"
        ));
    }
    fs::write(
        fixture_root.join("tests/visual/fixtures/m2-headless-core.toml"),
        fixture_metadata,
    )
    .expect("strong fixture metadata writes");
    fs::write(
        fixture_root.join("tests/visual/references/m2-headless-core.toml"),
        reference_metadata,
    )
    .expect("strong reference metadata writes");
    fs::write(
        fixture_root.join("tests/m2_visual_proof.rs"),
        "struct EffectPair; struct PixelMask; fn effect_pair_failures() {} fn quadrant_debug_rows_match() {} fn fixture_reference_mode() {} fn reference_mode() {} fn q03_quadrant_debug_rows_notice_coarse_corruption() {} fn q05_effect_footprint_masks_reject_erased_effect_regions() {}\n",
    )
    .expect("strong visual proof writes");
    findings.clear();
    check_q05_effect_footprint_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());
}
