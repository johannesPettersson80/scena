use crate::app::prelude::*;

mod q04_evidence;

pub(crate) fn check_c09_gpu_resource_lifecycle_contracts(root: &Path, findings: &mut Vec<Finding>) {
    q04_evidence::check_q04_required_gpu_lifecycle_evidence(root, findings);
    const RULE: &str = "RENDER-C09";
    let required: &[(&str, &[&str])] = &[
        (
            "src/render.rs",
            &[
                "pub enum RenderReadbackMode",
                "pub fn render_with_readback_mode",
                "pub fn render_batch_with_async_readback",
                "NotPreparedReason::OutputSettingsChanged",
                "self.last_render_work_metrics",
            ],
        ),
        (
            "src/render/settings.rs",
            &[
                "self.output_resources_revision = self.output_resources_revision.saturating_add(1)",
                "pub const fn output_resources_revision",
            ],
        ),
        (
            "src/render/prepare_lifecycle.rs",
            &[
                "let output_plan = gpu::GpuOutputPlan::new(",
                "output_plan,",
                "self.stats.gpu_textures = stats.textures",
            ],
        ),
        (
            "src/render/gpu/post/resources.rs",
            &[
                "let depth_texture_bind_groups = depth_color_view.map",
                "create_depth_texture_bind_group(",
                "depth_texture_bind_groups,",
                "scena.gpu_post.texture_pipeline_layout",
                "scena.gpu_post.depth_pipeline_layout",
                "fxaa::create_pipelines",
                "optional_surface_pipelines + 6 - u64::from(resources.surface_fxaa_pipeline.is_some())",
                "scena.gpu_post.uniform_staging",
                "POST_UNIFORM_SLOT_COUNT",
                "wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST",
            ],
        ),
        (
            "src/render/gpu/post/mod.rs",
            &[
                "copy::copy_output_to_buffer",
                "PostUniformSlot::Bloom",
                "PostUniformSlot::Fxaa",
                "encoder.copy_buffer_to_buffer(",
                "&resources.uniform_staging",
            ],
        ),
        (
            "src/render/gpu/post/fxaa.rs",
            &[
                "create_post_shader",
                "create_post_pipeline_with_shader",
                "let shader = create_post_shader",
            ],
        ),
        (
            "src/render/gpu/prepare_resources.rs",
            &[
                "output_plan: GpuOutputPlan",
                "super::post::create_resources(",
                "super::msaa::create_msaa_color_resources(",
                "depth::create_depth_prepass_resources(",
                "PrepareError::UnsupportedSampleCount",
                "buffers: 6",
            ],
        ),
        (
            "src/render/gpu/headless_target.rs",
            &[
                "let readback = std::array::from_fn",
                "wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ",
                "scena.headless_gpu.target",
            ],
        ),
        (
            "src/render/gpu/prepare_resources_wasm.rs",
            &[
                "output_plan: GpuOutputPlan",
                "super::post::create_resources(",
                "feature = \"scene-host\"",
                "create_browser_readback_resources(",
                "depth::create_depth_prepass_resources(",
                "PrepareError::UnsupportedSampleCount",
            ],
        ),
        (
            "src/render/gpu/draw_surface_support.rs",
            &[
                "pub(in crate::render) async fn browser_readback_rgba8",
                "renderer-owned WebGPU readback failed",
                "Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb",
                "pixel.swap(0, 2)",
            ],
        ),
        (
            "src/render/gpu/draw_surface.rs",
            &[
                "if !post_enabled",
                "surface_readback.is_none()",
                "let Some(readback) = resources.readback.as_ref()",
                "let chain_settings = if renderer_readback.is_some()",
                "scena.browser.capture_overlay_final_pass",
                "post::copy_output_to_buffer(",
                "surface.config.usage.contains(wgpu::TextureUsages::COPY_SRC)",
                "encode_texture_readback_copy(",
            ],
        ),
        (
            "src/render/gpu/draw_surface_probe.rs",
            &[
                "render_browser_probe",
                "scena.browser.proof_encoder",
                "post::copy_output_to_buffer(",
            ],
        ),
        (
            "src/render/gpu/browser_readback.rs",
            &[
                "feature = \"scene-host\"",
                "create_browser_readback_resources",
                "encode_texture_readback_copy",
            ],
        ),
        (
            "src/render/gpu/build.rs",
            &[
                "enable_scene_host_surface_readback",
                "capabilities.usages.contains(wgpu::TextureUsages::COPY_SRC)",
                "config.usage |= wgpu::TextureUsages::COPY_SRC",
                "browser_instance_descriptor",
                "descriptor.backend_options.gl.fence_behavior = wgpu::GlFenceBehavior::AutoFinish;",
            ],
        ),
        (
            "src/render/offscreen.rs",
            &["pub(crate) async fn browser_readback_rgba8"],
        ),
        (
            "src/scene_host/wasm_capture.rs",
            &[
                "capture_rgba8_for_wasm_async",
                "browser GPU capture readback failed",
            ],
        ),
        (
            "src/scene_host/wasm.rs",
            &[
                "js_name = captureAsync",
                "js_name = capturePngAsync",
                "js_name = captureJsonAsync",
            ],
        ),
        (
            "src/scene_host/wasm_introspection.rs",
            &[
                "js_name = renderIntrospectionJsonAsync",
                "capture_rgba8_for_wasm_async",
            ],
        ),
        (
            "src/render/gpu/stats.rs",
            &[
                "pub(in crate::render) fn destruction_records",
                "self.buffers",
                "+ self.textures",
                "+ self.pipelines",
                "+ self.bind_groups",
            ],
        ),
        (
            "src/render/gpu/lifecycle.rs",
            &[
                "browser_uses_automatic_resource_retirement",
                "self.device.poll(wgpu::PollType::Poll)",
                "wgpu::Backend::Gl | wgpu::Backend::BrowserWebGpu",
                "Browser WebGPU's Device::poll is automatic/no-op",
                "DevicePollStatus::Automatic",
                "DevicePollStatus::Unsupported",
            ],
        ),
        (
            "src/browser_probe/probes/state_lifecycle.rs",
            &[
                "fn verify_resource_lifetime",
                "completion_confirmed",
                "automatic-webgl2",
                "automatic-webgpu",
                "DevicePollStatus::Confirmed",
            ],
        ),
        (
            "tests/browser/m6_rust_wasm_renderer_probe.js",
            &[
                "expectedRetirementMode",
                "submitted_poll_status",
                "completion_poll_status",
                "\"automatic-webgl2\" : \"automatic-webgpu\"",
                "completion_confirmed !== false",
            ],
        ),
        (
            "tests/pf01_output_toggle.rs",
            &[
                "pf01_native_gpu_output_toggle_renders_off_on_off_without_lazy_resources",
                "SCENA_REQUIRE_HARDWARE_GPU",
                "RenderReadbackMode::Synchronous",
                "zero-render-time-gpu-object-creation",
                "off-again-determinism",
            ],
        ),
        (
            "tests/browser/pf01_output_toggle.js",
            &[
                "scena.pf01.browser_output_toggle.v1",
                "evaluateRequiredHardwareAdapter",
                "SCENA_REQUIRE_PARITY",
                "await host.captureAsync()",
                "collectBrowserGpuEvidence",
                "browserGpu,",
                "/assets/gltf/exploded_view_assembly.gltf",
                "validateOutputToggleResult(result)",
                "page.on(\"response\"",
                "page.on(\"requestfailed\"",
                "unexpected HTTP failures",
            ],
        ),
        (
            "tests/browser/pf01_output_toggle_validation.js",
            &[
                "validateOutputToggleResult",
                "bloomOnly.fnv1a64 === off.fnv1a64",
                "fxaaOnly.fnv1a64 === off.fnv1a64",
                "on.fnv1a64 === bloomOnly.fnv1a64",
                "on.fnv1a64 === fxaaOnly.fnv1a64",
                "bloom-only output is identical to baseline",
                "FXAA-only output is identical to baseline",
                "combined output is identical to bloom-only",
                "combined output is identical to FXAA-only",
                "off-again output is not deterministic",
                "render changed its prepared resource shape",
                "module.exports = { validateOutputToggleResult }",
            ],
        ),
        (
            "tests/browser/hardware_browser.js",
            &[
                "SCENA_WEBGPU_BROWSER",
                "SCENA_WEBGL2_BROWSER",
                "chromiumArgsForPlatform",
                "platform === \"linux\"",
                "--enable-features=Vulkan,WebGPU",
                "--enable-features=WebGPU",
                "firefoxUserPrefs",
                "dom.webgpu.enabled",
                "gfx.webgpu.force-enabled",
                "SystemInfo.getInfo",
                "chromium-cdp-system-info",
                "sanitizeChromiumGpuInfo",
            ],
        ),
        (
            "tests/browser/required_gpu_parity.js",
            &[
                "softwareBrowserGpu",
                "hardwareBrowserGpu",
                "browserGpu.source !== \"chromium-cdp-system-info\"",
                "devices.every",
                "renderer.includes(deviceIdentity)",
                "!hardwareAdapter(adapter) && !hardwareBrowserGpu(browserGpu)",
            ],
        ),
        (
            "tests/browser/fr06_semantic_aov.js",
            &[
                "collectBrowserGpuEvidence",
                "browserGpu,",
                "page.on(\"response\"",
                "page.on(\"requestfailed\"",
                "unexpected HTTP failures",
            ],
        ),
        (
            "examples/native_surface_hardware_proof.rs",
            &[
                "scena.pf01_pf02.native_surface_hardware_proof.v1",
                "PlatformSurface::native_window_handle",
                "prepare_with_assets",
                "RenderReadbackMode::PresentOnly",
                "RenderReadbackMode::Synchronous",
                "require_hardware_adapter",
                "cpu_frame_copy_bytes",
                "SCENA_HARDWARE_PROOF_ROOT",
                "bloom_only",
                "fxaa_only",
                "off_again",
                "native combined output collapsed to bloom-only",
                "native combined output collapsed to FXAA-only",
                "native off-again output did not restore baseline pixels",
                "resize_lifecycle",
                "target_changed_requires_prepare",
                "surface_loss_handling",
                "host_surface_recreation_required",
                "output_toggle",
            ],
        ),
        (
            "tests/release/windows_complete_hardware_proof_validation.js",
            &[
                "scena.windows_complete_hardware_proof.v1",
                "validateOutputToggleResult(backend)",
                "native present-only ${counter} must be zero",
                "native combined output is identical to bloom-only",
                "native combined output is identical to FXAA-only",
                "missing visual artifact",
                "validateNativeSurface",
                "validateNativeFr06",
                "validateFr06Browser",
                "validateQ01Parity",
                "validateQ04Lifecycle",
                "validateP01Benchmark",
                "native_surface_resize_recovery",
            ],
        ),
        (
            "scripts/run_windows_complete_hardware_proof.ps1",
            &[
                "bundle-files.sha256",
                "source-commit.txt",
                "Proof bundle source commit",
                "test:required-gpu-parity",
                "browser:q01-parity",
                "browser:pf01-output-toggle",
                "browser:fr06-semantic-aov",
                "scena-native-hardware-proof.exe",
                "scena-fr06-native-hardware-proof.exe",
                "scena-q04-gpu-resource-lifecycle.exe",
                "scena-p01-shader-module-cache.exe",
                "SCENA_RUN_CONTROLLED_P01_BENCHMARK",
                "SCENA_REQUIRE_HARDWARE_GPU",
                "windows_complete_hardware_proof_validation.js",
                "Compress-Archive",
                "Invoke-WebRequest -UseBasicParsing -Method Put",
            ],
        ),
        (
            "scripts/build_windows_complete_hardware_bundle.sh",
            &[
                "Windows release-evidence bundles require a clean committed checkout",
                "wasm-pack 0.14.0",
                "x86_64-pc-windows-gnu",
                "scena-native-hardware-proof.exe",
                "scena-fr06-native-hardware-proof.exe",
                "scena-q04-gpu-resource-lifecycle.exe",
                "scena-p01-shader-module-cache.exe",
                "bundle-files.sha256",
                "source-commit.txt",
            ],
        ),
        (
            "package.json",
            &[
                "browser:pf01-output-toggle",
                "tests/browser/pf01_output_toggle.js",
                "test:pf01-output-toggle-validation",
                "tests/browser/pf01_output_toggle_validation_test.js",
            ],
        ),
        (
            ".github/workflows/hardware-gpu.yml",
            &[
                "runs-on: [self-hosted, linux, x64, gpu, scena-gpu]",
                "SCENA_REQUIRE_HARDWARE_GPU: \"1\"",
                "SCENA_REQUIRE_PARITY: \"1\"",
                "cargo test --test pf01_output_toggle",
                "cargo run --example native_surface_hardware_proof",
                "npm run browser:pf01-output-toggle",
            ],
        ),
        (
            "docs/api.md",
            &[
                "GPU resource lifecycle invariant",
                "RendererStats::gpu_textures",
                "DevicePollStatus::Automatic",
                "DevicePollStatus::Confirmed",
                "RenderReadbackMode::PresentOnly",
                "RenderReadbackMode::Synchronous",
                "OutputSettingsChanged",
                "render_batch_with_async_readback",
                "wgpu::PipelineCache",
                "captureAsync()",
                "renderIntrospectionJsonAsync(detail)",
            ],
        ),
        (
            "docs/browser.md",
            &[
                "capturePngAsync()",
                "renderIntrospectionJsonAsync(detail)",
                "WebGPU canvas requires asynchronous buffer mapping",
            ],
        ),
    ];
    for (relative, needles) in required {
        require_contains(root, findings, RULE, relative, needles);
    }

    if fs::read_to_string(root.join("src/render/gpu/lifecycle.rs"))
        .is_ok_and(|source| source.contains("on_submitted_work_done"))
    {
        findings.push(Finding::new(
            RULE,
            "browser logical resource retirement must not wait on \
             on_submitted_work_done because browser WebGPU owns in-flight object lifetime",
        ));
    }

    if fs::read_to_string(root.join(".github/workflows/hardware-gpu.yml"))
        .is_ok_and(|source| source.contains("SCENA_ALLOW_PARTIAL_HARDWARE_BACKENDS"))
    {
        findings.push(Finding::new(
            RULE,
            ".github/workflows/hardware-gpu.yml must not set \
             SCENA_ALLOW_PARTIAL_HARDWARE_BACKENDS because partial backend artifacts are \
             diagnostic-only and never release evidence",
        ));
    }

    if fs::read_to_string(root.join("src/render/gpu/post/mod.rs"))
        .is_ok_and(|source| source.contains("create_pipeline_layout"))
    {
        findings.push(Finding::new(
            RULE,
            "post pipeline helper must consume shared layouts instead of creating one per pipeline",
        ));
    }

    if let Ok(source) = fs::read_to_string(root.join("src/render/gpu/post/mod.rs")) {
        const EXPORT: &str = "pub(super) use copy::copy_output_to_buffer;";
        if let Some(export_offset) = source.find(EXPORT) {
            let attribute_block_start = source[..export_offset]
                .rfind("\n\n")
                .map_or(0, |offset| offset + 2);
            if source[attribute_block_start..export_offset].contains("#[cfg") {
                findings.push(Finding::new(
                    RULE,
                    "src/render/gpu/post/mod.rs must export copy_output_to_buffer \
                     unconditionally because the plain wasm32 surface path calls it without \
                     browser-probe or scene-host features",
                ));
            }
        }
    }

    if fs::read_to_string(root.join("src/render/gpu/post/mod.rs"))
        .is_ok_and(|source| source.contains("queue.write_buffer(&resources.uniform,"))
    {
        findings.push(Finding::new(
            RULE,
            "src/render/gpu/post/mod.rs writes pass parameters directly into the shared post \
             uniform; command-ordered staging copies are required so later queue writes cannot \
             overwrite earlier passes",
        ));
    }

    const COMBINED_ORACLE_NEEDLES: &[&str] = &[
        "bloomOnly.fnv1a64 === off.fnv1a64",
        "fxaaOnly.fnv1a64 === off.fnv1a64",
        "on.fnv1a64 === bloomOnly.fnv1a64",
        "on.fnv1a64 === fxaaOnly.fnv1a64",
    ];
    if fs::read_to_string(root.join("tests/browser/pf01_output_toggle_validation.js")).map_or(
        true,
        |source| {
            COMBINED_ORACLE_NEEDLES
                .iter()
                .any(|needle| !source.contains(needle))
        },
    ) {
        findings.push(Finding::new(
            RULE,
            "PF01 browser oracle must reject inert single effects and combined-effect collapse \
             to either bloom-only or FXAA-only output",
        ));
    }

    for relative in ["src/render/gpu/draw.rs", "src/render/gpu/draw_surface.rs"] {
        let Ok(source) = fs::read_to_string(root.join(relative)) else {
            continue;
        };
        for forbidden in [
            "post::create_resources",
            "create_depth_prepass_resources",
            "create_msaa_color_resources",
        ] {
            if source.contains(forbidden) {
                findings.push(Finding::new(
                    RULE,
                    format!(
                        "{relative} contains forbidden render-time GPU allocation `{forbidden}`"
                    ),
                ));
            }
        }
    }

    for relative in ["src/render/gpu/post/ssao.rs", "src/render/gpu/post/dof.rs"] {
        let Ok(source) = fs::read_to_string(root.join(relative)) else {
            findings.push(Finding::new(RULE, format!("could not read {relative}")));
            continue;
        };
        if source.contains("create_bind_group") {
            findings.push(Finding::new(
                RULE,
                format!("{relative} creates a bind group in the render-time encode path"),
            ));
        }
    }

    if let Ok(source) = fs::read_to_string(root.join("src/render/gpu/lifecycle.rs")) {
        for forbidden in [
            "self.pending_destructions = 0;\n        (pending, DevicePollStatus::Confirmed)",
            "self.pending_destructions = 0;\n        (pending, DevicePollStatus::Automatic)",
            "scena.resource_destruction_completion",
        ] {
            if source.contains(forbidden) {
                findings.push(Finding::new(
                    RULE,
                    format!(
                        "src/render/gpu/lifecycle.rs contains forbidden fabricated completion `{forbidden}`"
                    ),
                ));
            }
        }
    }

    if let Ok(source) = fs::read_to_string(root.join("src/render/gpu/stats.rs")) {
        for forbidden in [
            "estimate_prepared_resource_stats",
            "PreparedResourceEstimateInput",
        ] {
            if source.contains(forbidden) {
                findings.push(Finding::new(
                    RULE,
                    format!(
                        "src/render/gpu/stats.rs contains forbidden aggregate estimate `{forbidden}`"
                    ),
                ));
            }
        }
    }
}
