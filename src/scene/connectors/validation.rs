use super::super::transforms::rotate_vec3;
use super::super::{ConnectorKey, NodeKey, NodeKind, Quat, Transform, Vec3};
use super::{ConnectOptions, ConnectionError, ConnectorFrame};

pub(super) fn validate_connector_live(
    connector: &ConnectorFrame,
    key: Option<ConnectorKey>,
) -> Result<(), ConnectionError> {
    if connector.is_live() {
        Ok(())
    } else {
        Err(ConnectionError::StaleConnectorHandle {
            connector: key,
            name: connector.name.clone(),
        })
    }
}

pub(super) fn validate_connector_kinds(
    source: &ConnectorFrame,
    target: &ConnectorFrame,
) -> Result<(), ConnectionError> {
    match (source.kind(), target.kind()) {
        (Some(source_kind), Some(target_kind))
            if source_kind != target_kind
                && !connectors_explicitly_allow_mate(source, source_kind, target, target_kind) =>
        {
            Err(ConnectionError::IncompatibleConnector {
                source_kind: source_kind.to_string(),
                target_kind: target_kind.to_string(),
            })
        }
        (Some(_), Some(_)) | (Some(_), None) | (None, Some(_)) | (None, None) => Ok(()),
    }
}

pub(super) fn validate_connector_source_metadata(
    source: &ConnectorFrame,
    target: &ConnectorFrame,
) -> Result<(), ConnectionError> {
    if source.import_live.is_none()
        && target.import_live.is_none()
        && source.source_units() != target.source_units()
    {
        return Err(ConnectionError::UnitMismatch {
            source_units: source.source_units(),
            target_units: target.source_units(),
        });
    }
    if source.import_live.is_none()
        && target.import_live.is_none()
        && source.source_coordinate_system() != target.source_coordinate_system()
    {
        return Err(ConnectionError::CoordinateSystemMismatch {
            source_coordinate_system: source.source_coordinate_system(),
            target_coordinate_system: target.source_coordinate_system(),
        });
    }
    Ok(())
}

fn connectors_explicitly_allow_mate(
    source: &ConnectorFrame,
    source_kind: &str,
    target: &ConnectorFrame,
    target_kind: &str,
) -> bool {
    source.allowed_mates.iter().any(|kind| kind == target_kind)
        || target.allowed_mates.iter().any(|kind| kind == source_kind)
}

pub(super) fn validate_connector_handedness(
    connector: &ConnectorFrame,
) -> Result<(), ConnectionError> {
    if connector.source_coordinate_system().is_left_handed() {
        return Err(ConnectionError::HandednessMismatch {
            connector: connector.name.clone(),
            coordinate_system: connector.source_coordinate_system(),
        });
    }
    Ok(())
}

pub(super) fn validate_connector_host_prepared(
    connector: &ConnectorFrame,
    kind: &NodeKind,
) -> Result<(), ConnectionError> {
    if matches!(kind, NodeKind::Model(_)) {
        return Err(ConnectionError::ConnectorHostNotPrepared {
            node: connector.node,
            connector: connector.name.clone(),
        });
    }
    Ok(())
}

pub(super) fn validate_connector_transform(
    connector: &ConnectorFrame,
    options: ConnectOptions,
) -> Result<(), ConnectionError> {
    if !is_valid_scale(connector.local_transform.scale)
        || !is_valid_rotation(connector.local_transform.rotation)
    {
        return Err(ConnectionError::DegenerateConnectorFrame {
            connector: connector.name.clone(),
        });
    }
    if has_negative_determinant(connector.local_transform.scale) {
        return Err(ConnectionError::FlippedConnection {
            connector: connector.name.clone(),
            node: None,
        });
    }
    if !options.allow_non_uniform_scale && !is_uniform_scale(connector.local_transform.scale) {
        return Err(ConnectionError::NonUniformScaleConnectionRisk {
            node: connector.node,
        });
    }
    Ok(())
}

pub(super) fn validate_node_transform(
    node: NodeKey,
    transform: Transform,
    options: ConnectOptions,
) -> Result<(), ConnectionError> {
    validate_transform_scale(node, transform, options)
}

pub(super) fn validate_transform_scale(
    node: NodeKey,
    transform: Transform,
    options: ConnectOptions,
) -> Result<(), ConnectionError> {
    if !is_valid_scale(transform.scale) || !is_valid_rotation(transform.rotation) {
        return Err(ConnectionError::DegenerateConnectorFrame { connector: None });
    }
    if has_negative_determinant(transform.scale) {
        return Err(ConnectionError::FlippedConnection {
            connector: None,
            node: Some(node),
        });
    }
    if !options.allow_non_uniform_scale && !is_uniform_scale(transform.scale) {
        return Err(ConnectionError::NonUniformScaleConnectionRisk { node });
    }
    Ok(())
}

pub(super) fn inverse_transform(transform: Transform) -> Option<Transform> {
    if !is_valid_scale(transform.scale) || !is_uniform_scale(transform.scale) {
        return None;
    }
    let uniform_scale = transform.scale.x;
    let inverse_scale = uniform_scale.recip();
    let inverse_rotation = inverse_quat(transform.rotation)?;
    let inverse_translation = scale_vec3(
        rotate_vec3(inverse_rotation, negate_vec3(transform.translation)),
        inverse_scale,
    );
    Some(Transform {
        translation: inverse_translation,
        rotation: inverse_rotation,
        scale: Vec3::new(inverse_scale, inverse_scale, inverse_scale),
    })
}

fn inverse_quat(rotation: Quat) -> Option<Quat> {
    let length_squared = rotation.x * rotation.x
        + rotation.y * rotation.y
        + rotation.z * rotation.z
        + rotation.w * rotation.w;
    if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        return None;
    }
    let inverse_length_squared = length_squared.recip();
    Some(Quat {
        x: -rotation.x * inverse_length_squared,
        y: -rotation.y * inverse_length_squared,
        z: -rotation.z * inverse_length_squared,
        w: rotation.w * inverse_length_squared,
    })
}

fn is_valid_scale(scale: Vec3) -> bool {
    scale.x.is_finite()
        && scale.y.is_finite()
        && scale.z.is_finite()
        && scale.x.abs() > f32::EPSILON
        && scale.y.abs() > f32::EPSILON
        && scale.z.abs() > f32::EPSILON
}

fn is_uniform_scale(scale: Vec3) -> bool {
    (scale.x - scale.y).abs() <= 1.0e-5 && (scale.x - scale.z).abs() <= 1.0e-5
}

fn has_negative_determinant(scale: Vec3) -> bool {
    scale.x * scale.y * scale.z < 0.0
}

fn is_valid_rotation(rotation: Quat) -> bool {
    let length_squared = rotation.x * rotation.x
        + rotation.y * rotation.y
        + rotation.z * rotation.z
        + rotation.w * rotation.w;
    length_squared.is_finite() && length_squared > f32::EPSILON
}

const fn negate_vec3(value: Vec3) -> Vec3 {
    Vec3::new(-value.x, -value.y, -value.z)
}

const fn scale_vec3(value: Vec3, scale: f32) -> Vec3 {
    Vec3::new(value.x * scale, value.y * scale, value.z * scale)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::transforms::{compose_transform, multiply_quat};

    #[test]
    fn inverse_round_trips_uniform_transform() {
        let transform = Transform::at(Vec3::new(2.0, 3.0, 4.0))
            .rotate_z_deg(90.0)
            .scale_by(2.0);
        let inverse = inverse_transform(transform).expect("uniform transform inverts");
        let round_trip = compose_transform(transform, inverse);

        assert!((round_trip.translation.x).abs() < 1.0e-5);
        assert!((round_trip.translation.y).abs() < 1.0e-5);
        assert!((round_trip.translation.z).abs() < 1.0e-5);
        assert!((round_trip.scale.x - 1.0).abs() < 1.0e-5);
        assert!((round_trip.scale.y - 1.0).abs() < 1.0e-5);
        assert!((round_trip.scale.z - 1.0).abs() < 1.0e-5);
        let identity_rotation = multiply_quat(transform.rotation, inverse.rotation);
        assert!(identity_rotation.x.abs() < 1.0e-5);
        assert!(identity_rotation.y.abs() < 1.0e-5);
        assert!(identity_rotation.z.abs() < 1.0e-5);
        assert!((identity_rotation.w - 1.0).abs() < 1.0e-5);
    }
}
