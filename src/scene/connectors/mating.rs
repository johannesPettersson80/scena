//! Where a connection puts its source node.
//!
//! A connection mates two connector frames. When every frame involved is
//! exactly translation, rotation, and scale, the frames are rigid up to scale
//! and the solver composes them directly, as it always has. A connector under
//! a rotated, non-uniformly scaled ancestor is sheared in the world and has no
//! such frame, so the solver then mates the *rigid frames* of the two
//! connectors, each read from its exact world matrix:
//!
//! - the origin is the connector origin, exactly;
//! - the mating axis is the connector's X axis, exactly;
//! - the up axis is the connector's Y axis made perpendicular to X.
//!
//! The source node keeps its local scale and is turned and moved under its
//! exact parent matrix until its connector's rigid frame coincides with the
//! mated target frame. Those three constraints are everything a rotation and
//! a translation can satisfy, so they hold for any invertible parent. What a
//! rigid frame cannot hold, the skew of a sheared connector's Y and Z axes,
//! is the only part left unmatched.

use super::super::transforms::compose_transform;
use super::super::{Transform, Vec3, WorldTransform, aim_rotation};
use super::roll::roll_transform;
use super::scale::preserve_source_scale;
use super::solving::{effective_mate_offset, effective_roll};
use super::validation::inverse_transform;
use super::{ConnectOptions, ConnectionError, ConnectorFrame};

pub(super) struct MatingInputs<'a> {
    pub(super) source: &'a ConnectorFrame,
    pub(super) target: &'a ConnectorFrame,
    pub(super) options: ConnectOptions,
    /// Exact world placement of the node each connector is attached to.
    pub(super) source_host: WorldTransform,
    pub(super) target_host: WorldTransform,
    /// Exact world placement of the node the source will hang under.
    pub(super) source_parent: WorldTransform,
    /// The source node's current local transform, whose scale is kept.
    pub(super) source_local: Transform,
}

pub(super) struct Mating {
    /// The source node's new local transform under its resolved parent.
    pub(super) resolved_transform: Transform,
    /// World position of the source connector before the move.
    pub(super) source_origin: Vec3,
    /// World position the source connector is moved to.
    pub(super) mated_origin: Vec3,
    pub(super) snap_distance: f32,
}

pub(super) fn solve_mating(inputs: &MatingInputs<'_>) -> Result<Mating, ConnectionError> {
    let target_frame = inputs.target_host.then(inputs.target.local_placement);
    let source_frame = inputs.source_host.then(inputs.source.local_placement);
    match RigidTrsFrames::of(inputs, target_frame, source_frame) {
        Some(frames) => Ok(frames.mate(inputs)),
        None => mate_rigid_frames(inputs, target_frame, source_frame),
    }
}

/// Every frame of a connection as one translation, rotation, and scale, with
/// the inverses the composition needs. Present only when each is exact.
struct RigidTrsFrames {
    target_connector: Transform,
    source_connector: Transform,
    source_host: Transform,
    source_connector_inverse: Transform,
    parent_inverse: Transform,
}

impl RigidTrsFrames {
    fn of(
        inputs: &MatingInputs<'_>,
        target_frame: WorldTransform,
        source_frame: WorldTransform,
    ) -> Option<Self> {
        Some(Self {
            target_connector: target_frame.exact_trs()?,
            source_connector: source_frame.exact_trs()?,
            source_host: inputs.source_host.exact_trs()?,
            // Only a uniform scale has a translation, rotation, and scale
            // inverse; any other frame is mated through the matrix.
            source_connector_inverse: inverse_transform(inputs.source.local_transform)?,
            parent_inverse: inverse_transform(inputs.source_parent.exact_trs()?)?,
        })
    }

    fn mate(&self, inputs: &MatingInputs<'_>) -> Mating {
        let snap_distance = self
            .source_connector
            .translation
            .distance(self.target_connector.translation);
        let mate_offset = effective_mate_offset(inputs.options, inputs.source, inputs.target);
        let target_offset = compose_transform(self.target_connector, mate_offset);
        let target_aligned = compose_transform(target_offset, inputs.options.alignment_transform());
        let target_mated = compose_transform(
            target_aligned,
            roll_transform(
                effective_roll(inputs.options, inputs.source),
                self.source_connector,
                target_aligned,
            ),
        );
        let desired_source_world = preserve_source_scale(
            compose_transform(target_mated, self.source_connector_inverse),
            self.source_host.scale,
            inputs.source.local_transform,
            target_mated.translation,
        );
        Mating {
            resolved_transform: compose_transform(self.parent_inverse, desired_source_world),
            source_origin: self.source_connector.translation,
            mated_origin: target_mated.translation,
            snap_distance,
        }
    }
}

/// Mates the rigid frames of two connectors when either is sheared in the
/// world or sits under a parent that translation, rotation, and scale cannot
/// invert.
fn mate_rigid_frames(
    inputs: &MatingInputs<'_>,
    target_frame: WorldTransform,
    source_frame: WorldTransform,
) -> Result<Mating, ConnectionError> {
    let degenerate = |connector: &ConnectorFrame| ConnectionError::DegenerateConnectorFrame {
        connector: connector.name.clone(),
    };
    let target = target_frame
        .rigid_frame()
        .ok_or_else(|| degenerate(inputs.target))?;
    let source = source_frame
        .rigid_frame()
        .ok_or_else(|| degenerate(inputs.source))?;

    // The offset, alignment, and roll are authored in the target connector's
    // own space. Their rotations turn its rigid frame; their translation is a
    // point of that space, carried into the world by the exact matrix.
    let mate_offset = effective_mate_offset(inputs.options, inputs.source, inputs.target);
    let aligned = compose_transform(mate_offset, inputs.options.alignment_transform());
    let roll = roll_transform(
        effective_roll(inputs.options, inputs.source),
        Transform {
            rotation: source.rotation,
            ..Transform::IDENTITY
        },
        Transform {
            rotation: (target.rotation * aligned.rotation).normalize(),
            ..Transform::IDENTITY
        },
    );
    let mated = compose_transform(aligned, roll);
    let mated_origin = target_frame.transform_point(mated.translation);
    let mated_rotation = (target.rotation * mated.rotation).normalize();

    // The connector's X and Y axes in the space of the source node's parent,
    // before the node's rotation: the node's local scale applied to the
    // connector's own exact axes.
    let scale = inputs.source_local.scale;
    let connector = inputs.source.local_placement;
    let rotation = aim_rotation(
        inputs.source_parent,
        scale * connector.transform_vector(Vec3::X),
        scale * connector.transform_vector(Vec3::Y),
        mated_rotation * Vec3::X,
        mated_rotation * Vec3::Y,
    )
    .ok_or_else(|| degenerate(inputs.source))?;
    let local_from_world = inputs
        .source_parent
        .inverse()
        .ok_or_else(|| degenerate(inputs.source))?;
    let translation = local_from_world.transform_point(mated_origin)
        - rotation * (scale * connector.translation());
    if !translation.is_finite() {
        return Err(degenerate(inputs.source));
    }
    Ok(Mating {
        resolved_transform: Transform {
            translation,
            rotation,
            scale,
        },
        source_origin: source.origin,
        mated_origin,
        snap_distance: source.origin.distance(target.origin),
    })
}
