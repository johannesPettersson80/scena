use std::collections::BTreeSet;
use std::fmt;

use crate::assets::{EnvironmentHandle, MaterialHandle};
use crate::geometry::Aabb;

use super::{NodeKey, ReflectionProbeKey, Scene, Vec3};

pub const MAX_REFLECTION_PROBES: usize = 4;
pub const DEFAULT_REFLECTION_PROBE_RESOLUTION: u32 = 256;

#[derive(Debug, Clone, PartialEq)]
pub struct ReflectionProbe {
    bounds: Aabb,
    capture_position: Vec3,
    resolution: u32,
    environment: Option<EnvironmentHandle>,
    assigned_nodes: BTreeSet<NodeKey>,
    assigned_materials: BTreeSet<MaterialHandle>,
}

impl ReflectionProbe {
    pub fn new(bounds: Aabb) -> Self {
        Self {
            capture_position: bounds.center(),
            bounds,
            resolution: DEFAULT_REFLECTION_PROBE_RESOLUTION,
            environment: None,
            assigned_nodes: BTreeSet::new(),
            assigned_materials: BTreeSet::new(),
        }
    }

    pub fn with_capture_position(mut self, position: Vec3) -> Self {
        self.capture_position = position;
        self
    }

    pub const fn with_resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }

    pub const fn with_environment(mut self, environment: EnvironmentHandle) -> Self {
        self.environment = Some(environment);
        self
    }

    pub fn assign_node(mut self, node: NodeKey) -> Self {
        self.assigned_nodes.insert(node);
        self
    }

    pub fn assign_material(mut self, material: MaterialHandle) -> Self {
        self.assigned_materials.insert(material);
        self
    }

    pub const fn bounds(&self) -> Aabb {
        self.bounds
    }

    pub const fn capture_position(&self) -> Vec3 {
        self.capture_position
    }

    pub const fn resolution(&self) -> u32 {
        self.resolution
    }

    pub const fn environment(&self) -> Option<EnvironmentHandle> {
        self.environment
    }

    pub fn assigned_nodes(&self) -> impl Iterator<Item = NodeKey> + '_ {
        self.assigned_nodes.iter().copied()
    }

    pub fn assigned_materials(&self) -> impl Iterator<Item = MaterialHandle> + '_ {
        self.assigned_materials.iter().copied()
    }

    pub(crate) fn set_environment(&mut self, environment: EnvironmentHandle) {
        self.environment = Some(environment);
    }

    fn matches(&self, scene: &Scene, node: NodeKey, material: MaterialHandle) -> bool {
        let node_matches = self.assigned_nodes.is_empty()
            || self
                .assigned_nodes
                .iter()
                .any(|assigned| node_is_descendant_of(scene, node, *assigned));
        let material_matches =
            self.assigned_materials.is_empty() || self.assigned_materials.contains(&material);
        node_matches && material_matches
    }

    pub(in crate::scene) fn remove_nodes(&mut self, removed: &BTreeSet<NodeKey>) {
        self.assigned_nodes.retain(|node| !removed.contains(node));
    }

    pub(in crate::scene) fn has_assignment(&self) -> bool {
        !self.assigned_nodes.is_empty() || !self.assigned_materials.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReflectionProbeError {
    InvalidBounds,
    InvalidCapturePosition,
    InvalidResolution { resolution: u32, maximum: u32 },
    MissingAssignment,
    NodeNotFound(NodeKey),
    CapacityExceeded { maximum: usize },
    ProbeNotFound(ReflectionProbeKey),
}

impl fmt::Display for ReflectionProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBounds => write!(
                formatter,
                "reflection probe bounds must be finite with positive extent on every axis"
            ),
            Self::InvalidCapturePosition => {
                write!(
                    formatter,
                    "reflection probe capture position must be finite"
                )
            }
            Self::InvalidResolution {
                resolution,
                maximum,
            } => write!(
                formatter,
                "reflection probe resolution {resolution} must be a power of two between 16 and {maximum}"
            ),
            Self::MissingAssignment => write!(
                formatter,
                "reflection probe requires at least one component node or material assignment"
            ),
            Self::NodeNotFound(node) => {
                write!(
                    formatter,
                    "reflection probe assignment node not found: {node:?}"
                )
            }
            Self::CapacityExceeded { maximum } => {
                write!(
                    formatter,
                    "a scene supports at most {maximum} reflection probes"
                )
            }
            Self::ProbeNotFound(probe) => {
                write!(formatter, "reflection probe not found: {probe:?}")
            }
        }
    }
}

impl std::error::Error for ReflectionProbeError {}

impl Scene {
    pub fn add_reflection_probe(
        &mut self,
        probe: ReflectionProbe,
    ) -> Result<ReflectionProbeKey, ReflectionProbeError> {
        validate_probe(self, &probe)?;
        if self.reflection_probes.len() >= MAX_REFLECTION_PROBES {
            return Err(ReflectionProbeError::CapacityExceeded {
                maximum: MAX_REFLECTION_PROBES,
            });
        }
        let key = self.reflection_probes.insert(probe);
        self.appearance_revision = self.appearance_revision.saturating_add(1);
        Ok(key)
    }

    pub fn reflection_probe(&self, probe: ReflectionProbeKey) -> Option<&ReflectionProbe> {
        self.reflection_probes.get(probe)
    }

    pub fn reflection_probes(
        &self,
    ) -> impl Iterator<Item = (ReflectionProbeKey, &ReflectionProbe)> {
        self.reflection_probes.iter()
    }

    pub fn select_reflection_probe(
        &self,
        node: NodeKey,
        material: MaterialHandle,
        world_position: Vec3,
    ) -> Option<(ReflectionProbeKey, &ReflectionProbe)> {
        if !self.reflection_probes_enabled {
            return None;
        }
        self.reflection_probes
            .iter()
            .filter(|(_, probe)| {
                probe.bounds.contains(world_position) && probe.matches(self, node, material)
            })
            .min_by(|(_, left), (_, right)| {
                probe_volume(left)
                    .total_cmp(&probe_volume(right))
                    .then_with(|| {
                        probe_center_distance_squared(left, world_position)
                            .total_cmp(&probe_center_distance_squared(right, world_position))
                    })
            })
    }

    pub fn set_reflection_probe_environment(
        &mut self,
        probe: ReflectionProbeKey,
        environment: EnvironmentHandle,
    ) -> Result<(), ReflectionProbeError> {
        let probe = self
            .reflection_probes
            .get_mut(probe)
            .ok_or(ReflectionProbeError::ProbeNotFound(probe))?;
        if probe.environment != Some(environment) {
            probe.set_environment(environment);
            self.appearance_revision = self.appearance_revision.saturating_add(1);
        }
        Ok(())
    }

    pub fn remove_reflection_probe(
        &mut self,
        probe: ReflectionProbeKey,
    ) -> Result<ReflectionProbe, ReflectionProbeError> {
        let removed = self
            .reflection_probes
            .remove(probe)
            .ok_or(ReflectionProbeError::ProbeNotFound(probe))?;
        self.appearance_revision = self.appearance_revision.saturating_add(1);
        Ok(removed)
    }

    pub(crate) const fn reflection_probes_enabled(&self) -> bool {
        self.reflection_probes_enabled
    }

    #[cfg(feature = "scene-host")]
    pub(crate) fn set_reflection_probes_enabled(&mut self, enabled: bool) {
        if self.reflection_probes_enabled != enabled {
            self.reflection_probes_enabled = enabled;
            self.appearance_revision = self.appearance_revision.saturating_add(1);
        }
    }
}

fn validate_probe(scene: &Scene, probe: &ReflectionProbe) -> Result<(), ReflectionProbeError> {
    let extent = probe.bounds.max - probe.bounds.min;
    if !probe.bounds.min.is_finite()
        || !probe.bounds.max.is_finite()
        || extent.x <= 0.0
        || extent.y <= 0.0
        || extent.z <= 0.0
    {
        return Err(ReflectionProbeError::InvalidBounds);
    }
    if !probe.capture_position.is_finite() {
        return Err(ReflectionProbeError::InvalidCapturePosition);
    }
    if probe.resolution < 16
        || probe.resolution > DEFAULT_REFLECTION_PROBE_RESOLUTION
        || !probe.resolution.is_power_of_two()
    {
        return Err(ReflectionProbeError::InvalidResolution {
            resolution: probe.resolution,
            maximum: DEFAULT_REFLECTION_PROBE_RESOLUTION,
        });
    }
    if !probe.has_assignment() {
        return Err(ReflectionProbeError::MissingAssignment);
    }
    for node in &probe.assigned_nodes {
        if !scene.nodes.contains_key(*node) {
            return Err(ReflectionProbeError::NodeNotFound(*node));
        }
    }
    Ok(())
}

fn probe_volume(probe: &ReflectionProbe) -> f32 {
    let extent = probe.bounds.max - probe.bounds.min;
    extent.x * extent.y * extent.z
}

fn probe_center_distance_squared(probe: &ReflectionProbe, point: Vec3) -> f32 {
    (probe.bounds.center() - point).length_squared()
}

fn node_is_descendant_of(scene: &Scene, candidate: NodeKey, ancestor: NodeKey) -> bool {
    let mut current = Some(candidate);
    while let Some(node) = current {
        if node == ancestor {
            return true;
        }
        current = scene.nodes.get(node).and_then(|node| node.parent());
    }
    false
}
