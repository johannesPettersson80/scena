use std::cell::{Cell, RefCell};
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use super::super::RasterTarget;
#[cfg(not(target_arch = "wasm32"))]
use super::pipeline::UnlitPipelines;
#[cfg(not(target_arch = "wasm32"))]
use super::{GpuPreparedResources, MsaaColorResources};

#[derive(Debug, Default)]
pub(super) struct SampleCountCapabilityCache {
    maximum_by_format: RefCell<HashMap<wgpu::TextureFormat, u32>>,
    probe_count: Cell<u64>,
}

impl SampleCountCapabilityCache {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn maximum_for<F>(&self, formats: &[wgpu::TextureFormat], probe: &mut F) -> u32
    where
        F: FnMut(wgpu::TextureFormat) -> u32,
    {
        formats
            .iter()
            .copied()
            .map(|format| {
                if let Some(maximum) = self.maximum_by_format.borrow().get(&format).copied() {
                    return maximum;
                }
                let maximum = probe(format);
                self.maximum_by_format.borrow_mut().insert(format, maximum);
                self.probe_count
                    .set(self.probe_count.get().saturating_add(1));
                maximum
            })
            .min()
            .unwrap_or(1)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn maximum_for_device(
        &self,
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        formats: &[wgpu::TextureFormat],
    ) -> u32 {
        self.maximum_for(formats, &mut |format| {
            max_supported_sample_count(device, adapter, &[format])
        })
    }

    pub(super) fn probe_count(&self) -> u64 {
        self.probe_count.get()
    }

    pub(super) fn clear(&self) {
        self.maximum_by_format.borrow_mut().clear();
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn offscreen_pipelines_for_sample_count(
    resources: &GpuPreparedResources,
    sample_count: u32,
) -> UnlitPipelines<'_> {
    match sample_count {
        4 => resources.offscreen_msaa4_pipelines.refs(),
        8 => resources
            .offscreen_msaa8_pipelines
            .as_ref()
            .expect("offscreen MSAA8 pipelines must be prepared before encoding")
            .refs(),
        _ => resources.offscreen_pipelines.refs(),
    }
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(super) fn max_supported_sample_count(
    device: &wgpu::Device,
    adapter: &wgpu::Adapter,
    formats: &[wgpu::TextureFormat],
) -> u32 {
    [8, 4, 1]
        .into_iter()
        .find(|sample_count| {
            formats.iter().all(|format| {
                texture_format_supports_sample_count(device, adapter, *format, *sample_count)
            })
        })
        .unwrap_or(1)
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(super) fn texture_format_supports_sample_count(
    device: &wgpu::Device,
    adapter: &wgpu::Adapter,
    format: wgpu::TextureFormat,
    sample_count: u32,
) -> bool {
    if sample_count > 4
        && !device
            .features()
            .contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES)
    {
        return false;
    }
    adapter
        .get_texture_format_features(format)
        .flags
        .sample_count_supported(sample_count)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn create_msaa_color_resources(
    device: &wgpu::Device,
    target: RasterTarget,
    format: wgpu::TextureFormat,
    sample_count: u32,
) -> MsaaColorResources {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scena.headless_gpu.msaa_color"),
        size: wgpu::Extent3d {
            width: target.width,
            height: target.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    MsaaColorResources {
        target,
        format,
        sample_count,
        texture,
        view,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn format_capability_cache_probes_each_format_only_once() {
        let cache = SampleCountCapabilityCache::default();
        let probes = Cell::new(0_u64);
        let formats = [
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::Depth32Float,
        ];
        let mut probe = |_format| {
            probes.set(probes.get() + 1);
            4
        };

        assert_eq!(cache.maximum_for(&formats, &mut probe), 4);
        assert_eq!(cache.maximum_for(&formats, &mut probe), 4);
        assert_eq!(
            probes.get(),
            2,
            "steady renders must perform zero new probes"
        );
        assert_eq!(cache.probe_count(), 2);
    }
}
