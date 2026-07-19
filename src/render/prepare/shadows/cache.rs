use std::cell::RefCell;
use std::collections::BTreeMap;

use super::super::lighting::PreparedLights;
use super::{PrepareWorkCounter, ShadowOccluderSet, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ShadowVisibilityKey {
    world_position_bits: [u32; 3],
    light_state_signature: u64,
    occluder_state_signature: u64,
}

#[derive(Default)]
pub(in crate::render) struct ShadowVisibilityCache {
    light_state_signature: u64,
    occluder_state_signature: u64,
    directional: RefCell<BTreeMap<ShadowVisibilityKey, f32>>,
    area: RefCell<BTreeMap<ShadowVisibilityKey, f32>>,
}

impl ShadowVisibilityCache {
    pub(in crate::render) fn new(lights: &PreparedLights, occluders: &ShadowOccluderSet) -> Self {
        Self {
            light_state_signature: lights.shadow_state_signature(),
            occluder_state_signature: occluders.state_signature,
            directional: RefCell::new(BTreeMap::new()),
            area: RefCell::new(BTreeMap::new()),
        }
    }

    pub(in crate::render) fn directional(
        &self,
        position: Vec3,
        work: Option<&PrepareWorkCounter>,
        compute: impl FnOnce() -> f32,
    ) -> f32 {
        self.cached(&self.directional, position, work, compute)
    }

    pub(in crate::render) fn area(
        &self,
        position: Vec3,
        work: Option<&PrepareWorkCounter>,
        compute: impl FnOnce() -> f32,
    ) -> f32 {
        self.cached(&self.area, position, work, compute)
    }

    fn cached(
        &self,
        values: &RefCell<BTreeMap<ShadowVisibilityKey, f32>>,
        position: Vec3,
        work: Option<&PrepareWorkCounter>,
        compute: impl FnOnce() -> f32,
    ) -> f32 {
        let key = ShadowVisibilityKey {
            world_position_bits: [
                position.x.to_bits(),
                position.y.to_bits(),
                position.z.to_bits(),
            ],
            light_state_signature: self.light_state_signature,
            occluder_state_signature: self.occluder_state_signature,
        };
        if let Some(value) = values.borrow().get(&key).copied() {
            if let Some(work) = work {
                work.record_shadow_visibility_cache(true);
            }
            return value;
        }
        let value = compute();
        values.borrow_mut().insert(key, value);
        if let Some(work) = work {
            work.record_shadow_visibility_cache(false);
        }
        value
    }
}

pub(super) fn shadow_triangle_signature(triangles: &[[Vec3; 3]]) -> u64 {
    let mut signature = 0xcbf2_9ce4_8422_2325_u64;
    for vertex in triangles.iter().flatten() {
        for value in [vertex.x, vertex.y, vertex.z] {
            for byte in value.to_bits().to_le_bytes() {
                signature ^= u64::from(byte);
                signature = signature.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
    }
    signature
}
