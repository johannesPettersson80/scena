use std::cell::RefCell;
use std::collections::BTreeMap;

use super::super::lighting::PreparedLights;
use super::{PrepareWorkCounter, ShadowOccluderSet, Vec3};
use crate::BakedAmbientOcclusionConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ShadowVisibilityKey {
    world_position_bits: [u32; 3],
    light_state_signature: u64,
    occluder_state_signature: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct AmbientVisibilityKey {
    world_position_bits: [u32; 3],
    world_normal_bits: [u32; 3],
    sample_count: u8,
    radius_fraction_bits: u32,
    intensity_bits: u32,
    occluder_state_signature: u64,
}

#[derive(Default, Debug)]
pub(in crate::render) struct ShadowVisibilityCache {
    light_state_signature: u64,
    occluder_state_signature: u64,
    directional: RefCell<BTreeMap<ShadowVisibilityKey, f32>>,
    area: RefCell<BTreeMap<ShadowVisibilityKey, f32>>,
    ambient: RefCell<BTreeMap<AmbientVisibilityKey, f32>>,
}

impl ShadowVisibilityCache {
    pub(in crate::render::prepare) fn new(
        lights: &PreparedLights,
        occluders: &ShadowOccluderSet,
    ) -> Self {
        Self {
            light_state_signature: lights.shadow_state_signature(),
            occluder_state_signature: occluders.state_signature,
            directional: RefCell::new(BTreeMap::new()),
            area: RefCell::new(BTreeMap::new()),
            ambient: RefCell::new(BTreeMap::new()),
        }
    }

    /// True when this cache was built for the same lighting and occluder state,
    /// so its entries still describe the current scene.
    ///
    /// Entries are keyed by both signatures as well, so a mismatched cache can
    /// never return a stale value; this check exists to avoid retaining a map
    /// that can no longer produce a hit.
    pub(in crate::render::prepare) fn matches(
        &self,
        lights: &PreparedLights,
        occluders: &ShadowOccluderSet,
    ) -> bool {
        self.light_state_signature == lights.shadow_state_signature()
            && self.occluder_state_signature == occluders.state_signature
    }

    pub(in crate::render::prepare) fn directional(
        &self,
        position: Vec3,
        work: Option<&PrepareWorkCounter>,
        compute: impl FnOnce() -> f32,
    ) -> f32 {
        self.cached(&self.directional, position, work, compute)
    }

    pub(in crate::render::prepare) fn area(
        &self,
        position: Vec3,
        work: Option<&PrepareWorkCounter>,
        compute: impl FnOnce() -> f32,
    ) -> f32 {
        self.cached(&self.area, position, work, compute)
    }

    pub(in crate::render::prepare) fn ambient(
        &self,
        position: Vec3,
        normal: Vec3,
        config: BakedAmbientOcclusionConfig,
        work: Option<&PrepareWorkCounter>,
        compute: impl FnOnce() -> f32,
    ) -> f32 {
        let key = AmbientVisibilityKey {
            world_position_bits: [
                position.x.to_bits(),
                position.y.to_bits(),
                position.z.to_bits(),
            ],
            world_normal_bits: [normal.x.to_bits(), normal.y.to_bits(), normal.z.to_bits()],
            sample_count: config.sample_count(),
            radius_fraction_bits: config.radius_fraction().to_bits(),
            intensity_bits: config.intensity().to_bits(),
            occluder_state_signature: self.occluder_state_signature,
        };
        if let Some(value) = self.ambient.borrow().get(&key).copied() {
            if let Some(work) = work {
                work.record_shadow_visibility_cache(true);
            }
            return value;
        }
        let value = compute();
        self.ambient.borrow_mut().insert(key, value);
        if let Some(work) = work {
            work.record_shadow_visibility_cache(false);
        }
        value
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
