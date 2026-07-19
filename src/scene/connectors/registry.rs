use super::super::{ConnectorKey, Scene, slot_index};
use super::{ConnectionError, ConnectorFrame, validate_connector_live};

impl Scene {
    pub fn connector(&self, connector: ConnectorKey) -> Result<&ConnectorFrame, ConnectionError> {
        let Some(frame) = self.connectors.get(connector) else {
            if let Some((retired, name)) = self.retired_connectors.get(&slot_index(connector))
                && *retired == connector
            {
                return Err(ConnectionError::StaleConnectorHandle {
                    connector: Some(connector),
                    name: name.clone(),
                });
            }
            return Err(ConnectionError::MissingConnector { connector });
        };
        validate_connector_live(frame, Some(connector))?;
        if !self.nodes.contains_key(frame.node) {
            return Err(ConnectionError::NodeNotFound(frame.node));
        }
        Ok(frame)
    }
}
