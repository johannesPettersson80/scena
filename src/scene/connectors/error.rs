use std::{error, fmt};

use super::super::{AnchorKey, ConnectorKey, NodeKey, SourceCoordinateSystem, SourceUnits};

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionError {
    NodeNotFound(NodeKey),
    MissingAnchor {
        anchor: AnchorKey,
    },
    MissingAnchorName {
        name: String,
    },
    AmbiguousAnchor {
        name: String,
        matches: Vec<AnchorKey>,
    },
    StaleAnchorHandle {
        anchor: Option<AnchorKey>,
        name: Option<String>,
    },
    MissingConnector {
        connector: ConnectorKey,
    },
    MissingConnectorName {
        name: String,
    },
    AmbiguousConnector {
        name: String,
        matches: Vec<ConnectorKey>,
    },
    AmbiguousImportConnector {
        name: String,
        hosts: Vec<NodeKey>,
    },
    StaleConnectorHandle {
        connector: Option<ConnectorKey>,
        name: Option<String>,
    },
    IncompatibleConnector {
        source_kind: String,
        target_kind: String,
    },
    UnitMismatch {
        source_units: SourceUnits,
        target_units: SourceUnits,
    },
    CoordinateSystemMismatch {
        source_coordinate_system: SourceCoordinateSystem,
        target_coordinate_system: SourceCoordinateSystem,
    },
    DegenerateConnectorFrame {
        connector: Option<String>,
    },
    HandednessMismatch {
        connector: Option<String>,
        coordinate_system: SourceCoordinateSystem,
    },
    NonUniformScaleConnectionRisk {
        node: NodeKey,
    },
    FlippedConnection {
        connector: Option<String>,
        node: Option<NodeKey>,
    },
    ConnectionWouldMoveLockedNode {
        node: NodeKey,
    },
    ConnectionWouldCreateCycle {
        source: NodeKey,
        parent: NodeKey,
    },
    ConnectorHostNotPrepared {
        node: NodeKey,
        connector: Option<String>,
    },
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeNotFound(node) => write!(formatter, "connector node {node:?} was not found"),
            Self::MissingAnchor { anchor } => {
                write!(formatter, "anchor handle {anchor:?} was not found")
            }
            Self::MissingAnchorName { name } => {
                write!(formatter, "anchor name '{name}' was not found")
            }
            Self::AmbiguousAnchor { name, matches } => write!(
                formatter,
                "anchor name '{name}' is ambiguous across {} handles",
                matches.len()
            ),
            Self::StaleAnchorHandle { anchor, name } => write!(
                formatter,
                "anchor handle {anchor:?}{} is stale",
                name.as_deref()
                    .map(|name| format!(" for '{name}'"))
                    .unwrap_or_default()
            ),
            Self::MissingConnector { connector } => {
                write!(formatter, "connector handle {connector:?} was not found")
            }
            Self::MissingConnectorName { name } => {
                write!(formatter, "connector name '{name}' was not found")
            }
            Self::AmbiguousConnector { name, matches } => write!(
                formatter,
                "connector name '{name}' is ambiguous across {} handles",
                matches.len()
            ),
            Self::AmbiguousImportConnector { name, hosts } => write!(
                formatter,
                "imported connector name '{name}' is ambiguous across {} host nodes",
                hosts.len()
            ),
            Self::StaleConnectorHandle { connector, name } => write!(
                formatter,
                "connector handle {connector:?}{} is stale",
                name.as_deref()
                    .map(|name| format!(" for '{name}'"))
                    .unwrap_or_default()
            ),
            Self::IncompatibleConnector {
                source_kind,
                target_kind,
            } => write!(
                formatter,
                "connector kinds are incompatible: source={source_kind}, target={target_kind}"
            ),
            Self::UnitMismatch {
                source_units,
                target_units,
            } => write!(
                formatter,
                "connector source units are incompatible: source={source_units:?}, target={target_units:?}"
            ),
            Self::CoordinateSystemMismatch {
                source_coordinate_system,
                target_coordinate_system,
            } => write!(
                formatter,
                "connector source coordinate systems are incompatible: source={source_coordinate_system:?}, target={target_coordinate_system:?}"
            ),
            Self::DegenerateConnectorFrame { connector } => write!(
                formatter,
                "connector frame{} is degenerate",
                connector
                    .as_deref()
                    .map(|name| format!(" '{name}'"))
                    .unwrap_or_default()
            ),
            Self::HandednessMismatch {
                connector,
                coordinate_system,
            } => write!(
                formatter,
                "connector{} comes from unsupported left-handed coordinate system {coordinate_system:?}",
                connector
                    .as_deref()
                    .map(|name| format!(" '{name}'"))
                    .unwrap_or_default()
            ),
            Self::NonUniformScaleConnectionRisk { node } => write!(
                formatter,
                "node {node:?} has non-uniform scale that could skew connector placement"
            ),
            Self::FlippedConnection { connector, node } => write!(
                formatter,
                "connector{}{} has a negative-determinant scale that would mirror placement",
                connector
                    .as_deref()
                    .map(|name| format!(" '{name}'"))
                    .unwrap_or_default(),
                node.map(|node| format!(" on node {node:?}"))
                    .unwrap_or_default()
            ),
            Self::ConnectionWouldMoveLockedNode { node } => {
                write!(formatter, "connection would move locked node {node:?}")
            }
            Self::ConnectionWouldCreateCycle { source, parent } => write!(
                formatter,
                "connection would reparent node {source:?} under descendant {parent:?}"
            ),
            Self::ConnectorHostNotPrepared { node, connector } => write!(
                formatter,
                "connector{} is attached to unprepared host node {node:?}",
                connector
                    .as_deref()
                    .map(|name| format!(" '{name}'"))
                    .unwrap_or_default()
            ),
        }
    }
}

impl error::Error for ConnectionError {}
