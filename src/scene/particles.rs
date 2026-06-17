use std::error;
use std::fmt;

use crate::diagnostics::LookupError;
use crate::geometry::Aabb;
use crate::material::Color;

use super::{NodeKey, NodeKind, ParticleSetKey, Scene, Transform, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Particle {
    position: Vec3,
    color: Color,
    size_px: f32,
    rotation_radians: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParticleSet {
    particles: Vec<Particle>,
    bounds: Aabb,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParticleSetError {
    Empty,
    InvalidPosition { index: usize },
    InvalidColor { index: usize },
    InvalidSize { index: usize },
    InvalidRotation { index: usize },
}

impl Scene {
    pub fn add_particle_set(
        &mut self,
        parent: NodeKey,
        particles: ParticleSet,
        transform: Transform,
    ) -> Result<ParticleSetKey, LookupError> {
        self.add_particle_set_node(parent, particles, transform)
            .map(|(_, particle_set)| particle_set)
    }

    pub fn add_particle_set_node(
        &mut self,
        parent: NodeKey,
        particles: ParticleSet,
        transform: Transform,
    ) -> Result<(NodeKey, ParticleSetKey), LookupError> {
        let bounds = particles.bounds();
        let particle_set = self.particle_sets.insert(particles);
        match self.insert_node(parent, NodeKind::ParticleSet(particle_set), transform) {
            Ok(node) => {
                self.node_bounds.insert(node, bounds);
                Ok((node, particle_set))
            }
            Err(error) => {
                self.particle_sets.remove(particle_set);
                Err(error)
            }
        }
    }

    pub fn particle_set(&self, particle_set: ParticleSetKey) -> Option<&ParticleSet> {
        self.particle_sets.get(particle_set)
    }

    pub fn set_particle_set(
        &mut self,
        particle_set: ParticleSetKey,
        replacement: ParticleSet,
    ) -> Result<(), LookupError> {
        let bounds = replacement.bounds();
        if !self.particle_sets.contains_key(particle_set) {
            return Err(LookupError::ParticleSetNotFound(particle_set));
        }
        if self.particle_sets.get(particle_set) == Some(&replacement) {
            return Ok(());
        }
        let nodes = self
            .nodes
            .iter()
            .filter_map(|(node, node_data)| {
                (node_data.kind == NodeKind::ParticleSet(particle_set)).then_some(node)
            })
            .collect::<Vec<_>>();
        *self
            .particle_sets
            .get_mut(particle_set)
            .expect("particle set existence checked") = replacement;
        for node in nodes {
            self.node_bounds.insert(node, bounds);
        }
        self.structure_revision = self.structure_revision.saturating_add(1);
        Ok(())
    }

    pub(crate) fn particle_set_nodes(
        &self,
    ) -> impl Iterator<Item = (NodeKey, &ParticleSet, Transform)> + '_ {
        self.nodes.iter().filter_map(|(node_key, node)| {
            let NodeKind::ParticleSet(particle_set) = node.kind else {
                return None;
            };
            if !self.visible_for_active_camera(node_key) {
                return None;
            }
            self.particle_sets
                .get(particle_set)
                .and_then(|particle_set| {
                    self.world_transform(node_key)
                        .map(|transform| (node_key, particle_set, transform))
                })
        })
    }
}

impl Particle {
    pub const fn new(position: Vec3, color: Color, size_px: f32) -> Self {
        Self {
            position,
            color,
            size_px,
            rotation_radians: 0.0,
        }
    }

    pub const fn with_rotation_radians(mut self, rotation_radians: f32) -> Self {
        self.rotation_radians = rotation_radians;
        self
    }

    pub const fn position(self) -> Vec3 {
        self.position
    }

    pub const fn color(self) -> Color {
        self.color
    }

    pub const fn size_px(self) -> f32 {
        self.size_px
    }

    pub const fn rotation_radians(self) -> f32 {
        self.rotation_radians
    }
}

impl ParticleSet {
    pub fn try_new(particles: Vec<Particle>) -> Result<Self, ParticleSetError> {
        validate_particles(&particles)?;
        let bounds = particle_bounds(&particles).expect("validated particle set is non-empty");
        Ok(Self { particles, bounds })
    }

    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    pub const fn bounds(&self) -> Aabb {
        self.bounds
    }

    pub const fn len(&self) -> usize {
        self.particles.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }
}

impl fmt::Display for ParticleSetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("particle set must contain at least one particle"),
            Self::InvalidPosition { index } => {
                write!(formatter, "particle {index} position must be finite")
            }
            Self::InvalidColor { index } => {
                write!(formatter, "particle {index} color channels must be finite")
            }
            Self::InvalidSize { index } => {
                write!(
                    formatter,
                    "particle {index} size_px must be finite and positive"
                )
            }
            Self::InvalidRotation { index } => {
                write!(
                    formatter,
                    "particle {index} rotation_radians must be finite"
                )
            }
        }
    }
}

impl error::Error for ParticleSetError {}

fn validate_particles(particles: &[Particle]) -> Result<(), ParticleSetError> {
    if particles.is_empty() {
        return Err(ParticleSetError::Empty);
    }
    for (index, particle) in particles.iter().enumerate() {
        let position = particle.position();
        if !position.x.is_finite() || !position.y.is_finite() || !position.z.is_finite() {
            return Err(ParticleSetError::InvalidPosition { index });
        }
        let color = particle.color();
        if !color.r.is_finite()
            || !color.g.is_finite()
            || !color.b.is_finite()
            || !color.a.is_finite()
        {
            return Err(ParticleSetError::InvalidColor { index });
        }
        if !particle.size_px().is_finite() || particle.size_px() <= 0.0 {
            return Err(ParticleSetError::InvalidSize { index });
        }
        if !particle.rotation_radians().is_finite() {
            return Err(ParticleSetError::InvalidRotation { index });
        }
    }
    Ok(())
}

fn particle_bounds(particles: &[Particle]) -> Option<Aabb> {
    let first = particles.first()?.position();
    let mut min = first;
    let mut max = first;
    for particle in &particles[1..] {
        let position = particle.position();
        min.x = min.x.min(position.x);
        min.y = min.y.min(position.y);
        min.z = min.z.min(position.z);
        max.x = max.x.max(position.x);
        max.y = max.y.max(position.y);
        max.z = max.z.max(position.z);
    }
    Some(Aabb::new(min, max))
}
