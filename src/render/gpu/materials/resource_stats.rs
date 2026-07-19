use super::super::stats::GpuResourceStats;
use super::{MATERIAL_UNIFORM_ENTRY_STRIDE, MaterialResources};

fn material_texture_byte_len(resources: &MaterialResources) -> u64 {
    match resources {
        MaterialResources::PerMaterial(slots) => {
            slots.iter().map(|slot| slot.texture_byte_len).sum()
        }
        MaterialResources::Batched(batched) => batched.texture_byte_len,
    }
}

fn material_texture_count(resources: &MaterialResources) -> u64 {
    match resources {
        MaterialResources::PerMaterial(slots) => slots
            .iter()
            .map(|slot| slot.texture_bindings.len() as u64)
            .sum(),
        MaterialResources::Batched(batched) => batched.texture_bindings.len() as u64,
    }
}

pub(in crate::render::gpu) fn resource_stats(resources: &MaterialResources) -> GpuResourceStats {
    let (buffers, bind_groups, uniform_bytes, material_bind_groups) = match resources {
        MaterialResources::PerMaterial(slots) => {
            let count = slots.len() as u64;
            (
                count,
                count,
                count.saturating_mul(MATERIAL_UNIFORM_ENTRY_STRIDE),
                slots.len() as u32,
            )
        }
        MaterialResources::Batched(batched) => (
            1,
            1,
            MATERIAL_UNIFORM_ENTRY_STRIDE.saturating_mul(u64::from(batched.layer_count)),
            1,
        ),
    };
    GpuResourceStats {
        buffers,
        textures: material_texture_count(resources),
        bind_groups,
        approximate_gpu_memory_bytes: material_texture_byte_len(resources)
            .saturating_add(uniform_bytes),
        material_bind_groups,
        ..GpuResourceStats::default()
    }
}
