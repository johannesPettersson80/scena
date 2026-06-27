use crate::geometry::Aabb;
use crate::scene::{NodeKey, SourceCoordinateSystem, SourceUnits, Transform};

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDiagnosticOverlay {
    kind: ImportDiagnosticOverlayKind,
    node: NodeKey,
    transform: Transform,
    bounds: Option<Aabb>,
    label: Option<String>,
    source_units: SourceUnits,
    source_coordinate_system: SourceCoordinateSystem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportDiagnosticOverlayKind {
    Origin,
    Axes,
    Bounds,
    Anchor,
    Connector,
    Pivot,
}

impl ImportDiagnosticOverlay {
    pub fn new(
        kind: ImportDiagnosticOverlayKind,
        node: NodeKey,
        transform: Transform,
        bounds: Option<Aabb>,
        label: Option<String>,
    ) -> Self {
        Self {
            kind,
            node,
            transform,
            bounds,
            label,
            source_units: SourceUnits::Meters,
            source_coordinate_system: SourceCoordinateSystem::GltfYUpRightHanded,
        }
    }

    pub const fn with_source_metadata(
        mut self,
        units: SourceUnits,
        coordinate_system: SourceCoordinateSystem,
    ) -> Self {
        self.source_units = units;
        self.source_coordinate_system = coordinate_system;
        self
    }

    pub const fn kind(&self) -> ImportDiagnosticOverlayKind {
        self.kind
    }

    pub const fn node(&self) -> NodeKey {
        self.node
    }

    pub const fn transform(&self) -> Transform {
        self.transform
    }

    pub const fn bounds(&self) -> Option<Aabb> {
        self.bounds
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub const fn source_units(&self) -> SourceUnits {
        self.source_units
    }

    pub const fn source_coordinate_system(&self) -> SourceCoordinateSystem {
        self.source_coordinate_system
    }
}
