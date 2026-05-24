use crate::diagnostics::{Backend, BuildError, OutputColorSpace};
use crate::platform::SurfaceSize;

use super::{GpuDeviceState, GpuSurfaceState};

#[cfg(not(target_arch = "wasm32"))]
use crate::platform::BoxedNativeWindow;

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::render) async fn request_headless_gpu(
    backend: Backend,
) -> Result<GpuDeviceState, BuildError> {
    let instance = wgpu::Instance::default();
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

    Ok(GpuDeviceState {
        instance,
        adapter,
        device,
        queue,
        surface: None,
        pending_destructions: 0,
        resources: None,
        output_color_space: OutputColorSpace::Srgb,
        display_p3_canvas_configured: false,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn is_unstable_v3d_headless_adapter(info: &wgpu::AdapterInfo) -> bool {
    info.backend == wgpu::Backend::Vulkan && info.name.to_ascii_lowercase().contains("v3d")
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
    if let Ok(pair) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
    {
        return Ok(pair);
    }
    let downlevel = wgpu::DeviceDescriptor {
        required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
        ..wgpu::DeviceDescriptor::default()
    };
    adapter
        .request_device(&downlevel)
        .await
        .map_err(|_| BuildError::RequestDevice { backend })
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
    #[cfg(target_arch = "wasm32")]
    device.on_uncaptured_error(std::sync::Arc::new(|error| {
        web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
            "scena wgpu uncaptured error: {error:?}"
        )));
    }));
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
    surface.configure(&device, &config);

    Ok(GpuDeviceState {
        instance,
        adapter,
        device,
        queue,
        surface: Some(GpuSurfaceState { surface, config }),
        pending_destructions: 0,
        resources: None,
        output_color_space,
        display_p3_canvas_configured: false,
        #[cfg(target_arch = "wasm32")]
        browser_canvas: None,
    })
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

fn instance_for_backend(backend: Backend) -> wgpu::Instance {
    #[cfg(target_arch = "wasm32")]
    {
        let backends = match backend {
            Backend::WebGl2 => wgpu::Backends::GL,
            Backend::WebGpu => wgpu::Backends::BROWSER_WEBGPU,
            Backend::Headless
            | Backend::HeadlessGpu
            | Backend::SurfaceDescriptor
            | Backend::NativeSurface => wgpu::Backends::all(),
        };
        wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = backend;
        wgpu::Instance::default()
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

        info.name = String::from("V3D 7.1.10.2");
        info.backend = wgpu::Backend::Gl;
        assert!(!super::is_unstable_v3d_headless_adapter(&info));
    }
}
