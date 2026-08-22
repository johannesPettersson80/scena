use super::{MeshPipelineRequirements, PrimitiveDrawBatch};

fn draw_batch(double_sided: bool) -> PrimitiveDrawBatch {
    PrimitiveDrawBatch {
        start_vertex: 0,
        vertex_count: 3,
        material_slot: 0,
        draw_uniform_index: 0,
        depth_prepass_eligible: true,
        double_sided,
        semantic_eligible: false,
        reflection_probe_slot: None,
    }
}

#[test]
fn browser_surface_pipeline_compilation_is_limited_to_used_sides() {
    let pipeline = include_str!("pipeline.rs")
        .split("#[cfg(test)]")
        .next()
        .expect("pipeline implementation precedes tests");
    let requirements = include_str!("pipeline_requirements.rs");
    let browser_prepare = include_str!("prepare_resources_wasm.rs");
    assert!(
        pipeline.contains("MeshPipelineRequirements")
            && pipeline.contains("compiled_pipeline_count")
            && requirements.contains("from_batches")
            && requirements.contains("compiled_pipeline_count")
            && browser_prepare.contains("MeshPipelineRequirements::from_batches")
            && browser_prepare.contains("create_unlit_pipeline_set_for_requirements"),
        "WebGL2 preparation must compile only the culling variants used by the encoded scene",
    );

    let single = MeshPipelineRequirements::from_batches(&[draw_batch(false)], &[]);
    assert_eq!(single.compiled_pipeline_count(), 1);
    assert!(single.single_sided);
    assert!(!single.double_sided);

    let mixed = MeshPipelineRequirements::from_batches(&[draw_batch(false), draw_batch(true)], &[]);
    assert_eq!(mixed.compiled_pipeline_count(), 2);
    assert!(mixed.single_sided);
    assert!(mixed.double_sided);

    let empty = MeshPipelineRequirements::from_batches(&[], &[]);
    assert_eq!(empty.compiled_pipeline_count(), 1);
    assert!(empty.single_sided);
    assert!(!empty.double_sided);
}

#[test]
fn unlit_pipeline_source_wires_depth_state_into_visible_color_pass() {
    let source = include_str!("pipeline.rs");
    let implementation = source
        .split("#[cfg(test)]")
        .next()
        .expect("pipeline implementation precedes tests");
    assert!(
        implementation.contains("RenderPassDepthStencilAttachment")
            && implementation.contains("depth_stencil: depth_compare.map"),
        "visible GPU color pass must use the prepared depth buffer when one exists"
    );
}

#[test]
fn depth_prepass_and_color_pass_use_identical_clip_space_transform() {
    // Pi 5 V3D WebGL2 runs the fragment stage at lower-than-highp
    // precision by default. If the depth pre-pass computes clip-space
    // depth via a different matrix multiplication path than the color
    // pass, the two ULP-diverge and the LessEqual depth test rejects
    // most color-pass fragments, producing a mostly-black render on
    // V3D-class hardware while Lavapipe/desktop GL is unaffected.
    // Both shaders must use `clip_from_world * world_position`.
    let depth = include_str!("depth.rs");
    let color = include_str!("output_shader.wgsl");
    let color_tex2d = include_str!("output_shader_texture_2d.wgsl");
    for (label, source) in [
        ("depth.rs", depth),
        ("output_shader.wgsl", color),
        ("output_shader_texture_2d.wgsl", color_tex2d),
    ] {
        assert!(
            source.contains("camera.clip_from_world * world_position"),
            "{label} vs_main must use `camera.clip_from_world * world_position` so depth values match the other passes bit-for-bit",
        );
        assert!(
            !source.contains("camera.clip_from_view * camera.view_from_world * world_position")
                && !source.contains(
                    "camera.clip_from_view * camera.view_from_world * draw.world_from_model",
                ),
            "{label} must not reintroduce a divergent clip-space matrix path",
        );
    }
}

#[test]
fn unlit_pipeline_binds_material_group_for_fragment_sampling() {
    let source = include_str!("pipeline.rs");
    let implementation = source
        .split("#[cfg(test)]")
        .next()
        .expect("pipeline implementation precedes tests");
    assert!(
        implementation.contains("material_bind_group_layout")
            && implementation.contains("material_resources")
            && implementation.contains("pass.set_bind_group(1, &material.bind_group"),
        "visible GPU color pass must bind material resources, not only camera uniforms"
    );
}

#[test]
fn unlit_pipeline_can_split_opaque_and_transparent_draws_for_transmission() {
    let source = include_str!("pipeline.rs");
    let implementation = source
        .split("#[cfg(test)]")
        .next()
        .expect("pipeline implementation precedes tests");
    assert!(
        implementation.contains("enum DrawFilter")
            && implementation.contains("DrawFilter::OpaqueOnly")
            && implementation.contains("DrawFilter::TransparentOnly")
            && implementation.contains("LoadOp::Load")
            && implementation.contains("depth_prepass_eligible"),
        "physical glass needs an opaque scene-color pass followed by a transparent \
         transmission pass; one all-material alpha-blended pass can still ship fake glass"
    );
}

#[test]
fn semantic_capture_pipeline_writes_a_witness_in_the_beauty_pass() {
    let pipeline = include_str!("pipeline.rs")
        .split("#[cfg(test)]")
        .next()
        .expect("pipeline implementation precedes tests");
    let scene_color = include_str!("scene_color.rs");
    for (label, shader) in [
        ("output_shader.wgsl", include_str!("output_shader.wgsl")),
        (
            "output_shader_texture_2d.wgsl",
            include_str!("output_shader_texture_2d.wgsl"),
        ),
    ] {
        assert!(
            shader.contains("fn fs_beauty_semantic")
                && shader.contains("@location(0) color")
                && shader.contains("@location(1) semantic_id"),
            "{label} must emit beauty color and semantic identity from one fragment invocation",
        );
    }
    assert!(
        pipeline.contains("entry_point: Some(fragment_entry_point)")
            && pipeline.contains("semantic_target_format")
            && scene_color.contains("semantic_view"),
        "the visible color pipeline must bind the semantic witness as a second attachment"
    );
}
