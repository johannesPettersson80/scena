#[cfg(target_arch = "wasm32")]
#[test]
fn webgl2_typed_array_readback_unbinds_wgpu_pack_buffer_first() {
    let source = include_str!("browser_readback.rs");
    let unbind = source
        .find("PIXEL_PACK_BUFFER")
        .expect("WebGL2 readback identifies the pixel pack binding");
    let reset = source
        .get(unbind..)
        .and_then(|suffix| suffix.find("PACK_ROW_LENGTH").map(|offset| unbind + offset))
        .expect("WebGL2 readback resets WGPU's padded pack row length");
    let read = source
        .find("read_pixels.apply(&context, &args)")
        .expect("WebGL2 readback invokes readPixels");
    assert!(
        unbind < reset && reset < read,
        "typed-array readPixels must reset WGPU's pixel-pack state first"
    );
}

#[test]
fn direct_surface_copy_does_not_compile_duplicate_readback_pipelines() {
    let source = include_str!("browser_readback.rs");
    assert!(
        source.contains("render_pipeline_required: bool")
            && source.contains(
                "descriptor.render_pipeline_required && !descriptor.surface_pipeline_reusable",
            )
            && source.contains("pipelines: Option<MeshPipelineSet>"),
        "a COPY_SRC-capable browser surface must not compile an unused duplicate pipeline set"
    );
}

#[test]
fn compatible_independent_readback_reuses_surface_pipelines() {
    let source = include_str!("browser_readback.rs");
    assert!(
        source.contains("surface_pipeline_reusable: bool")
            && source.contains("render_pipeline_required && !descriptor.surface_pipeline_reusable"),
        "an independent readback target with the same attachment contract must reuse the \
         already-compiled surface pipelines"
    );
}

#[test]
fn independent_readback_overlays_use_the_readback_attachment_format() {
    let readback_source = include_str!("browser_readback.rs");
    let prepare_source = include_str!("prepare_resources_wasm.rs");
    assert!(
        readback_source.contains("pipeline: strokes::flat_pipeline(stroke_resources)")
            && readback_source.contains("pipeline: labels::flat_pipeline(label_resources)"),
        "independent byte readback must not use the Rgba16Float post overlay pipelines"
    );
    assert!(
        prepare_source
            .contains("target_format: readback_format_for_surface(surface.config.format)"),
        "browser overlay byte-target pipelines must match the independent readback format"
    );
}
