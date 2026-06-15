use std::collections::BTreeSet;

use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{
    AssetFetcher, ConnectOptions, ConnectionError, ConnectionMagnetVisualCue, ConnectorFrame,
    ConnectorPolarity, ConnectorRollPolicy, NodeKey, SceneImport, Transform,
};

mod types;
pub use types::{
    CONNECTOR_BROWSER_SCHEMA_V1, ConnectorBrowserCandidateV1, ConnectorBrowserConnectorV1,
    ConnectorBrowserReportV1, ConnectorBrowserScopeV1, ConnectorBrowserSummaryV1,
    ConnectorBrowserVisualCueV1, ConnectorLineV1, ConnectorTransformV1,
};

#[derive(Clone)]
struct ConnectorRecord {
    report: ConnectorBrowserConnectorV1,
    frame: ConnectorFrame,
    node: NodeKey,
    placement_node: NodeKey,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn connector_browser_json(
        &mut self,
        source_import: u64,
        target_imports: &[u64],
    ) -> Result<String, SceneHostError> {
        let source = self.resolve_import(source_import)?.clone();
        let targets = target_imports
            .iter()
            .copied()
            .map(|handle| Ok((handle, self.resolve_import(handle)?.clone())))
            .collect::<Result<Vec<_>, SceneHostError>>()?;
        let target_refs = targets
            .iter()
            .map(|(handle, import)| (*handle, import))
            .collect::<Vec<_>>();
        let report = self.connector_browser_for_imports(source_import, &source, &target_refs)?;
        serialize_connector_browser(&report)
    }

    pub fn connector_browser_subtree_json(&mut self, root: u64) -> Result<String, SceneHostError> {
        let root_node = self.resolve_node(root)?;
        let subtree = self.scene.subtree_nodes(root_node)?;
        let report = self.connector_browser_for_nodes(
            ConnectorBrowserScopeV1 {
                kind: "subtree".to_owned(),
                import: None,
                root: Some(root),
                selection: Vec::new(),
                target_imports: Vec::new(),
            },
            &subtree,
        )?;
        serialize_connector_browser(&report)
    }

    pub fn connector_browser_selection_json(
        &mut self,
        selection: &[u64],
    ) -> Result<String, SceneHostError> {
        let nodes = selection
            .iter()
            .copied()
            .map(|handle| self.resolve_node(handle))
            .collect::<Result<Vec<_>, SceneHostError>>()?;
        let report = self.connector_browser_for_nodes(
            ConnectorBrowserScopeV1 {
                kind: "selection".to_owned(),
                import: None,
                root: None,
                selection: selection.to_vec(),
                target_imports: Vec::new(),
            },
            &nodes,
        )?;
        serialize_connector_browser(&report)
    }

    fn connector_browser_for_imports(
        &mut self,
        source_handle: u64,
        source: &SceneImport,
        targets: &[(u64, &SceneImport)],
    ) -> Result<ConnectorBrowserReportV1, SceneHostError> {
        let mut connectors = self.import_connector_records(Some(source_handle), source)?;
        connectors.sort_by(|left, right| left.report.id.cmp(&right.report.id));
        let mut target_connectors = Vec::new();
        for (target_handle, target) in targets {
            target_connectors.extend(self.import_connector_records(Some(*target_handle), target)?);
        }
        target_connectors.sort_by(|left, right| left.report.id.cmp(&right.report.id));
        let candidates = self.connector_candidates(&connectors, &target_connectors);
        let visual_cues = connector_visual_cues(&candidates);
        Ok(connector_browser_report(
            ConnectorBrowserScopeV1 {
                kind: "import".to_owned(),
                import: Some(source_handle),
                root: None,
                selection: Vec::new(),
                target_imports: targets.iter().map(|(handle, _)| *handle).collect(),
            },
            connectors,
            target_connectors,
            candidates,
            visual_cues,
        ))
    }

    fn connector_browser_for_nodes(
        &mut self,
        scope: ConnectorBrowserScopeV1,
        nodes: &[NodeKey],
    ) -> Result<ConnectorBrowserReportV1, SceneHostError> {
        let selected = nodes.iter().copied().collect::<BTreeSet<_>>();
        let imports = self
            .import_handles
            .entries()
            .map(|(handle, import)| (handle, import.clone()))
            .collect::<Vec<_>>();
        let mut connectors = Vec::new();
        for (import_handle, import) in imports {
            for record in self.import_connector_records(Some(import_handle), &import)? {
                if selected.contains(&record.node) || selected.contains(&record.placement_node) {
                    connectors.push(record);
                }
            }
        }
        connectors.sort_by(|left, right| left.report.id.cmp(&right.report.id));
        Ok(connector_browser_report(
            scope,
            connectors,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ))
    }

    fn import_connector_records(
        &mut self,
        import_handle: Option<u64>,
        import: &SceneImport,
    ) -> Result<Vec<ConnectorRecord>, SceneHostError> {
        let connectors = import.connectors()?;
        let mut records = Vec::with_capacity(connectors.len());
        for connector in connectors {
            let node = connector.node();
            let placement_node = connector.placement_node();
            let node_handle = self.register_node(node);
            let placement_handle = self.register_node(placement_node);
            let name = connector.name().to_owned();
            let id = connector_id(import_handle, node_handle, &name);
            let frame = ConnectorFrame::from_import_connector(connector);
            records.push(ConnectorRecord {
                report: ConnectorBrowserConnectorV1 {
                    id,
                    name,
                    node: node_handle,
                    placement_node: placement_handle,
                    import: import_handle,
                    kind: connector.kind().map(str::to_owned),
                    allowed_mates: connector
                        .allowed_mates()
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                    tags: connector.tags().iter().cloned().collect(),
                    snap_tolerance: connector.snap_tolerance().map(round3),
                    clearance_hint: connector.clearance_hint().map(round3),
                    roll_policy: roll_policy_name(connector.roll_policy()).to_owned(),
                    polarity: connector.polarity().map(polarity_name).map(str::to_owned),
                    source_units: connector.source_units(),
                    source_coordinate_system: connector.source_coordinate_system(),
                    metadata: connector
                        .metadata()
                        .map(|metadata| metadata.value().clone()),
                },
                frame,
                node,
                placement_node,
            });
        }
        Ok(records)
    }

    fn connector_candidates(
        &self,
        sources: &[ConnectorRecord],
        targets: &[ConnectorRecord],
    ) -> Vec<ConnectorBrowserCandidateV1> {
        let mut candidates = Vec::new();
        for source in sources {
            for target in targets {
                candidates.push(self.connector_candidate(source, target));
            }
        }
        candidates.sort_by(|left, right| {
            left.source_id
                .cmp(&right.source_id)
                .then_with(|| left.target_id.cmp(&right.target_id))
        });
        candidates
    }

    fn connector_candidate(
        &self,
        source: &ConnectorRecord,
        target: &ConnectorRecord,
    ) -> ConnectorBrowserCandidateV1 {
        let mut invalid_reasons = metadata_invalid_reasons(&source.frame, &target.frame);
        let preview = self.scene.preview_connector_magnet(
            source.frame.clone(),
            target.frame.clone(),
            ConnectOptions::default(),
        );
        match preview {
            Ok(preview) if invalid_reasons.is_empty() => {
                let visual_cue = preview.visual_cue().css_class().to_owned();
                let line = preview.connection_line();
                ConnectorBrowserCandidateV1 {
                    source_id: source.report.id.clone(),
                    source_name: source.report.name.clone(),
                    target_id: target.report.id.clone(),
                    target_name: target.report.name.clone(),
                    compatible: true,
                    snap_ready: preview.is_snap_ready(),
                    distance: Some(round3(preview.distance())),
                    tolerance: Some(round3(preview.tolerance())),
                    visual_cue: Some(visual_cue),
                    ghost_transform: Some(round_transform(preview.ghost_transform())),
                    connection_line: Some(ConnectorLineV1 {
                        start: round_vec3(line.start()),
                        end: round_vec3(line.end()),
                    }),
                    invalid_reasons: if preview.is_snap_ready() {
                        Vec::new()
                    } else {
                        vec!["out_of_snap_range".to_owned()]
                    },
                    message: if preview.is_snap_ready() {
                        "snap ready".to_owned()
                    } else {
                        "compatible but outside snap tolerance".to_owned()
                    },
                }
            }
            Ok(_) => ConnectorBrowserCandidateV1 {
                source_id: source.report.id.clone(),
                source_name: source.report.name.clone(),
                target_id: target.report.id.clone(),
                target_name: target.report.name.clone(),
                compatible: false,
                snap_ready: false,
                distance: None,
                tolerance: None,
                visual_cue: None,
                ghost_transform: None,
                connection_line: None,
                message: metadata_message(&invalid_reasons),
                invalid_reasons,
            },
            Err(error) => {
                invalid_reasons.push(connection_error_code(&error).to_owned());
                invalid_reasons.sort();
                invalid_reasons.dedup();
                ConnectorBrowserCandidateV1 {
                    source_id: source.report.id.clone(),
                    source_name: source.report.name.clone(),
                    target_id: target.report.id.clone(),
                    target_name: target.report.name.clone(),
                    compatible: false,
                    snap_ready: false,
                    distance: None,
                    tolerance: None,
                    visual_cue: None,
                    ghost_transform: None,
                    connection_line: None,
                    message: error.to_string(),
                    invalid_reasons,
                }
            }
        }
    }
}

fn serialize_connector_browser(
    report: &ConnectorBrowserReportV1,
) -> Result<String, SceneHostError> {
    serde_json::to_string(report).map_err(|error| {
        SceneHostError::new(
            SceneHostErrorCode::Inspect,
            format!("connector browser serialization failed: {error}"),
        )
    })
}

fn connector_browser_report(
    scope: ConnectorBrowserScopeV1,
    connectors: Vec<ConnectorRecord>,
    target_connectors: Vec<ConnectorRecord>,
    candidates: Vec<ConnectorBrowserCandidateV1>,
    visual_cues: Vec<ConnectorBrowserVisualCueV1>,
) -> ConnectorBrowserReportV1 {
    let summary = ConnectorBrowserSummaryV1 {
        connector_count: connectors.len(),
        target_connector_count: target_connectors.len(),
        candidate_count: candidates.len(),
        compatible_count: candidates
            .iter()
            .filter(|candidate| candidate.compatible)
            .count(),
        snap_ready_count: candidates
            .iter()
            .filter(|candidate| candidate.snap_ready)
            .count(),
        invalid_count: candidates
            .iter()
            .filter(|candidate| !candidate.compatible)
            .count(),
    };
    ConnectorBrowserReportV1 {
        schema: CONNECTOR_BROWSER_SCHEMA_V1.to_owned(),
        scope,
        summary,
        connectors: connectors.into_iter().map(|record| record.report).collect(),
        target_connectors: target_connectors
            .into_iter()
            .map(|record| record.report)
            .collect(),
        candidates,
        visual_cues,
    }
}

fn connector_visual_cues(
    candidates: &[ConnectorBrowserCandidateV1],
) -> Vec<ConnectorBrowserVisualCueV1> {
    candidates
        .iter()
        .filter(|candidate| candidate.compatible && candidate.ghost_transform.is_some())
        .flat_map(|candidate| {
            let id = format!("{}->{}", candidate.source_id, candidate.target_id);
            let style = candidate
                .visual_cue
                .clone()
                .unwrap_or_else(|| ConnectionMagnetVisualCue::OutOfRange.css_class().to_owned());
            [
                ConnectorBrowserVisualCueV1 {
                    candidate: id.clone(),
                    kind: "ghost_transform".to_owned(),
                    style: style.clone(),
                },
                ConnectorBrowserVisualCueV1 {
                    candidate: id,
                    kind: "connection_line".to_owned(),
                    style,
                },
            ]
        })
        .collect()
}

fn connector_id(import: Option<u64>, node: u64, name: &str) -> String {
    match import {
        Some(import) => format!("import:{import}:connector:{name}:node:{node}"),
        None => format!("node:{node}:connector:{name}"),
    }
}

fn roll_policy_name(policy: ConnectorRollPolicy) -> &'static str {
    match policy {
        ConnectorRollPolicy::Preserve => "preserve",
        ConnectorRollPolicy::ChooseNearest => "choose_nearest",
        ConnectorRollPolicy::ExplicitAngle => "explicit_angle",
    }
}

fn polarity_name(polarity: ConnectorPolarity) -> &'static str {
    match polarity {
        ConnectorPolarity::Plug => "plug",
        ConnectorPolarity::Socket => "socket",
        ConnectorPolarity::Neutral => "neutral",
    }
}

fn metadata_invalid_reasons(source: &ConnectorFrame, target: &ConnectorFrame) -> Vec<String> {
    let mut reasons = Vec::new();
    if polarity_mismatch(source.polarity(), target.polarity()) {
        reasons.push("polarity_mismatch".to_owned());
    }
    if !source.tags().is_empty()
        && !target.tags().is_empty()
        && source.tags().is_disjoint(target.tags())
    {
        reasons.push("tag_mismatch".to_owned());
    }
    reasons
}

fn polarity_mismatch(source: Option<ConnectorPolarity>, target: Option<ConnectorPolarity>) -> bool {
    matches!(
        (source, target),
        (Some(ConnectorPolarity::Plug), Some(ConnectorPolarity::Plug))
            | (
                Some(ConnectorPolarity::Socket),
                Some(ConnectorPolarity::Socket)
            )
    )
}

fn metadata_message(reasons: &[String]) -> String {
    if reasons.is_empty() {
        "metadata compatible".to_owned()
    } else {
        format!("metadata rejected mate: {}", reasons.join(", "))
    }
}

fn connection_error_code(error: &ConnectionError) -> &'static str {
    match error {
        ConnectionError::IncompatibleConnector { .. } => "incompatible_kind",
        ConnectionError::UnitMismatch { .. } => "unit_mismatch",
        ConnectionError::CoordinateSystemMismatch { .. } => "coordinate_system_mismatch",
        ConnectionError::HandednessMismatch { .. } => "handedness_mismatch",
        ConnectionError::DegenerateConnectorFrame { .. } => "degenerate_connector_frame",
        ConnectionError::NonUniformScaleConnectionRisk { .. } => "non_uniform_scale_risk",
        ConnectionError::FlippedConnection { .. } => "flipped_connection",
        ConnectionError::ConnectionWouldMoveLockedNode { .. } => "locked_node",
        ConnectionError::ConnectionWouldCreateCycle { .. } => "connection_cycle",
        ConnectionError::ConnectorHostNotPrepared { .. } => "connector_host_not_prepared",
        ConnectionError::NodeNotFound(_)
        | ConnectionError::MissingAnchor { .. }
        | ConnectionError::MissingAnchorName { .. }
        | ConnectionError::AmbiguousAnchor { .. }
        | ConnectionError::StaleAnchorHandle { .. }
        | ConnectionError::MissingConnector { .. }
        | ConnectionError::MissingConnectorName { .. }
        | ConnectionError::AmbiguousConnector { .. }
        | ConnectionError::AmbiguousImportConnector { .. }
        | ConnectionError::StaleConnectorHandle { .. } => "connector_lookup",
    }
}

fn round_transform(transform: Transform) -> ConnectorTransformV1 {
    ConnectorTransformV1 {
        translation: round_vec3(transform.translation),
        rotation: [
            round3(transform.rotation.x),
            round3(transform.rotation.y),
            round3(transform.rotation.z),
            round3(transform.rotation.w),
        ],
        scale: round_vec3(transform.scale),
    }
}

fn round_vec3(value: crate::Vec3) -> [f64; 3] {
    [round3(value.x), round3(value.y), round3(value.z)]
}

fn round3(value: f32) -> f64 {
    let value = f64::from(value);
    (value * 1000.0).round() / 1000.0
}
