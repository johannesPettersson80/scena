use super::super::prepare::TiledLightAssignment;
use super::stats::GpuResourceStats;

#[derive(Debug)]
pub(super) struct LightAssignmentResources {
    pub(super) records: wgpu::Buffer,
    pub(super) tile_indices: wgpu::Buffer,
    pub(super) tiles: wgpu::Buffer,
    allocated_byte_len: u64,
}

pub(super) fn create_light_assignment_resources(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    assignment: &TiledLightAssignment,
) -> LightAssignmentResources {
    let record_bytes = assignment.record_bytes();
    let tile_index_bytes = assignment.tile_index_bytes();
    let tile_bytes = assignment.tile_bytes();
    let records =
        create_storage_buffer(device, queue, "scena.b2.tiled_light_records", &record_bytes);
    let tile_indices = create_storage_buffer(
        device,
        queue,
        "scena.b2.tiled_light_tile_indices",
        &tile_index_bytes,
    );
    let tiles = create_storage_buffer(device, queue, "scena.b2.tiled_light_tiles", &tile_bytes);
    LightAssignmentResources {
        records,
        tile_indices,
        tiles,
        allocated_byte_len: record_bytes.len().max(4) as u64
            + tile_index_bytes.len().max(4) as u64
            + tile_bytes.len().max(4) as u64,
    }
}

pub(super) fn resource_stats(resources: &LightAssignmentResources) -> GpuResourceStats {
    GpuResourceStats {
        buffers: 3,
        approximate_gpu_memory_bytes: resources.allocated_byte_len,
        ..GpuResourceStats::default()
    }
}

fn create_storage_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &'static str,
    bytes: &[u8],
) -> wgpu::Buffer {
    let size = bytes.len().max(4) as u64;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    if !bytes.is_empty() {
        queue.write_buffer(&buffer, 0, bytes);
    }
    buffer
}
