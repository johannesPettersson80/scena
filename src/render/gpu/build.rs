use crate::diagnostics::{Backend, BuildError, OutputColorSpace};
use crate::platform::SurfaceSize;

use super::surface_frame::{GpuRuntimeFaultState, install_gpu_error_callback};
use super::{GpuDeviceState, GpuSurfaceState};

#[cfg(not(target_arch = "wasm32"))]
use crate::platform::BoxedNativeWindow;

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::render) async fn request_headless_gpu(
    backend: Backend,
) -> Result<GpuDeviceState, BuildError> {
    let instance = instance_for_backend(backend);
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .map_err(|_| BuildError::NoAdapter { backend })?;
    let adapter_info = adapter.get_info();
    if is_unstable_v3d_headless_adapter(&adapter_info)
        && std::env::var_os("SCENA_ALLOW_UNSTABLE_V3D_HEADLESS_GPU").is_none()
    {
        return Err(BuildError::RequestDevice { backend });
    }
    let (device, queue) = request_device_with_downlevel_fallback(&adapter, backend).await?;
    let runtime_fault = GpuRuntimeFaultState::default();
    install_gpu_error_callback(&device, runtime_fault.clone());
    let auto_exposure_meter = super::readback::GpuAutoExposureMeter::new(&device);

    Ok(GpuDeviceState {
        instance,
        adapter,
        device,
        queue,
        surface: None,
        runtime_fault,
        unstable_v3d_headless: v3d_headless_workarounds_required(&adapter_info),
        pending_destructions: 0,
        triangle_shader_modules: Default::default(),
        sample_count_capabilities: Default::default(),
        auto_exposure_meter,
        #[cfg(target_arch = "wasm32")]
        last_poll_observation: "not-polled",
        resources: None,
        output_color_space: OutputColorSpace::Srgb,
        display_p3_canvas_configured: false,
    })
}

/// Refuses the Raspberry Pi V3D adapter for headless GPU rendering.
///
/// The original reason recorded for this was that the adapter hangs. That is no
/// longer what happens, and it was scena's fault rather than the driver's: two
/// 256-entry LTC tables indexed at runtime from module-scope `const` arrays made
/// V3DV's register allocator fail all thirteen of its fallback strategies and
/// emit a 22,518-instruction fragment shader, which took 19 minutes to compile.
/// With the tables in a uniform block that is gone: 8,894 instructions, zero
/// allocation failures, and V3D rasterizes about six times faster than lavapipe.
///
/// Two V3DV defects remain in the explicit diagnostic lane. First, a completed
/// empty submission after full resource preparation is required before the
/// first graphics submission. Second, large unaligned headless target widths
/// silently lose beauty draws without a validation or runtime fault. A
/// near-identical aligned target renders on the first cycle, so the diagnostic
/// lane renders to an aspect-preserving 64-pixel-aligned internal target and
/// resolves back to the caller's exact requested dimensions.
///
/// The default refusal remains while these workarounds are hardware-only and
/// the adapter has not passed the complete release matrix. The
/// `SCENA_ALLOW_UNSTABLE_V3D_HEADLESS_GPU` escape hatch receives the barrier and
/// remains available for explicit hardware proof.
#[cfg(not(target_arch = "wasm32"))]
fn is_unstable_v3d_headless_adapter(info: &wgpu::AdapterInfo) -> bool {
    info.backend == wgpu::Backend::Vulkan && info.name.to_ascii_lowercase().contains("v3d")
}

#[cfg(not(target_arch = "wasm32"))]
fn v3d_headless_workarounds_required(info: &wgpu::AdapterInfo) -> bool {
    is_unstable_v3d_headless_adapter(info)
}

/// Try the WebGPU baseline first, fall back to `downlevel_defaults` if the
/// adapter rejects it. Embedded GPUs like the Pi 5's V3D and many tile-based
/// mobile GPUs cannot meet the desktop baseline (e.g. compute workgroup
/// invocations, storage buffer binding size) but do support every limit the
/// renderer actually consumes. Without this fallback, scena returns
/// `RequestDevice` on these hosts even though their drivers are functional.
async fn request_device_with_downlevel_fallback(
    adapter: &wgpu::Adapter,
    backend: Backend,
) -> Result<(wgpu::Device, wgpu::Queue), BuildError> {
    if let Ok(pair) = adapter.request_device(&device_descriptor(adapter)).await {
        return Ok(pair);
    }
    let downlevel = wgpu::DeviceDescriptor {
        required_features: adapter_features(adapter),
        required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
        ..wgpu::DeviceDescriptor::default()
    };
    adapter
        .request_device(&downlevel)
        .await
        .map_err(|_| BuildError::RequestDevice { backend })
}

fn device_descriptor(adapter: &wgpu::Adapter) -> wgpu::DeviceDescriptor<'static> {
    wgpu::DeviceDescriptor {
        required_features: adapter_features(adapter),
        ..wgpu::DeviceDescriptor::default()
    }
}

fn adapter_features(adapter: &wgpu::Adapter) -> wgpu::Features {
    let mut features = wgpu::Features::empty();
    if adapter
        .features()
        .contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES)
    {
        features |= wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
    }
    features
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::render) async fn request_native_surface_gpu(
    backend: Backend,
    size: SurfaceSize,
    window: BoxedNativeWindow,
) -> Result<GpuDeviceState, BuildError> {
    request_surface_gpu(backend, size, wgpu::SurfaceTarget::from(window)).await
}

#[cfg(target_arch = "wasm32")]
pub(in crate::render) async fn request_browser_surface_gpu(
    backend: Backend,
    size: crate::platform::SurfaceSize,
    canvas: web_sys::HtmlCanvasElement,
    output_color_space: OutputColorSpace,
) -> Result<GpuDeviceState, BuildError> {
    if backend == Backend::WebGl2 {
        prepare_webgl2_opaque_canvas_context(&canvas);
    }
    if backend == Backend::WebGpu && output_color_space == OutputColorSpace::DisplayP3 {
        super::browser_color_space::prepare_browser_canvas_output_color_space(
            backend,
            &canvas,
            output_color_space,
        );
    }
    let instance = instance_for_backend(backend);
    let surface = create_browser_canvas_surface(&instance, backend, &canvas)?;
    let mut state =
        request_gpu_for_surface(backend, size, instance, surface, output_color_space).await?;
    let effective_size = state.surface_size().unwrap_or(size);
    if effective_size != size {
        canvas.set_width(effective_size.width);
        canvas.set_height(effective_size.height);
    }
    state.browser_canvas = Some(canvas);
    state.refresh_browser_canvas_output_color_space(backend);
    Ok(state)
}

#[cfg(target_arch = "wasm32")]
impl GpuDeviceState {
    pub(in crate::render) fn attach_browser_surface(
        &mut self,
        backend: Backend,
        size: crate::platform::SurfaceSize,
        canvas: web_sys::HtmlCanvasElement,
    ) -> Result<crate::platform::SurfaceSize, BuildError> {
        if backend == Backend::WebGl2 {
            prepare_webgl2_opaque_canvas_context(&canvas);
        }
        if backend == Backend::WebGpu && self.output_color_space == OutputColorSpace::DisplayP3 {
            super::browser_color_space::prepare_browser_canvas_output_color_space(
                backend,
                &canvas,
                self.output_color_space,
            );
        }
        let surface = create_browser_canvas_surface(&self.instance, backend, &canvas)?;
        let effective_size = clamp_surface_size_to_adapter_limits(
            size,
            self.device.limits().max_texture_dimension_2d,
        );
        let mut config = surface
            .get_default_config(&self.adapter, effective_size.width, effective_size.height)
            .ok_or(BuildError::SurfaceUnsupported { backend })?;
        let capabilities = surface.get_capabilities(&self.adapter);
        if capabilities
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::Opaque)
        {
            config.alpha_mode = wgpu::CompositeAlphaMode::Opaque;
        }
        enable_scene_host_surface_readback(&mut config, &capabilities);
        surface.configure(&self.device, &config);
        if effective_size != size {
            canvas.set_width(effective_size.width);
            canvas.set_height(effective_size.height);
        }
        self.surface = Some(GpuSurfaceState { surface, config });
        self.browser_canvas = Some(canvas);
        self.refresh_browser_canvas_output_color_space(backend);
        Ok(effective_size)
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn request_surface_gpu(
    backend: Backend,
    size: crate::platform::SurfaceSize,
    target: wgpu::SurfaceTarget<'static>,
) -> Result<GpuDeviceState, BuildError> {
    let instance = instance_for_backend(backend);
    let surface = instance
        .create_surface(target)
        .map_err(|_| BuildError::CreateSurface { backend })?;
    request_gpu_for_surface(backend, size, instance, surface, OutputColorSpace::Srgb).await
}

async fn request_gpu_for_surface(
    backend: Backend,
    size: crate::platform::SurfaceSize,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    output_color_space: OutputColorSpace,
) -> Result<GpuDeviceState, BuildError> {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..wgpu::RequestAdapterOptions::default()
        })
        .await
        .map_err(|_| BuildError::NoAdapter { backend })?;
    let (device, queue) = if backend == Backend::WebGl2 {
        let descriptor = wgpu::DeviceDescriptor {
            required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            ..wgpu::DeviceDescriptor::default()
        };
        adapter
            .request_device(&descriptor)
            .await
            .map_err(|_| BuildError::RequestDevice { backend })?
    } else {
        request_device_with_downlevel_fallback(&adapter, backend).await?
    };
    let runtime_fault = GpuRuntimeFaultState::default();
    install_gpu_error_callback(&device, runtime_fault.clone());
    #[cfg(not(target_arch = "wasm32"))]
    let auto_exposure_meter = super::readback::GpuAutoExposureMeter::new(&device);
    #[cfg(target_arch = "wasm32")]
    let browser_auto_exposure_meter = super::browser_meter::BrowserAutoExposureMeter::new(&device);
    let effective_size =
        clamp_surface_size_to_adapter_limits(size, device.limits().max_texture_dimension_2d);
    let mut config = surface
        .get_default_config(&adapter, effective_size.width, effective_size.height)
        .ok_or(BuildError::SurfaceUnsupported { backend })?;
    let capabilities = surface.get_capabilities(&adapter);
    if capabilities
        .alpha_modes
        .contains(&wgpu::CompositeAlphaMode::Opaque)
    {
        config.alpha_mode = wgpu::CompositeAlphaMode::Opaque;
    }
    enable_scene_host_surface_readback(&mut config, &capabilities);
    surface.configure(&device, &config);

    Ok(GpuDeviceState {
        instance,
        adapter,
        device,
        queue,
        surface: Some(GpuSurfaceState { surface, config }),
        runtime_fault,
        #[cfg(not(target_arch = "wasm32"))]
        unstable_v3d_headless: false,
        pending_destructions: 0,
        triangle_shader_modules: Default::default(),
        sample_count_capabilities: Default::default(),
        #[cfg(not(target_arch = "wasm32"))]
        auto_exposure_meter,
        #[cfg(target_arch = "wasm32")]
        browser_auto_exposure_meter,
        #[cfg(target_arch = "wasm32")]
        last_poll_observation: "not-polled",
        resources: None,
        output_color_space,
        display_p3_canvas_configured: false,
        #[cfg(target_arch = "wasm32")]
        browser_canvas: None,
    })
}

pub(super) fn enable_scene_host_surface_readback(
    config: &mut wgpu::SurfaceConfiguration,
    capabilities: &wgpu::SurfaceCapabilities,
) {
    #[cfg(any(
        not(target_arch = "wasm32"),
        all(
            target_arch = "wasm32",
            feature = "scene-host",
            not(feature = "browser-probe")
        )
    ))]
    if capabilities.usages.contains(wgpu::TextureUsages::COPY_SRC)
        && matches!(
            config.format,
            wgpu::TextureFormat::Rgba8Unorm
                | wgpu::TextureFormat::Rgba8UnormSrgb
                | wgpu::TextureFormat::Bgra8Unorm
                | wgpu::TextureFormat::Bgra8UnormSrgb
        )
    {
        config.usage |= wgpu::TextureUsages::COPY_SRC;
    }
    let _ = (config, capabilities);
}

pub(super) fn clamp_surface_size_to_adapter_limits(
    size: SurfaceSize,
    max_texture_dimension_2d: u32,
) -> SurfaceSize {
    if max_texture_dimension_2d == 0
        || (size.width <= max_texture_dimension_2d && size.height <= max_texture_dimension_2d)
    {
        return size;
    }

    let scale = max_texture_dimension_2d as f64 / size.width.max(size.height) as f64;
    SurfaceSize {
        width: ((size.width as f64 * scale).floor() as u32)
            .max(1)
            .min(max_texture_dimension_2d),
        height: ((size.height as f64 * scale).floor() as u32)
            .max(1)
            .min(max_texture_dimension_2d),
    }
}

#[cfg(target_arch = "wasm32")]
fn create_browser_canvas_surface(
    instance: &wgpu::Instance,
    backend: Backend,
    canvas: &web_sys::HtmlCanvasElement,
) -> Result<wgpu::Surface<'static>, BuildError> {
    if backend == Backend::WebGpu {
        return instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|_| BuildError::CreateSurface { backend });
    }

    use std::ptr::NonNull;

    let value: &wasm_bindgen::JsValue = canvas;
    let raw_window_handle =
        raw_window_handle::WebCanvasWindowHandle::new(NonNull::from(value).cast()).into();
    let raw_display_handle = raw_window_handle::WebDisplayHandle::new().into();
    // SAFETY: wgpu 29's safe `SurfaceTarget::Canvas` omits WebDisplayHandle,
    // which the WebGL2 backend still needs. The raw handles are produced from
    // the live HtmlCanvasElement, and wgpu copies the canvas reference during
    // surface creation.
    unsafe {
        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: Some(raw_display_handle),
            raw_window_handle,
        })
    }
    .map_err(|_| BuildError::CreateSurface { backend })
}

#[cfg(target_arch = "wasm32")]
fn prepare_webgl2_opaque_canvas_context(canvas: &web_sys::HtmlCanvasElement) {
    let attributes = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &attributes,
        &wasm_bindgen::JsValue::from_str("alpha"),
        &wasm_bindgen::JsValue::FALSE,
    );
    let _ = js_sys::Reflect::set(
        &attributes,
        &wasm_bindgen::JsValue::from_str("premultipliedAlpha"),
        &wasm_bindgen::JsValue::FALSE,
    );
    let _ = js_sys::Reflect::set(
        &attributes,
        &wasm_bindgen::JsValue::from_str("preserveDrawingBuffer"),
        &wasm_bindgen::JsValue::TRUE,
    );
    let _ = canvas.get_context_with_context_options("webgl2", attributes.as_ref());
}

#[cfg(any(target_arch = "wasm32", test))]
fn browser_instance_descriptor(backend: Backend) -> wgpu::InstanceDescriptor {
    let backends = match backend {
        Backend::WebGl2 => wgpu::Backends::GL,
        Backend::WebGpu => wgpu::Backends::BROWSER_WEBGPU,
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::SurfaceDescriptor
        | Backend::NativeSurface => wgpu::Backends::all(),
    };
    let mut descriptor = wgpu::InstanceDescriptor {
        backends,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    };
    if backend == Backend::WebGl2 {
        // Browser GL fence sync objects can remain unsignalled indefinitely
        // under software ANGLE/SwiftShader. wgpu explicitly provides this
        // policy for WebGL: GL owns the real in-flight lifetime after objects
        // are deleted, while wgpu may retire its logical submission records.
        descriptor.backend_options.gl.fence_behavior = wgpu::GlFenceBehavior::AutoFinish;
    }
    descriptor
}

fn instance_for_backend(backend: Backend) -> wgpu::Instance {
    #[cfg(target_arch = "wasm32")]
    {
        wgpu::Instance::new(browser_instance_descriptor(backend))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = backend;
        let backends = wgpu::Backends::all().with_env();
        wgpu::Instance::new(native_instance_descriptor(backends))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn native_instance_descriptor(backends: wgpu::Backends) -> wgpu::InstanceDescriptor {
    wgpu::InstanceDescriptor {
        backends,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use crate::platform::SurfaceSize;

    #[test]
    fn browser_surface_config_prefers_opaque_alpha() {
        let source = include_str!("build.rs");
        assert!(
            source.contains("CompositeAlphaMode::Opaque")
                && source.contains("config.alpha_mode = wgpu::CompositeAlphaMode::Opaque")
                && source.contains("prepare_webgl2_opaque_canvas_context")
                && source.contains("\"alpha\"")
                && source.contains("JsValue::FALSE"),
            "browser material proof must configure an opaque surface when supported; otherwise \
             the WebGL canvas clears to alpha 0 and screenshots composite over page/chrome backgrounds"
        );
    }

    #[test]
    fn webgl2_uses_automatic_fence_retirement_without_weakening_webgpu() {
        let webgl2 = super::browser_instance_descriptor(crate::Backend::WebGl2);
        assert_eq!(
            webgl2.backend_options.gl.fence_behavior,
            wgpu::GlFenceBehavior::AutoFinish,
            "WebGL2 cannot depend on a browser GL fence that may never signal"
        );

        let webgpu = super::browser_instance_descriptor(crate::Backend::WebGpu);
        assert_eq!(
            webgpu.backend_options.gl.fence_behavior,
            wgpu::GlFenceBehavior::Normal,
            "the WebGPU backend must retain real queue-completion semantics"
        );
    }

    #[test]
    fn native_instance_honors_wgpu_backend_filter() {
        let descriptor = super::native_instance_descriptor(wgpu::Backends::DX12);
        assert_eq!(
            descriptor.backends,
            wgpu::Backends::DX12,
            "native GPU construction must preserve the WGPU_BACKEND filter before adapter \
             selection so backend-specific release lanes cannot silently run on another native API"
        );
    }

    #[test]
    fn headless_gpu_uses_the_filtered_native_instance_path() {
        let source = include_str!("build.rs");
        let headless_constructor = source
            .split("pub(in crate::render) async fn request_headless_gpu")
            .nth(1)
            .and_then(|tail| tail.split("fn is_unstable_v3d_headless_adapter").next())
            .expect("headless GPU constructor source is present");
        assert!(
            headless_constructor.contains("let instance = instance_for_backend(backend);")
                && !headless_constructor.contains("wgpu::Instance::default()"),
            "headless GPU construction must use the same WGPU_BACKEND-filtered native instance \
             path as attached surfaces"
        );
    }

    #[test]
    fn oversized_surface_size_is_clamped_to_adapter_limit_preserving_aspect() {
        assert_eq!(
            super::clamp_surface_size_to_adapter_limits(
                SurfaceSize {
                    width: 2560,
                    height: 1191,
                },
                2048,
            ),
            SurfaceSize {
                width: 2048,
                height: 952,
            },
        );
        assert_eq!(
            super::clamp_surface_size_to_adapter_limits(
                SurfaceSize {
                    width: 1440,
                    height: 900,
                },
                2048,
            ),
            SurfaceSize {
                width: 1440,
                height: 900,
            },
        );
    }

    #[test]
    fn v3d_vulkan_headless_adapter_is_rejected_by_default() {
        let info = wgpu::AdapterInfo {
            name: String::from("V3D 7.1.10.2"),
            vendor: 0,
            device: 0,
            device_type: wgpu::DeviceType::IntegratedGpu,
            device_pci_bus_id: String::new(),
            driver: String::from("V3DV"),
            driver_info: String::new(),
            backend: wgpu::Backend::Vulkan,
            subgroup_min_size: 8,
            subgroup_max_size: 8,
            transient_saves_memory: false,
        };

        assert!(super::is_unstable_v3d_headless_adapter(&info));
        assert!(super::v3d_headless_workarounds_required(&info));
    }

    #[test]
    fn non_vulkan_or_non_v3d_headless_adapter_is_not_rejected() {
        let mut info = wgpu::AdapterInfo {
            name: String::from("llvmpipe"),
            vendor: 0,
            device: 0,
            device_type: wgpu::DeviceType::Cpu,
            device_pci_bus_id: String::new(),
            driver: String::from("lavapipe"),
            driver_info: String::new(),
            backend: wgpu::Backend::Vulkan,
            subgroup_min_size: 8,
            subgroup_max_size: 8,
            transient_saves_memory: false,
        };

        assert!(!super::is_unstable_v3d_headless_adapter(&info));
        assert!(!super::v3d_headless_workarounds_required(&info));

        info.name = String::from("V3D 7.1.10.2");
        info.backend = wgpu::Backend::Gl;
        assert!(!super::is_unstable_v3d_headless_adapter(&info));
        assert!(!super::v3d_headless_workarounds_required(&info));
    }

    #[test]
    fn device_request_includes_adapter_specific_format_feature_when_available() {
        let source = include_str!("build.rs");
        assert!(
            source.contains("TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES")
                && source.contains("required_features: adapter_features(adapter)"),
            "native GPU device requests must opt into adapter-specific texture format features so \
             supported MSAA8 pipelines can be prepared instead of failing after adapter probing"
        );
    }
}
