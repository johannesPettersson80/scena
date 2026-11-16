//! Anti-deletion gate for the post-effect quality contracts.
//!
//! Source-substring pins, like the rest of `ARCH-RENDER-QUALITY`: they stop a
//! quiet return to the filters these replaced. The rendered proofs are the
//! tests each block names.

use crate::app::prelude::*;

pub(crate) fn check_post_effect_quality_contracts(root: &Path, findings: &mut Vec<Finding>) {
    check_bloom_contract(root, findings);
}

/// GPU bloom is a separable Gaussian with a soft knee: a bright pass with the
/// horizontal blur, then the vertical blur added onto the frame.
fn check_bloom_contract(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "ARCH-RENDER-QUALITY";
    require_contains(
        root,
        findings,
        RULE,
        "src/render/gpu/post/bloom.wgsl",
        &[
            "fn fs_composite",
            "const BLOOM_MAX_REACH: i32 = 22;",
            "bloom_blur(coord, dims, vec2<i32>(1, 0), true)",
            "bloom_blur(coord, dims, vec2<i32>(0, 1), false)",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/render/gpu/post/bloom_kernel.wgsl",
        &[
            "fn bloom_bright",
            "smoothstep(threshold, knee_end, luminance)",
            "fn bloom_sigma",
            "sqrt(radius * (radius + 1.0) / 3.0)",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/render/gpu/post/bloom.rs",
        &[
            "include_str!(\"bloom_kernel.wgsl\")",
            "entry_point: Some(\"fs_composite\")",
            "encoder.copy_texture_to_texture(",
            "scena.gpu_post.bloom_composite_pass",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/render/gpu/post/bloom_fxaa.rs",
        &["include_str!(\"bloom_kernel.wgsl\")"],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/render/gpu/post/mod.rs",
        &["bloom::encode(encoder, resources, current, draw_submissions)"],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/render/post_quality_tests/bloom.rs",
        &[
            "gpu_bloom_gives_a_small_emitter_a_round_monotonic_halo",
            "halo_oracle_rejects_a_plus_shaped_bloom",
            "gpu_bloom_leaves_a_frame_below_the_threshold_unchanged",
        ],
    );
    // The plus-shaped nine-tap kernel must not come back.
    for shader in [
        "src/render/gpu/post/bloom.wgsl",
        "src/render/gpu/post/bloom_fxaa.wgsl",
    ] {
        forbid_contains(root, findings, RULE, shader, &["array<vec2<i32>, 9>"]);
    }
}
