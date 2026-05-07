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
#[allow(dead_code)]
pub(in crate::render) async fn request_browser_surface_gpu(
    backend: Backend,
    size: crate::platform::SurfaceSize,
    canvas: web_sys::HtmlCanvasElement,
) -> Result<GpuDeviceState, BuildError> {
    request_surface_gpu(backend, size, wgpu::SurfaceTarget::Canvas(canvas)).await
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
async fn request_surface_gpu(
    backend: Backend,
    size: crate::platform::SurfaceSize,
    target: wgpu::SurfaceTarget<'static>,
) -> Result<GpuDeviceState, BuildError> {
    let instance = wgpu::Instance::default();
    let surface = instance
        .create_surface(target)
        .map_err(|_| BuildError::CreateSurface { backend })?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..wgpu::RequestAdapterOptions::default()
        })
        .await
        .map_err(|_| BuildError::NoAdapter { backend })?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .map_err(|_| BuildError::RequestDevice { backend })?;
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
        #[cfg(not(target_arch = "wasm32"))]
        resources: None,
    })
}
