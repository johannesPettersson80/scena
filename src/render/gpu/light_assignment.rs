use super::super::prepare::TiledLightAssignment;

#[derive(Debug)]
pub(super) struct LightAssignmentResources {
    pub(super) records: wgpu::Buffer,
    pub(super) tile_indices: wgpu::Buffer,
    pub(super) tiles: wgpu::Buffer,
    pub(super) byte_len: u64,
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
        byte_len: (record_bytes.len() + tile_index_bytes.len() + tile_bytes.len()) as u64,
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
