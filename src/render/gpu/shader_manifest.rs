//! Production-derived inventory of every WGSL source assembled by the GPU
//! renderer. The entries reference the same constants passed to wgpu, so the
//! offline validator cannot drift into a parallel hand-written shader list.

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(super) struct ShaderVariant {
    pub(super) id: &'static str,
    pub(super) source: &'static str,
    pub(super) entry_points: &'static [&'static str],
    pub(super) profile: ShaderProfile,
    pub(super) feature_axes: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(super) enum ShaderProfile {
    NativeWebGpu,
    WebGl2Compatible,
}

const VF: &[&str] = &["vs_main", "fs_main"];
const VS: &[&str] = &["vs_main"];
const TRIANGLE: &[&str] = &["vs_main", "fs_main", "fs_semantic"];

macro_rules! define_shader_variants {
    ($($variant:ident => ($id:literal, $source:expr, $entries:expr, $profile:ident, [$($axis:literal),* $(,)?])),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub(super) enum ShaderVariantId {
            $($variant),+
        }

        const PRODUCTION_SHADER_IDS: &[ShaderVariantId] = &[
            $(ShaderVariantId::$variant),+
        ];

        impl ShaderVariantId {
            const fn variant(self) -> ShaderVariant {
                match self {
                    $(Self::$variant => ShaderVariant {
                        id: $id,
                        source: $source,
                        entry_points: $entries,
                        profile: ShaderProfile::$profile,
                        feature_axes: &[$($axis),*],
                    }),+
                }
            }
        }
    };
}

define_shader_variants! {
    TriangleTexture2dArray => (
        "triangle.texture_2d_array", super::output::GPU_TRIANGLE_SHADER, TRIANGLE,
        NativeWebGpu, ["binding:texture_2d_array", "materials", "shadows", "picking", "semantic_aov", "instancing", "skinning", "morphing"]
    ),
    TriangleTexture2d => (
        "triangle.texture_2d", super::output::GPU_TRIANGLE_SHADER_TEXTURE_2D, TRIANGLE,
        WebGl2Compatible, ["binding:texture_2d", "materials", "shadows", "picking", "semantic_aov", "instancing", "skinning", "morphing"]
    ),
    DepthPrepass => (
        "depth.prepass", super::depth::DEPTH_PREPASS_SHADER, VF,
        NativeWebGpu, ["depth", "instancing"]
    ),
    LabelsFinal => (
        "labels.final", super::labels::FINAL_SHADER, VF,
        NativeWebGpu, ["labels", "target:srgb"]
    ),
    LabelsEncoded => (
        "labels.encoded", super::labels::ENCODED_SHADER, VF,
        WebGl2Compatible, ["labels", "target:linear_bytes"]
    ),
    StrokesFinal => (
        "strokes.final", super::strokes::FINAL_SHADER, VF,
        NativeWebGpu, ["strokes", "target:srgb", "instancing"]
    ),
    StrokesEncoded => (
        "strokes.encoded", super::strokes::ENCODED_SHADER, VF,
        WebGl2Compatible, ["strokes", "target:linear_bytes", "instancing"]
    ),
    ShadowDirectional => (
        "shadow.directional", super::shadow::SHADOW_CASTER_SHADER, VS,
        NativeWebGpu, ["shadows", "depth", "instancing"]
    ),
    PostBloom => (
        "post.bloom", super::post::BLOOM_SHADER, VF,
        NativeWebGpu, ["post", "bloom"]
    ),
    PostSsao => (
        "post.ssao", super::post::SSAO_SHADER, VF,
        NativeWebGpu, ["post", "ssao", "depth"]
    ),
    PostSsr => (
        "post.ssr", super::post::SSR_SHADER, VF,
        NativeWebGpu, ["post", "ssr"]
    ),
    PostDof => (
        "post.dof", super::post::DOF_SHADER, VF,
        NativeWebGpu, ["post", "dof", "depth"]
    ),
    PostBlitLinear => (
        "post.blit.linear", super::post::BLIT_LINEAR_SHADER, VF,
        NativeWebGpu, ["post", "blit", "target:srgb"]
    ),
    PostBlitSrgbBytes => (
        "post.blit.srgb_bytes", super::post::BLIT_SRGB_BYTE_SHADER, VF,
        WebGl2Compatible, ["post", "blit", "target:linear_bytes"]
    ),
    PostFxaaLinear => (
        "post.fxaa.linear", super::post::FXAA_LINEAR_SHADER, VF,
        NativeWebGpu, ["post", "fxaa", "target:srgb"]
    ),
    PostFxaaSrgbBytes => (
        "post.fxaa.srgb_bytes", super::post::FXAA_SRGB_BYTE_SHADER, VF,
        WebGl2Compatible, ["post", "fxaa", "target:linear_bytes"]
    ),
    PostBloomFxaaLinear => (
        "post.bloom_fxaa.linear", super::post::BLOOM_FXAA_LINEAR_SHADER, VF,
        NativeWebGpu, ["post", "bloom", "fxaa", "target:srgb"]
    ),
    PostBloomFxaaSrgbBytes => (
        "post.bloom_fxaa.srgb_bytes", super::post::BLOOM_FXAA_SRGB_BYTE_SHADER, VF,
        WebGl2Compatible, ["post", "bloom", "fxaa", "target:linear_bytes"]
    ),
    SemanticAovWebgl2Readback => (
        "semantic_aov.webgl2_readback", super::semantic_aov::WEBGL2_READBACK_SHADER, VF,
        WebGl2Compatible, ["semantic_aov", "readback", "target:srgb"]
    ),
}

#[allow(dead_code)]
pub(super) fn production_shader_variants() -> impl ExactSizeIterator<Item = ShaderVariant> {
    PRODUCTION_SHADER_IDS
        .iter()
        .copied()
        .map(ShaderVariantId::variant)
}

pub(super) fn create_shader_module(
    device: &wgpu::Device,
    variant_id: ShaderVariantId,
    label: &'static str,
) -> wgpu::ShaderModule {
    let variant = variant_id.variant();
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(variant.source.into()),
    })
}

pub(super) const fn variant_for_srgb_target(
    format: wgpu::TextureFormat,
    linear: ShaderVariantId,
    encoded: ShaderVariantId,
) -> ShaderVariantId {
    match format {
        wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm => encoded,
        _ => linear,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate_manifest_coverage(ids: &[ShaderVariantId]) -> Result<(), String> {
        let expected = production_shader_variants()
            .map(|variant| variant.id)
            .collect::<std::collections::BTreeSet<_>>();
        let actual = ids
            .iter()
            .copied()
            .map(ShaderVariantId::variant)
            .map(|variant| variant.id)
            .collect::<std::collections::BTreeSet<_>>();
        if ids.len() != PRODUCTION_SHADER_IDS.len() || actual != expected {
            return Err(format!(
                "shader manifest coverage mismatch: expected {expected:?}, got {actual:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn production_shader_modules_are_created_only_by_manifest_owner() {
        fn rust_sources(path: &std::path::Path, output: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(path).expect("GPU source directory must be readable") {
                let path = entry.expect("GPU source entry must be readable").path();
                if path.is_dir() {
                    rust_sources(&path, output);
                } else if path.extension().is_some_and(|extension| extension == "rs") {
                    output.push(path);
                }
            }
        }

        let gpu_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/render/gpu");
        let owner = gpu_root.join("shader_manifest.rs");
        let mut sources = Vec::new();
        rust_sources(&gpu_root, &mut sources);
        sources.sort();

        let bypasses = sources
            .into_iter()
            .filter(|path| path != &owner)
            .filter_map(|path| {
                let source = std::fs::read_to_string(&path).ok()?;
                source
                    .contains(".create_shader_module(")
                    .then(|| path.strip_prefix(&gpu_root).unwrap().display().to_string())
            })
            .collect::<Vec<_>>();

        assert!(
            bypasses.is_empty(),
            "production shader creation bypasses the manifest owner: {bypasses:?}"
        );
    }

    fn validate(source: &str, capabilities: wgpu::naga::valid::Capabilities) -> Result<(), String> {
        let module = wgpu::naga::front::wgsl::parse_str(source)
            .map_err(|error| error.emit_to_string(source))?;
        wgpu::naga::valid::Validator::new(wgpu::naga::valid::ValidationFlags::all(), capabilities)
            .validate(&module)
            .map_err(|error| error.to_string())?;
        let mut bindings = std::collections::BTreeSet::new();
        for (_, variable) in module.global_variables.iter() {
            if let Some(binding) = variable.binding.as_ref()
                && !bindings.insert((binding.group, binding.binding))
            {
                return Err(format!(
                    "duplicate resource binding @group({}) @binding({})",
                    binding.group, binding.binding
                ));
            }
        }
        Ok(())
    }

    #[test]
    fn every_production_shader_variant_parses_validates_and_exports_required_entries() {
        let variants = production_shader_variants();
        let mut ids = std::collections::BTreeSet::new();
        for variant in variants {
            assert!(
                ids.insert(variant.id),
                "duplicate shader variant id {}",
                variant.id
            );
            let module =
                wgpu::naga::front::wgsl::parse_str(variant.source).unwrap_or_else(|error| {
                    panic!(
                        "{} failed WGSL parse: {}",
                        variant.id,
                        error.emit_to_string(variant.source)
                    )
                });
            wgpu::naga::valid::Validator::new(
                wgpu::naga::valid::ValidationFlags::all(),
                wgpu::naga::valid::Capabilities::empty(),
            )
            .validate(&module)
            .unwrap_or_else(|error| {
                panic!(
                    "{} failed Naga validation for {:?}: {error}",
                    variant.id, variant.profile
                )
            });
            for required in variant.entry_points {
                assert!(
                    module
                        .entry_points
                        .iter()
                        .any(|entry| entry.name == *required),
                    "{} omitted required entry point {required}",
                    variant.id
                );
            }
        }
        assert_eq!(ids.len(), PRODUCTION_SHADER_IDS.len());
    }

    #[test]
    fn production_manifest_inventories_feature_axes_and_rejects_an_omitted_variant() {
        let mut axes = std::collections::BTreeSet::new();
        let mut compute_entry_points = 0;
        for variant in production_shader_variants() {
            axes.extend(variant.feature_axes.iter().copied());
            compute_entry_points += variant
                .entry_points
                .iter()
                .filter(|entry| entry.starts_with("cs_"))
                .count();
        }
        for required in [
            "binding:texture_2d",
            "binding:texture_2d_array",
            "materials",
            "shadows",
            "post",
            "labels",
            "strokes",
            "depth",
            "picking",
            "semantic_aov",
            "instancing",
            "skinning",
            "morphing",
        ] {
            assert!(
                axes.contains(required),
                "missing shader feature axis {required}"
            );
        }
        assert_eq!(
            compute_entry_points, 0,
            "the production renderer currently owns no compute shader modules"
        );
        validate_manifest_coverage(PRODUCTION_SHADER_IDS).unwrap();
        assert!(
            validate_manifest_coverage(&PRODUCTION_SHADER_IDS[..PRODUCTION_SHADER_IDS.len() - 1])
                .is_err(),
            "omitting a generated production shader variant must fail coverage"
        );
    }

    #[test]
    fn offline_shader_gate_rejects_syntax_binding_location_entry_and_capability_mutations() {
        assert!(
            validate(
                "@compute @workgroup_size(1) fn cs_main() { @ }",
                wgpu::naga::valid::Capabilities::all()
            )
            .is_err()
        );
        let duplicate_binding = "@group(0) @binding(0) var a: texture_2d<f32>; @group(0) @binding(0) var b: texture_2d<f32>; @compute @workgroup_size(1) fn cs_main() {}";
        assert!(validate(duplicate_binding, wgpu::naga::valid::Capabilities::all()).is_err());
        let duplicate_location = "struct Out { @builtin(position) p: vec4<f32>, @location(0) a: f32, @location(0) b: f32 } @vertex fn vs_main() -> Out { return Out(vec4<f32>(), 0.0, 0.0); }";
        assert!(validate(duplicate_location, wgpu::naga::valid::Capabilities::all()).is_err());
        let parsed =
            wgpu::naga::front::wgsl::parse_str("@compute @workgroup_size(1) fn wrong_entry() {}")
                .unwrap();
        assert!(
            !parsed
                .entry_points
                .iter()
                .any(|entry| entry.name == "cs_main")
        );
        let unsupported =
            "enable f16; @compute @workgroup_size(1) fn cs_main() { var value: f16 = 1.0h; }";
        assert!(validate(unsupported, wgpu::naga::valid::Capabilities::empty()).is_err());
    }

    /// Largest constant-space array a shader may index with a runtime value.
    ///
    /// Hardware without an indexed constant-register file cannot express such a
    /// read directly; the driver expands it into a select chain over every
    /// element. Eight reads into two 256-entry LTC tables made V3D's register
    /// allocator fail at all thirteen of its fallback strategies and produced a
    /// 22,518-instruction fragment shader against a 74-instruction median.
    /// Uniform and storage buffers are exempt: those are memory loads.
    const MAX_DYNAMICALLY_INDEXED_CONSTANT_ARRAY: u32 = 64;

    /// Emitted-size budgets, ratcheted to the measured values with headroom.
    /// They are coarse by design: the rule above names the specific hazard,
    /// while these catch any other change that inflates a shader wholesale.
    ///
    /// Measured maxima, both `triangle` fragment entry points: 5,263 SPIR-V
    /// instructions and 84,931 bytes of GLSL ES 3.00. With the LTC tables baked
    /// in as `const` arrays the same entry points measured 7,405 instructions
    /// and roughly 117,000 bytes, so these budgets reject that shape.
    const MAX_ENTRY_POINT_SPIRV_INSTRUCTIONS: usize = 6_000;
    const MAX_WEBGL2_ENTRY_POINT_GLSL_BYTES: usize = 100_000;

    /// Resolves an access chain to the global it ultimately reads, so a uniform
    /// or storage buffer is not mistaken for an in-shader constant table.
    fn access_root_address_space(
        function: &wgpu::naga::Function,
        module: &wgpu::naga::Module,
        mut expr: wgpu::naga::Handle<wgpu::naga::Expression>,
    ) -> Option<wgpu::naga::AddressSpace> {
        use wgpu::naga::Expression;
        for _ in 0..64 {
            match &function.expressions[expr] {
                Expression::Access { base, .. } | Expression::AccessIndex { base, .. } => {
                    expr = *base;
                }
                Expression::GlobalVariable(handle) => {
                    return Some(module.global_variables[*handle].space);
                }
                _ => return None,
            }
        }
        None
    }

    fn dynamic_constant_array_indexes(
        module: &wgpu::naga::Module,
        info: &wgpu::naga::valid::ModuleInfo,
    ) -> Vec<String> {
        use wgpu::naga::{AddressSpace, ArraySize, Expression, TypeInner};
        let mut findings = Vec::new();
        let mut scan = |function: &wgpu::naga::Function,
                        function_info: &wgpu::naga::valid::FunctionInfo,
                        label: &str| {
            for (_handle, expr) in function.expressions.iter() {
                let Expression::Access { base, index } = expr else {
                    continue;
                };
                if matches!(function.expressions[*index], Expression::Literal(_)) {
                    continue;
                }
                if matches!(
                    access_root_address_space(function, module, *base),
                    Some(AddressSpace::Uniform | AddressSpace::Storage { .. })
                ) {
                    continue;
                }
                let TypeInner::Array {
                    size: ArraySize::Constant(len),
                    ..
                } = function_info[*base].ty.inner_with(&module.types)
                else {
                    continue;
                };
                if len.get() > MAX_DYNAMICALLY_INDEXED_CONSTANT_ARRAY {
                    findings.push(format!(
                        "{label} indexes a {}-element constant-space array with a runtime value",
                        len.get()
                    ));
                }
            }
        };
        for (handle, function) in module.functions.iter() {
            scan(function, &info[handle], "function");
        }
        for (index, entry) in module.entry_points.iter().enumerate() {
            scan(&entry.function, info.get_entry_point(index), &entry.name);
        }
        findings
    }

    fn parse_and_validate(
        variant: &ShaderVariant,
    ) -> (wgpu::naga::Module, wgpu::naga::valid::ModuleInfo) {
        let module = wgpu::naga::front::wgsl::parse_str(variant.source).unwrap_or_else(|error| {
            panic!(
                "{} parses: {}",
                variant.id,
                error.emit_to_string(variant.source)
            )
        });
        let info = wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap_or_else(|error| panic!("{} validates: {error}", variant.id));
        // Backend writers reject modules with unresolved `override` constants.
        // Resolving with the defaults keeps every budget measured against the
        // full shader, which is what an unspecialized pipeline compiles.
        let (module, info) = wgpu::naga::back::pipeline_constants::process_overrides(
            &module,
            &info,
            None,
            &wgpu::naga::back::PipelineConstants::default(),
        )
        .map(|(module, info)| (module.into_owned(), info.into_owned()))
        .unwrap_or_else(|error| panic!("{} resolves overrides: {error}", variant.id));
        (module, info)
    }

    #[test]
    fn no_production_shader_indexes_a_large_constant_array_dynamically() {
        let mut offenders = Vec::new();
        for variant in production_shader_variants() {
            let (module, info) = parse_and_validate(&variant);
            for finding in dynamic_constant_array_indexes(&module, &info) {
                offenders.push(format!("{}: {finding}", variant.id));
            }
        }
        assert!(
            offenders.is_empty(),
            "move the table into a uniform or storage buffer; hardware without an indexed \
             constant-register file expands each read into a select chain over every element:\n{}",
            offenders.join("\n")
        );
    }

    #[test]
    fn every_webgl2_shader_variant_lowers_to_glsl_es_300() {
        use wgpu::naga::back::glsl;
        let mut measured: Vec<(String, usize)> = Vec::new();
        for variant in production_shader_variants() {
            if variant.profile != ShaderProfile::WebGl2Compatible {
                continue;
            }
            let (module, info) = parse_and_validate(&variant);
            for entry in module.entry_points.iter() {
                let mut source = String::new();
                let options = glsl::Options {
                    version: glsl::Version::new_gles(300),
                    ..Default::default()
                };
                let pipeline = glsl::PipelineOptions {
                    shader_stage: entry.stage,
                    entry_point: entry.name.clone(),
                    multiview: None,
                };
                let mut writer = glsl::Writer::new(
                    &mut source,
                    &module,
                    &info,
                    &options,
                    &pipeline,
                    wgpu::naga::proc::BoundsCheckPolicies::default(),
                )
                .unwrap_or_else(|error| {
                    panic!(
                        "{} {} builds a GLSL ES 3.00 writer: {error}",
                        variant.id, entry.name
                    )
                });
                writer.write().unwrap_or_else(|error| {
                    panic!(
                        "{} {} lowers to GLSL ES 3.00: {error}",
                        variant.id, entry.name
                    )
                });
                measured.push((format!("{} {}", variant.id, entry.name), source.len()));
            }
        }
        let over: Vec<_> = measured
            .iter()
            .filter(|(_, bytes)| *bytes > MAX_WEBGL2_ENTRY_POINT_GLSL_BYTES)
            .collect();
        assert!(
            over.is_empty(),
            "GLSL ES 3.00 budget is {MAX_WEBGL2_ENTRY_POINT_GLSL_BYTES} bytes; measured {measured:?}"
        );
    }

    #[test]
    fn every_production_entry_point_stays_inside_its_spirv_budget() {
        let mut measured: Vec<(String, usize)> = Vec::new();
        for variant in production_shader_variants() {
            let (module, info) = parse_and_validate(&variant);
            for entry in module.entry_points.iter() {
                let words = wgpu::naga::back::spv::write_vec(
                    &module,
                    &info,
                    &wgpu::naga::back::spv::Options::default(),
                    Some(&wgpu::naga::back::spv::PipelineOptions {
                        shader_stage: entry.stage,
                        entry_point: entry.name.clone(),
                    }),
                )
                .unwrap_or_else(|error| {
                    panic!("{} {} lowers to SPIR-V: {error}", variant.id, entry.name)
                });
                let mut offset = 5usize;
                let mut instructions = 0usize;
                while offset < words.len() {
                    let word_count = (words[offset] >> 16) as usize;
                    if word_count == 0 {
                        break;
                    }
                    instructions += 1;
                    offset += word_count;
                }
                measured.push((format!("{} {}", variant.id, entry.name), instructions));
            }
        }
        let over: Vec<_> = measured
            .iter()
            .filter(|(_, count)| *count > MAX_ENTRY_POINT_SPIRV_INSTRUCTIONS)
            .collect();
        assert!(
            over.is_empty(),
            "SPIR-V budget is {MAX_ENTRY_POINT_SPIRV_INSTRUCTIONS} instructions; measured {measured:?}"
        );
    }
}
