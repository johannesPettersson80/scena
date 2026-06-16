use super::metadata::ConnectorRollPolicy;
use super::options::{ConnectOptions, ConnectionRoll};
use super::{ConnectionError, ConnectionPreview, ConnectorFrame, Transform};

pub(super) fn effective_mate_offset(
    options: ConnectOptions,
    source: &ConnectorFrame,
    target: &ConnectorFrame,
) -> Transform {
    let clearance = source
        .clearance_hint()
        .into_iter()
        .chain(target.clearance_hint())
        .fold(0.0, f32::max);
    let mut offset = options.mate_offset;
    if clearance > offset.translation.x {
        offset.translation.x = clearance;
    }
    offset
}

pub(super) fn effective_roll(options: ConnectOptions, source: &ConnectorFrame) -> ConnectionRoll {
    if options.roll() != ConnectionRoll::MatchTarget {
        return options.roll();
    }
    match source.roll_policy() {
        ConnectorRollPolicy::Preserve => ConnectionRoll::PreserveSource,
        ConnectorRollPolicy::ChooseNearest => ConnectionRoll::ChooseNearest { step_degrees: 90.0 },
    }
}

pub(super) fn validate_snap_tolerance_for_apply(
    preview: &ConnectionPreview,
    options: ConnectOptions,
) -> Result<(), ConnectionError> {
    if options.mate_offset != Transform::IDENTITY {
        return Ok(());
    }
    let tolerance = preview
        .source
        .snap_tolerance()
        .into_iter()
        .chain(preview.target.snap_tolerance())
        .fold(0.0, f32::max);
    if tolerance <= 0.0 {
        return Ok(());
    }
    let distance = preview.snap_distance;
    if distance <= tolerance {
        Ok(())
    } else {
        Err(ConnectionError::SnapToleranceExceeded {
            distance,
            tolerance,
        })
    }
}

impl PartialEq for ConnectorFrame {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
            && self.local_transform == other.local_transform
            && self.name == other.name
            && self.kind == other.kind
            && self.allowed_mates == other.allowed_mates
            && self.tags == other.tags
            && self.snap_tolerance == other.snap_tolerance
            && self.clearance_hint == other.clearance_hint
            && self.roll_policy == other.roll_policy
            && self.polarity == other.polarity
            && self.metadata == other.metadata
            && self.source_units == other.source_units
            && self.source_coordinate_system == other.source_coordinate_system
    }
}
