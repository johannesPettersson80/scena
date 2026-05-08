use crate::diagnostics::{ImportDiagnosticOverlay, ImportDiagnosticOverlayKind};
use crate::geometry::Aabb;
use crate::scene::{ImportOptions, NodeKey, Transform};

pub(super) fn diagnostic_overlay(
    options: ImportOptions,
    kind: ImportDiagnosticOverlayKind,
    node: NodeKey,
    transform: Transform,
    bounds: Option<Aabb>,
    label: Option<String>,
) -> ImportDiagnosticOverlay {
    ImportDiagnosticOverlay::new(kind, node, transform, bounds, label)
        .with_source_metadata(options.source_units(), options.source_coordinate_system())
}
