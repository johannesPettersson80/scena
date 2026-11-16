//! Instanced draws: one retained triangle template drawn once per record.

use crate::material::Color;
use crate::scene::{InstanceId, InstanceSetKey, NodeKey};

use super::PreparedPrimitive;

#[derive(Debug, Clone, PartialEq)]
pub(in crate::render) struct PreparedInstanceSet {
    source_node: NodeKey,
    source_set: Option<InstanceSetKey>,
    /// The mesh nodes an automatically batched group draws, one per record.
    /// Empty for a scene instance set, whose records come from `source_set`.
    member_nodes: Vec<NodeKey>,
    primitives: Vec<PreparedPrimitive>,
    instances: Vec<PreparedInstanceRecord>,
}

impl PreparedInstanceSet {
    /// Fewest mesh nodes sharing geometry and material that are drawn as one
    /// automatically batched group. Fewer are baked one by one.
    pub(in crate::render) const MIN_AUTO_BATCHED_MEMBERS: usize = 4;

    pub(in crate::render) fn new(
        source_node: NodeKey,
        source_set: InstanceSetKey,
        primitives: Vec<PreparedPrimitive>,
        instances: Vec<PreparedInstanceRecord>,
    ) -> Self {
        Self {
            source_node,
            source_set: Some(source_set),
            member_nodes: Vec::new(),
            primitives,
            instances,
        }
    }

    /// A group of mesh nodes that share geometry and material, drawn as one
    /// instanced batch. `instances` holds one record per entry of
    /// `member_nodes`, in the same order.
    pub(in crate::render) fn new_auto_batched(
        member_nodes: Vec<NodeKey>,
        primitives: Vec<PreparedPrimitive>,
        instances: Vec<PreparedInstanceRecord>,
    ) -> Option<Self> {
        Some(Self {
            source_node: *member_nodes.first()?,
            source_set: None,
            member_nodes,
            primitives,
            instances,
        })
    }

    pub(in crate::render) fn member_nodes(&self) -> &[NodeKey] {
        &self.member_nodes
    }

    pub(in crate::render) const fn source_node(&self) -> NodeKey {
        self.source_node
    }

    pub(in crate::render) const fn source_set(&self) -> Option<InstanceSetKey> {
        self.source_set
    }

    pub(in crate::render) fn primitives(&self) -> &[PreparedPrimitive] {
        &self.primitives
    }

    pub(in crate::render) fn primitives_mut(&mut self) -> &mut [PreparedPrimitive] {
        &mut self.primitives
    }

    pub(in crate::render) fn instances(&self) -> &[PreparedInstanceRecord] {
        &self.instances
    }

    pub(in crate::render) fn set_instances(&mut self, instances: Vec<PreparedInstanceRecord>) {
        self.instances = instances;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::render) struct PreparedInstanceRecord {
    source_instance: Option<InstanceId>,
    world_from_model: [f32; 16],
    normal_from_model: [f32; 16],
    tint: Color,
}

impl PreparedInstanceRecord {
    pub(in crate::render) const fn new(
        source_instance: InstanceId,
        world_from_model: [f32; 16],
        normal_from_model: [f32; 16],
        tint: Color,
    ) -> Self {
        Self {
            source_instance: Some(source_instance),
            world_from_model,
            normal_from_model,
            tint,
        }
    }

    pub(in crate::render) const fn auto_batched(
        world_from_model: [f32; 16],
        normal_from_model: [f32; 16],
        tint: Color,
    ) -> Self {
        Self {
            source_instance: None,
            world_from_model,
            normal_from_model,
            tint,
        }
    }

    pub(in crate::render) const fn world_from_model(self) -> [f32; 16] {
        self.world_from_model
    }

    pub(in crate::render) const fn normal_from_model(self) -> [f32; 16] {
        self.normal_from_model
    }

    pub(in crate::render) const fn tint(self) -> Color {
        self.tint
    }

    pub(in crate::render) const fn source_instance(self) -> Option<InstanceId> {
        self.source_instance
    }
}
