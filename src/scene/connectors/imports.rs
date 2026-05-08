use crate::diagnostics::LookupError;

use super::super::Scene;
use super::super::import::SceneImport;
use super::{ConnectOptions, ConnectionError, ConnectionPreview, ConnectorFrame};

impl Scene {
    pub fn connect_import_connectors(
        &mut self,
        source_import: &SceneImport,
        source_name: &str,
        target_import: &SceneImport,
        target_name: &str,
        options: ConnectOptions,
    ) -> Result<ConnectionPreview, ConnectionError> {
        let source = source_import
            .connector(source_name)
            .map(ConnectorFrame::from_import_connector)
            .map_err(|error| connector_lookup_error(error, source_name))?;
        let target = target_import
            .connector(target_name)
            .map(ConnectorFrame::from_import_connector)
            .map_err(|error| connector_lookup_error(error, target_name))?;
        self.connect(source, target, options)
    }
}

fn connector_lookup_error(error: LookupError, requested_name: &str) -> ConnectionError {
    match error {
        LookupError::ConnectorNotFound { name } => ConnectionError::MissingConnectorName { name },
        LookupError::AmbiguousConnectorName { name, hosts } => {
            ConnectionError::AmbiguousImportConnector { name, hosts }
        }
        LookupError::StaleImport => ConnectionError::StaleConnectorHandle {
            connector: None,
            name: Some(requested_name.to_string()),
        },
        LookupError::NodeNotFound(node) => ConnectionError::NodeNotFound(node),
        LookupError::AmbiguousNodeName { matches, .. } => {
            ConnectionError::AmbiguousImportConnector {
                name: requested_name.to_string(),
                hosts: matches,
            }
        }
        LookupError::NodeNameNotFound { .. }
        | LookupError::AnchorNotFound { .. }
        | LookupError::AmbiguousAnchorName { .. }
        | LookupError::ClipNotFound { .. }
        | LookupError::AmbiguousClipName { .. }
        | LookupError::PathNotFound { .. }
        | LookupError::InvalidViewport { .. }
        | LookupError::ImportHasNoBounds
        | LookupError::NodeIsNotMesh { .. }
        | LookupError::NonInvertibleParentTransform { .. }
        | LookupError::GeometryNotFound { .. }
        | LookupError::CameraNotFound(_)
        | LookupError::ClippingPlaneNotFound(_)
        | LookupError::InstanceSetNotFound(_)
        | LookupError::LabelNotFound(_) => ConnectionError::MissingConnectorName {
            name: requested_name.to_string(),
        },
    }
}
