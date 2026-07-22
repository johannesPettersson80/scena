use std::fmt;

use super::LookupError;

impl fmt::Display for LookupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeNotFound(_) => write!(formatter, "node key does not exist in the scene"),
            Self::CannotRemoveRootNode(_) => {
                write!(formatter, "the scene root node cannot be removed")
            }
            Self::NodeNameNotFound { name, candidates } => {
                write_missing_with_candidates(formatter, "node", name, candidates)
            }
            Self::AmbiguousNodeName { name, matches } => write!(
                formatter,
                "imported scene node name '{name}' is ambiguous across {} nodes",
                matches.len()
            ),
            Self::AnchorNotFound { name, candidates } => {
                write_missing_with_candidates(formatter, "anchor", name, candidates)
            }
            Self::AmbiguousAnchorName { name, hosts } => write!(
                formatter,
                "imported scene anchor name '{name}' is ambiguous across {} host nodes",
                hosts.len()
            ),
            Self::ConnectorNotFound { name, candidates } => {
                write_missing_with_candidates(formatter, "connector", name, candidates)
            }
            Self::AmbiguousConnectorName { name, hosts } => write!(
                formatter,
                "imported scene connector name '{name}' is ambiguous across {} host nodes",
                hosts.len()
            ),
            Self::ClipNotFound { name, candidates } => {
                write_missing_with_candidates(formatter, "animation clip", name, candidates)
            }
            Self::VariantNotFound { name, candidates } => write_missing_with_candidates(
                formatter,
                "KHR_materials_variants variant",
                name,
                candidates,
            ),
            Self::AmbiguousVariantName { name, matches } => write!(
                formatter,
                "imported scene KHR_materials_variants name '{name}' is ambiguous across {} variants",
                matches.len()
            ),
            Self::AmbiguousClipName { name, matches } => write!(
                formatter,
                "imported scene animation clip name '{name}' is ambiguous across {} clips",
                matches.len()
            ),
            Self::PathNotFound { path } => {
                write!(formatter, "imported scene path '{path}' was not found")
            }
            Self::InvalidViewport { width, height } => write!(
                formatter,
                "viewport {width}x{height} is invalid; width and height must be non-zero"
            ),
            Self::InvalidBounds { reason } => write!(formatter, "bounds are invalid: {reason}"),
            Self::InvalidFramingOption { field, reason } => write!(
                formatter,
                "camera framing option '{field}' is invalid: {reason}"
            ),
            Self::UnsupportedCameraType {
                camera,
                operation,
                supported,
            } => write!(
                formatter,
                "{operation} does not support camera {camera:?}; supported camera type: {supported}"
            ),
            Self::ImportHasNoBounds => write!(
                formatter,
                "imported scene has no renderable bounds to frame"
            ),
            Self::StaleImport => write!(formatter, "scene import has been invalidated"),
            Self::ImportFromDifferentScene => {
                write!(formatter, "scene import belongs to a different scene")
            }
            Self::NodeIsNotMesh { node } => write!(formatter, "node {node:?} is not a mesh node"),
            Self::NonInvertibleParentTransform { node, parent } => write!(
                formatter,
                "node {node:?} cannot be placed in world space because parent {parent:?} has a non-invertible transform"
            ),
            Self::InvalidTransform { reason } => {
                write!(formatter, "transform is invalid: {reason}")
            }
            Self::GeometryNotFound { node, .. } => write!(
                formatter,
                "geometry for mesh node {node:?} was not found in Assets"
            ),
            Self::InvalidSkinBinding {
                joint_count,
                inverse_bind_count,
            } => write!(
                formatter,
                "skin binding has {joint_count} joints but {inverse_bind_count} inverse bind matrices"
            ),
            Self::CameraNotFound(_) => write!(formatter, "camera key does not exist in the scene"),
            Self::ClippingPlaneNotFound(_) => {
                write!(formatter, "clipping plane key does not exist in the scene")
            }
            Self::InstanceSetNotFound(_) => {
                write!(formatter, "instance set key does not exist in the scene")
            }
            Self::ParticleSetNotFound(_) => {
                write!(formatter, "particle set key does not exist in the scene")
            }
            Self::InstanceNotFound {
                instance_set,
                instance,
            } => write!(
                formatter,
                "instance {:?} does not exist in instance set {:?}",
                instance, instance_set
            ),
            Self::InvalidInstanceTint {
                instance_set,
                instance,
                reason,
            } => write!(
                formatter,
                "instance {:?} in instance set {:?} has invalid tint: {reason}",
                instance, instance_set
            ),
            Self::LabelNotFound(_) => write!(formatter, "label key does not exist in the scene"),
            Self::UnsupportedLabelText { reason, .. } => write!(
                formatter,
                "label text is not supported by its font: {reason}"
            ),
            Self::InvalidLabelStyle { field, reason } => {
                write!(formatter, "{field} is not supported: {reason}")
            }
        }
    }
}

fn write_missing_with_candidates(
    formatter: &mut fmt::Formatter<'_>,
    kind: &str,
    name: &str,
    candidates: &[String],
) -> fmt::Result {
    write!(formatter, "imported scene has no {kind} named '{name}'")?;
    if !candidates.is_empty() {
        write!(formatter, "; nearest candidates: {}", candidates.join(", "))?;
    }
    Ok(())
}
