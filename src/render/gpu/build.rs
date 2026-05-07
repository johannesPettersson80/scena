use crate::diagnostics::{Backend, BuildError};

use super::{GpuDeviceState, GpuSurfaceState};

#[cfg(not(target_arch = "wasm32"))]
use crate::platform::{BoxedNativeWindow, SurfaceSize};

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::render) async fn request_headless_gpu(
    backend: Backend,
) -> Result<GpuDeviceState, BuildError> {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .map_err(|_| BuildError::NoAdapter { backend })?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .map_err(|_| BuildError::RequestDevice { backend })?;

    Ok(GpuDeviceState {
        instance,
        adapter,
        device,
        queue,
        surface: None,
        pending_destructions: 0,
        resources: None,
    })
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
) -> Result<GpuDeviceState, BuildError> {
    let instance = instance_for_backend(backend);
    let surface = create_browser_canvas_surface(&instance, backend, &canvas)?;
    let mut state = request_gpu_for_surface(backend, size, instance, surface).await?;
    state.browser_canvas = Some(canvas);
    Ok(state)
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
    request_gpu_for_surface(backend, size, instance, surface).await
}

async fn request_gpu_for_surface(
    backend: Backend,
    size: crate::platform::SurfaceSize,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
) -> Result<GpuDeviceState, BuildError> {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..wgpu::RequestAdapterOptions::default()
        })
        .await
        .map_err(|_| BuildError::NoAdapter { backend })?;
    let mut descriptor = wgpu::DeviceDescriptor::default();
    if backend == Backend::WebGl2 {
        descriptor.required_limits = wgpu::Limits::downlevel_webgl2_defaults();
    }
    let (device, queue) = adapter
        .request_device(&descriptor)
        .await
        .map_err(|_| BuildError::RequestDevice { backend })?;
    #[cfg(target_arch = "wasm32")]
    device.on_uncaptured_error(std::sync::Arc::new(|error| {
        web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
            "scena wgpu uncaptured error: {error:?}"
        )));
    }));
    let config = surface
        .get_default_config(&adapter, size.width, size.height)
        .ok_or(BuildError::SurfaceUnsupported { backend })?;
    surface.configure(&device, &config);

    Ok(GpuDeviceState {
        instance,
        adapter,
        device,
        queue,
        surface: Some(GpuSurfaceState { surface, config }),
        pending_destructions: 0,
        resources: None,
        #[cfg(target_arch = "wasm32")]
        browser_canvas: None,
    })
}

#[cfg(target_arch = "wasm32")]
fn create_browser_canvas_surface(
    instance: &wgpu::Instance,
    backend: Backend,
    canvas: &web_sys::HtmlCanvasElement,
) -> Result<wgpu::Surface<'static>, BuildError> {
    use std::ptr::NonNull;

    let value: &wasm_bindgen::JsValue = canvas;
    let raw_window_handle =
        raw_window_handle::WebCanvasWindowHandle::new(NonNull::from(value).cast()).into();
    let raw_display_handle = raw_window_handle::WebDisplayHandle::new().into();
    // SAFETY: the handles are produced from the live `HtmlCanvasElement` passed by the host.
    // wgpu copies the canvas reference during surface creation, and this function does not
    // retain borrowed raw handles beyond the call.
    unsafe {
        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: Some(raw_display_handle),
            raw_window_handle,
        })
    }
    .map_err(|_| BuildError::CreateSurface { backend })
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
