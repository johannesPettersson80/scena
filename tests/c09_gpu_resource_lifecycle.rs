#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use scena::{
    AdapterLimitsReport, AntiAliasing, Assets, CaptureError, DepthOfFieldConfig, DevicePollStatus,
    GpuAdapterReport, PerspectiveCamera, PostBloomConfig, PrepareError, Primitive, RenderError,
    RenderReadbackMode, Renderer, RendererStats, Scene, ScreenSpaceAmbientOcclusionConfig,
    ScreenSpaceReflectionConfig, SurfaceEvent, Transform, Vec3,
};
use serde::Serialize;

const REQUIRED_LIFECYCLE_ENV: &str = "SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE";
const LIFECYCLE_ARTIFACT_DIR: &str = "target/gate-artifacts/c09-gpu-resource-lifecycle";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
struct ResourceCounters {
    buffers: u64,
    gpu_textures: u64,
    render_targets: u64,
    pipelines: u64,
    bind_groups: u64,
    shader_modules: u64,
    pending_destructions: u64,
}

impl From<RendererStats> for ResourceCounters {
    fn from(stats: RendererStats) -> Self {
        Self {
            buffers: stats.buffers,
            gpu_textures: stats.gpu_textures,
            render_targets: stats.render_targets,
            pipelines: stats.pipelines,
            bind_groups: stats.bind_groups,
            shader_modules: stats.shader_modules,
            pending_destructions: stats.pending_destructions,
        }
    }
}

impl ResourceCounters {
    const fn retained_shape(self) -> [u64; 6] {
        [
            self.buffers,
            self.gpu_textures,
            self.render_targets,
            self.pipelines,
            self.bind_groups,
            self.shader_modules,
        ]
    }

    fn retained_total(self) -> u64 {
        self.retained_shape().into_iter().sum()
    }
}

#[derive(Debug, Clone, Serialize)]
struct RequiredLifecycleEvidence {
    schema: String,
    status: String,
    proof_class: String,
    producer: String,
    command: String,
    commit_sha: String,
    timestamp_unix_seconds: u64,
    adapter: Option<GpuAdapterReport>,
    baseline: ResourceCounters,
    prepared: ResourceCounters,
    released: ResourceCounters,
    poll_status: String,
    poll_pending_before: u64,
    poll_destroyed_resources: u64,
    poll_pending_after: u64,
    assertions_executed: u64,
    complete_lifecycle: bool,
}

impl RequiredLifecycleEvidence {
    fn known_good() -> Self {
        Self {
            schema: "scena.q04.required_gpu_resource_lifecycle.v1".to_string(),
            status: "passed".to_string(),
            proof_class: "physical-hardware-required".to_string(),
            producer: "mutation-test".to_string(),
            command: "mutation-test".to_string(),
            commit_sha: "test-commit".to_string(),
            timestamp_unix_seconds: 1,
            adapter: Some(GpuAdapterReport {
                name: "Mutation Test GPU".to_string(),
                backend: "Vulkan".to_string(),
                device_type: "DiscreteGpu".to_string(),
                vendor: 1,
                device: 2,
                driver: "test-driver".to_string(),
                driver_info: "hardware".to_string(),
                features: String::new(),
                limits: AdapterLimitsReport {
                    max_texture_dimension_2d: 8192,
                    max_bind_groups: 4,
                    max_uniform_buffer_binding_size: 65_536,
                    max_vertex_attributes: 16,
                },
            }),
            baseline: ResourceCounters {
                buffers: 10,
                gpu_textures: 20,
                render_targets: 4,
                pipelines: 9,
                bind_groups: 6,
                shader_modules: 3,
                pending_destructions: 0,
            },
            prepared: ResourceCounters {
                buffers: 12,
                gpu_textures: 27,
                render_targets: 11,
                pipelines: 21,
                bind_groups: 12,
                shader_modules: 10,
                pending_destructions: 0,
            },
            released: ResourceCounters {
                buffers: 10,
                gpu_textures: 20,
                render_targets: 4,
                pipelines: 9,
                bind_groups: 6,
                shader_modules: 3,
                pending_destructions: 72,
            },
            poll_status: "Confirmed".to_string(),
            poll_pending_before: 72,
            poll_destroyed_resources: 72,
            poll_pending_after: 0,
            assertions_executed: 12,
            complete_lifecycle: true,
        }
    }
}

fn validate_required_lifecycle_evidence(
    evidence: &RequiredLifecycleEvidence,
) -> Result<(), String> {
    if evidence.schema != "scena.q04.required_gpu_resource_lifecycle.v1" {
        return Err("required lifecycle evidence has the wrong schema".to_string());
    }
    if evidence.status != "passed" || evidence.proof_class != "physical-hardware-required" {
        return Err("required lifecycle evidence is not a passed hardware proof".to_string());
    }
    let adapter = evidence
        .adapter
        .as_ref()
        .ok_or_else(|| "required lifecycle evidence is missing adapter provenance".to_string())?;
    require_hardware_adapter(adapter)?;
    if !evidence.complete_lifecycle || evidence.assertions_executed < 10 {
        return Err(
            "required lifecycle evidence did not execute the complete assertion set".into(),
        );
    }
    if evidence.prepared.retained_total() <= evidence.baseline.retained_total() {
        return Err("heavy preparation did not add tracked GPU resources".to_string());
    }
    if evidence.released.retained_shape() != evidence.baseline.retained_shape() {
        return Err("released resource shape did not return to baseline".to_string());
    }
    if evidence.released.pending_destructions == 0
        || evidence.poll_pending_before != evidence.released.pending_destructions
        || evidence.poll_destroyed_resources != evidence.poll_pending_before
        || evidence.poll_pending_after != evidence.baseline.pending_destructions
    {
        return Err("pending destructions were not completely confirmed and retired".to_string());
    }
    if evidence.poll_status != "Confirmed" {
        return Err("device poll did not confirm GPU completion".to_string());
    }
    Ok(())
}

fn require_hardware_adapter(adapter: &GpuAdapterReport) -> Result<(), String> {
    if !matches!(
        adapter.device_type.as_str(),
        "DiscreteGpu" | "IntegratedGpu" | "VirtualGpu"
    ) {
        return Err(format!(
            "required lifecycle adapter is not a hardware device type: {}",
            adapter.device_type
        ));
    }
    let identity = format!(
        "{} {} {} {}",
        adapter.name, adapter.device_type, adapter.driver, adapter.driver_info
    )
    .to_ascii_lowercase();
    for marker in [
        "swiftshader",
        "llvmpipe",
        "lavapipe",
        "software rasterizer",
        "microsoft basic render",
    ] {
        if identity.contains(marker) {
            return Err(format!(
                "required lifecycle adapter contains software marker {marker}"
            ));
        }
    }
    Ok(())
}

fn optional_gpu_renderer(test_name: &str) -> Option<Renderer> {
    match Renderer::headless_gpu(16, 16) {
        Ok(renderer) => Some(renderer),
        Err(error) => {
            write_lifecycle_artifact(
                &format!("optional-{test_name}.json"),
                &serde_json::json!({
                    "schema": "scena.q04.optional_gpu_resource_lifecycle_smoke.v1",
                    "status": "skipped",
                    "proof_class": "optional-developer-smoke",
                    "test": test_name,
                    "reason": format!("{error:?}"),
                    "producer": "cargo test --test c09_gpu_resource_lifecycle",
                    "commit_sha": current_commit_label(),
                    "timestamp_unix_seconds": current_timestamp_unix_seconds(),
                }),
            );
            None
        }
    }
}

fn write_lifecycle_artifact(name: &str, value: &impl Serialize) {
    let path = Path::new(LIFECYCLE_ARTIFACT_DIR).join(name);
    fs::create_dir_all(path.parent().expect("Q04 artifact parent exists"))
        .expect("Q04 artifact directory creates");
    let body = serde_json::to_string_pretty(value).expect("Q04 artifact serializes");
    fs::write(&path, format!("{body}\n")).expect("Q04 artifact writes");
}

fn current_commit_label() -> String {
    std::env::var("GITHUB_SHA")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "local-checkout".to_string())
}

fn current_timestamp_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn gpu_scene() -> (Scene, scena::CameraKey) {
    let mut scene = Scene::new();
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("C09 triangle inserts");
    let camera = scene
        .add_default_camera()
        .expect("C09 default camera inserts");
    (scene, camera)
}

#[test]
fn required_lifecycle_evaluator_rejects_known_leak_and_missing_adapter() {
    let valid = RequiredLifecycleEvidence::known_good();
    assert!(validate_required_lifecycle_evidence(&valid).is_ok());

    let mut leaked = valid.clone();
    leaked.poll_pending_after = 1;
    assert!(
        validate_required_lifecycle_evidence(&leaked)
            .expect_err("known leak must fail")
            .contains("pending destructions")
    );

    let mut missing_adapter = valid;
    missing_adapter.adapter = None;
    assert!(
        validate_required_lifecycle_evidence(&missing_adapter)
            .expect_err("missing adapter must fail")
            .contains("adapter")
    );
}

#[test]
fn msaa8_is_fully_prepared_or_rejected_before_render_optional_gpu_smoke() {
    let Some(mut renderer) = optional_gpu_renderer("msaa8") else {
        return;
    };
    let (mut scene, camera) = gpu_scene();
    disable_heavy_output(&mut renderer);
    renderer.prepare(&mut scene).expect("baseline prepares");
    let baseline = renderer.stats();

    renderer.set_anti_aliasing(AntiAliasing::Msaa8);
    match renderer.prepare(&mut scene) {
        Ok(()) => {
            let prepared = renderer.stats();
            assert_eq!(
                (
                    prepared.buffers,
                    prepared.gpu_textures,
                    prepared.render_targets,
                    prepared.pipelines,
                    prepared.bind_groups,
                    prepared.shader_modules,
                ),
                (10, 22, 6, 13, 6, 4),
                "supported MSAA8 owns its pipeline set, color target, overlay depth set, and shared triangle shader before render"
            );
            assert!(prepared.approximate_gpu_memory_bytes > baseline.approximate_gpu_memory_bytes);
            let signature = resource_signature(prepared);
            renderer
                .render(&scene, camera)
                .expect("prepared MSAA8 renders without lazy creation");
            assert_eq!(resource_signature(renderer.stats()), signature);
        }
        Err(PrepareError::UnsupportedSampleCount {
            requested, maximum, ..
        }) => {
            assert_eq!(requested, 8);
            assert!(maximum < requested);
        }
        Err(error) => panic!("unexpected MSAA8 prepare result: {error:?}"),
    }
}

fn resource_signature(stats: RendererStats) -> (u64, u64, u64, u64, u64, u64, Option<u64>) {
    (
        stats.buffers,
        stats.gpu_textures,
        stats.render_targets,
        stats.pipelines,
        stats.bind_groups,
        stats.shader_modules,
        stats.approximate_gpu_memory_bytes,
    )
}

fn enable_heavy_output(renderer: &mut Renderer) {
    renderer.set_anti_aliasing(AntiAliasing::Msaa4);
    renderer.set_bloom(Some(PostBloomConfig::subtle()));
    renderer.set_screen_space_ambient_occlusion(Some(ScreenSpaceAmbientOcclusionConfig::new(
        3, 0.5, 0.015,
    )));
    renderer.set_screen_space_reflections(Some(ScreenSpaceReflectionConfig::default()));
    renderer.set_depth_of_field(Some(DepthOfFieldConfig::new(2.5, 1.2, 6)));
}

fn disable_heavy_output(renderer: &mut Renderer) {
    renderer.set_anti_aliasing(AntiAliasing::None);
    renderer.clear_bloom();
    renderer.clear_screen_space_ambient_occlusion();
    renderer.clear_screen_space_reflections();
    renderer.clear_depth_of_field();
}

#[test]
fn output_resource_changes_require_prepare_and_stats_are_complete_before_render_optional_gpu_smoke()
{
    let Some(mut renderer) = optional_gpu_renderer("output-resource-changes") else {
        return;
    };
    let (mut scene, camera) = gpu_scene();
    disable_heavy_output(&mut renderer);
    renderer.prepare(&mut scene).expect("baseline prepares");
    let baseline = renderer.stats();
    assert_eq!(
        (
            baseline.buffers,
            baseline.gpu_textures,
            baseline.render_targets,
            baseline.pipelines,
            baseline.bind_groups,
            baseline.shader_modules,
        ),
        (10, 20, 4, 9, 6, 3),
        "the simple native baseline inventories core, light assignment, fallback material, shadow/environment, transmission, and depth owners exactly"
    );
    assert!(
        baseline.gpu_textures > 0,
        "baseline GPU texture owners are counted"
    );

    enable_heavy_output(&mut renderer);
    assert!(
        matches!(
            renderer.render(&scene, camera),
            Err(RenderError::NotPrepared { .. })
        ),
        "resource-affecting AA/post changes must invalidate prepared output resources"
    );

    renderer
        .prepare(&mut scene)
        .expect("MSAA/post resources prepare explicitly");
    let prepared = renderer.stats();
    assert_eq!(
        (
            prepared.buffers,
            prepared.gpu_textures,
            prepared.render_targets,
            prepared.pipelines,
            prepared.bind_groups,
            prepared.shader_modules,
        ),
        (12, 27, 11, 21, 12, 10),
        "MSAA4 + post + command-ordered uniform staging + depth-color + overlay owners are additive and exact"
    );
    assert!(
        prepared.buffers == baseline.buffers + 2,
        "post owns both its live uniform and a command-ordered staging buffer"
    );
    assert!(
        prepared.render_targets > baseline.render_targets,
        "MSAA, post, and depth-color targets must be counted before render"
    );
    assert!(
        prepared.gpu_textures > baseline.gpu_textures,
        "post, MSAA, and depth-color texture owners must be counted before render"
    );
    assert!(
        prepared.pipelines > baseline.pipelines,
        "post and MSAA pipelines must be counted before render"
    );
    assert!(
        prepared.bind_groups > baseline.bind_groups,
        "post bind groups must be counted before render"
    );
    assert!(
        prepared.shader_modules > baseline.shader_modules,
        "post and MSAA shader modules must be counted before render"
    );
    assert!(
        prepared.approximate_gpu_memory_bytes > baseline.approximate_gpu_memory_bytes,
        "post, MSAA, and depth-color texture bytes must be included before render"
    );

    let before_render = resource_signature(prepared);
    renderer
        .render(&scene, camera)
        .expect("fully prepared heavy output renders");
    assert_eq!(
        resource_signature(renderer.stats()),
        before_render,
        "render must not create, destroy, or revise reported GPU resources"
    );

    disable_heavy_output(&mut renderer);
    renderer
        .prepare(&mut scene)
        .expect("disabling heavy output reprepares baseline resources");
    let released = renderer.stats();
    assert_eq!(
        resource_signature(released),
        resource_signature(baseline),
        "disabling all optional output resources must return exact counters to baseline"
    );
    assert_eq!(
        released.pending_destructions,
        prepared.buffers + prepared.gpu_textures + prepared.pipelines + prepared.bind_groups,
        "pending destruction counts each tracked live GPU object exactly once; render targets are textures and prepared shader modules are compilation inputs, not retained objects"
    );
    let poll = renderer.poll_device();
    assert_eq!(poll.status, DevicePollStatus::Confirmed);
    assert_eq!(
        poll.pending_destructions_before,
        released.pending_destructions
    );
    assert_eq!(poll.destroyed_resources, released.pending_destructions);
    assert_eq!(poll.pending_destructions_after, 0);
}

#[test]
fn cpu_poll_reports_explicitly_unsupported_instead_of_success() {
    let mut renderer = Renderer::headless(4, 4).expect("CPU renderer builds");
    let poll = renderer.poll_device();
    assert_eq!(poll.status, DevicePollStatus::Unsupported);
    assert_eq!(poll.destroyed_resources, 0);
    assert!(!poll.gpu_polled);
}

#[test]
fn output_revision_and_native_readback_modes_are_explicit_and_render_allocates_no_gpu_resources_optional_gpu_smoke()
 {
    let Some(mut renderer) = optional_gpu_renderer("readback-modes") else {
        return;
    };
    let (mut scene, camera) = gpu_scene();
    disable_heavy_output(&mut renderer);
    renderer.prepare(&mut scene).expect("baseline prepares");

    enable_heavy_output(&mut renderer);
    assert!(matches!(
        renderer.render(&scene, camera),
        Err(RenderError::NotPrepared {
            reason: scena::NotPreparedReason::OutputSettingsChanged { .. }
        })
    ));
    renderer
        .prepare(&mut scene)
        .expect("output resources prepare before either render mode");

    renderer
        .render_with_readback_mode(&scene, camera, RenderReadbackMode::PresentOnly)
        .expect("present-only render succeeds");
    let present = renderer.last_render_work_metrics();
    assert_eq!(present.readback_copies, 0);
    assert_eq!(present.map_requests, 0);
    assert_eq!(present.blocking_polls, 0);
    assert_eq!(present.blocking_waits, 0);
    assert_eq!(present.cpu_frame_copy_bytes, 0);
    assert_eq!(present.gpu_buffer_creations, 0);
    assert_eq!(present.gpu_texture_creations, 0);
    assert_eq!(present.gpu_pipeline_creations, 0);
    assert_eq!(present.gpu_bind_group_creations, 0);
    assert_eq!(present.gpu_shader_module_creations, 0);
    assert!(matches!(
        renderer.capture_rgba8(&scene, Default::default()),
        Err(CaptureError::NoRenderedFrame)
    ));

    renderer
        .render_with_readback_mode(&scene, camera, RenderReadbackMode::Synchronous)
        .expect("synchronous capture render succeeds");
    let synchronous = renderer.last_render_work_metrics();
    assert_eq!(synchronous.readback_copies, 1);
    assert_eq!(synchronous.map_requests, 1);
    assert_eq!(synchronous.blocking_polls, 1);
    assert_eq!(synchronous.blocking_waits, 1);
    assert_eq!(synchronous.cpu_frame_copy_bytes, 16 * 16 * 4);
    assert_eq!(synchronous.gpu_buffer_creations, 0);
    assert_eq!(synchronous.gpu_texture_creations, 0);
    assert_eq!(synchronous.gpu_pipeline_creations, 0);
    assert_eq!(synchronous.gpu_bind_group_creations, 0);
    assert_eq!(synchronous.gpu_shader_module_creations, 0);
    assert!(renderer.capture_rgba8(&scene, Default::default()).is_ok());
}

#[test]
fn double_buffered_async_readback_batch_preserves_input_order_optional_gpu_smoke() {
    let Some(mut renderer) = optional_gpu_renderer("async-readback") else {
        return;
    };
    let (mut scene, camera) = gpu_scene();
    let alternate = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(8.0, 0.0, 3.0)),
        )
        .expect("alternate C09 camera inserts");
    disable_heavy_output(&mut renderer);
    renderer.prepare(&mut scene).expect("batch scene prepares");

    let frames = renderer
        .render_batch_with_async_readback(&scene, &[camera, alternate, camera])
        .expect("double-buffered batch capture succeeds");
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].rgba8(), frames[2].rgba8());
    assert_ne!(frames[0].rgba8(), frames[1].rgba8());
    assert_eq!(renderer.frame_rgba8(), frames[2].rgba8());

    let metrics = renderer.last_render_work_metrics();
    assert_eq!(metrics.readback_copies, 3);
    assert_eq!(metrics.map_requests, 3);
    assert_eq!(metrics.async_readback_submissions, 3);
    assert_eq!(metrics.peak_readbacks_in_flight, 2);
    assert_eq!(metrics.gpu_buffer_creations, 0);
    assert_eq!(metrics.gpu_texture_creations, 0);
    assert_eq!(metrics.gpu_pipeline_creations, 0);
    assert_eq!(metrics.gpu_bind_group_creations, 0);
    assert_eq!(metrics.gpu_shader_module_creations, 0);
}

#[test]
fn resize_and_context_recovery_rebuild_the_same_resource_shape_optional_gpu_smoke() {
    let Some(mut renderer) = optional_gpu_renderer("resize-context-recovery") else {
        return;
    };
    let assets = Assets::new();
    let (mut scene, camera) = gpu_scene();
    enable_heavy_output(&mut renderer);
    renderer.prepare(&mut scene).expect("heavy output prepares");
    let initial = renderer.stats();

    renderer
        .handle_surface_event(SurfaceEvent::Resize {
            width: 32,
            height: 16,
        })
        .expect("target resizes");
    renderer
        .prepare(&mut scene)
        .expect("resized output prepares");
    let resized = renderer.stats();
    assert_eq!(
        (
            resized.buffers,
            resized.gpu_textures,
            resized.render_targets,
            resized.pipelines,
            resized.bind_groups,
            resized.shader_modules,
        ),
        (
            initial.buffers,
            initial.gpu_textures,
            initial.render_targets,
            initial.pipelines,
            initial.bind_groups,
            initial.shader_modules,
        ),
        "resize must rebuild the same owner shape"
    );
    assert!(
        resized.approximate_gpu_memory_bytes > initial.approximate_gpu_memory_bytes,
        "doubling target pixels must increase exact texture/buffer byte accounting"
    );
    let before_render = resource_signature(resized);
    renderer
        .render(&scene, camera)
        .expect("resized prepared output renders");
    assert_eq!(resource_signature(renderer.stats()), before_render);

    renderer
        .handle_surface_event(SurfaceEvent::ContextLost { recoverable: true })
        .expect("context loss records");
    renderer
        .handle_surface_event(SurfaceEvent::ContextRestored)
        .expect("context restore records");
    renderer
        .recover_context(&assets, &mut scene)
        .expect("retained state recovers");
    renderer
        .prepare(&mut scene)
        .expect("recovered output resources prepare");
    assert_eq!(
        resource_signature(renderer.stats()),
        resource_signature(resized),
        "context recovery must rebuild the complete optional resource shape"
    );
}

#[test]
fn required_hardware_gpu_resource_lifecycle_executes_complete_cycle() {
    if std::env::var(REQUIRED_LIFECYCLE_ENV).as_deref() != Ok("1") {
        write_lifecycle_artifact(
            "required-skip.json",
            &serde_json::json!({
                "schema": "scena.q04.required_gpu_resource_lifecycle.v1",
                "status": "skipped",
                "proof_class": "diagnostic-without-required-hardware-policy",
                "reason": format!("set {REQUIRED_LIFECYCLE_ENV}=1 on a physical GPU lane"),
                "producer": "cargo test --test c09_gpu_resource_lifecycle required_hardware_gpu_resource_lifecycle_executes_complete_cycle -- --exact --nocapture",
                "commit_sha": current_commit_label(),
                "timestamp_unix_seconds": current_timestamp_unix_seconds(),
            }),
        );
        return;
    }

    let mut renderer = Renderer::headless_gpu(16, 16).unwrap_or_else(|error| {
        panic!("required lifecycle lane could not create GPU renderer: {error:?}")
    });
    let adapter = renderer
        .capability_report()
        .adapter()
        .cloned()
        .expect("required lifecycle GPU renderer reports live adapter provenance");
    require_hardware_adapter(&adapter).expect("required lifecycle lane uses physical hardware");

    let (mut scene, camera) = gpu_scene();
    disable_heavy_output(&mut renderer);
    renderer.prepare(&mut scene).expect("baseline prepares");
    let baseline = ResourceCounters::from(renderer.stats());

    enable_heavy_output(&mut renderer);
    renderer.prepare(&mut scene).expect("heavy output prepares");
    let prepared = ResourceCounters::from(renderer.stats());
    renderer
        .render_with_readback_mode(&scene, camera, RenderReadbackMode::Synchronous)
        .expect("heavy output renders before retirement");
    assert_eq!(
        ResourceCounters::from(renderer.stats()).retained_shape(),
        prepared.retained_shape()
    );

    disable_heavy_output(&mut renderer);
    renderer
        .prepare(&mut scene)
        .expect("baseline output reprepares after release");
    let released = ResourceCounters::from(renderer.stats());
    let poll = renderer.poll_device();

    let evidence = RequiredLifecycleEvidence {
        schema: "scena.q04.required_gpu_resource_lifecycle.v1".to_string(),
        status: "passed".to_string(),
        proof_class: "physical-hardware-required".to_string(),
        producer: "cargo test --test c09_gpu_resource_lifecycle".to_string(),
        command: format!(
            "{REQUIRED_LIFECYCLE_ENV}=1 cargo test --test c09_gpu_resource_lifecycle required_hardware_gpu_resource_lifecycle_executes_complete_cycle -- --exact --nocapture"
        ),
        commit_sha: current_commit_label(),
        timestamp_unix_seconds: current_timestamp_unix_seconds(),
        adapter: Some(adapter),
        baseline,
        prepared,
        released,
        poll_status: format!("{:?}", poll.status),
        poll_pending_before: poll.pending_destructions_before,
        poll_destroyed_resources: poll.destroyed_resources,
        poll_pending_after: poll.pending_destructions_after,
        assertions_executed: 12,
        complete_lifecycle: true,
    };
    validate_required_lifecycle_evidence(&evidence)
        .expect("required lifecycle evidence validates before publication");
    write_lifecycle_artifact("required-result.json", &evidence);
}
