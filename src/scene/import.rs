use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, Weak};

use serde::{Deserialize, Serialize};

use self::types::{ImportBuild, ImportedNode};
pub(super) use self::variants::MeshVariantRecord;
use super::{
    ConnectorMetadata, ConnectorPolarity, ConnectorRollPolicy, NodeKey, NodeKind, Scene, Transform,
    WorldTransform,
};
use crate::animation::AnimationClip;
use crate::assets::SceneAsset;
use crate::diagnostics::{ImportDiagnosticOverlay, InstantiateError};

mod accessors;
mod animation_bindings;
mod bounds;
mod diagnostic_overlays;
mod handedness;
mod instancing;
mod instantiate;
mod load;
mod lookups;
mod nodes;
mod options;
mod prevalidation;
mod skin_bindings;
mod source_node_index;
#[cfg(test)]
mod transaction_tests;
mod types;
mod units;
mod variants;

#[derive(Debug, Clone)]
pub struct SceneImport {
    roots: Vec<NodeKey>,
    records: Vec<ImportedNode>,
    anchors: Vec<ImportAnchor>,
    connectors: Vec<ImportConnector>,
    clips: Vec<ImportClip>,
    diagnostic_overlays: Vec<ImportDiagnosticOverlay>,
    source_units: SourceUnits,
    source_coordinate_system: SourceCoordinateSystem,
    scene_identity: Weak<()>,
    live: Arc<AtomicBool>,
    // Phase 2B step 3: KHR_materials_variants runtime state.
    pub(super) material_variants: Vec<String>,
    pub(super) active_variant: Arc<Mutex<Option<u32>>>,
    pub(super) variant_records: Vec<MeshVariantRecord>,
}

#[derive(Debug, Clone)]
pub struct ImportAnchor {
    name: String,
    node: NodeKey,
    placement_node: NodeKey,
    transform: Transform,
    /// Exact placement of the anchor frame relative to `placement_node`.
    placement: WorldTransform,
    tags: BTreeSet<String>,
    label: Option<String>,
    source_units: SourceUnits,
    source_coordinate_system: SourceCoordinateSystem,
    live: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
pub struct ImportConnector {
    name: String,
    kind: Option<String>,
    allowed_mates: Vec<String>,
    tags: BTreeSet<String>,
    snap_tolerance: Option<f32>,
    clearance_hint: Option<f32>,
    roll_policy: ConnectorRollPolicy,
    polarity: Option<ConnectorPolarity>,
    metadata: Option<ConnectorMetadata>,
    node: NodeKey,
    placement_node: NodeKey,
    transform: Transform,
    /// Exact placement of the connector frame relative to `placement_node`.
    placement: WorldTransform,
    source_units: SourceUnits,
    source_coordinate_system: SourceCoordinateSystem,
    live: Arc<AtomicBool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportAnchorDebugMetadata {
    name: String,
    node: NodeKey,
    transform: Transform,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportClip {
    clip: AnimationClip,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportPivot {
    name: Option<String>,
    node: NodeKey,
    transform: Transform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImportOptions {
    source_units: SourceUnits,
    source_coordinate_system: SourceCoordinateSystem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceUnits {
    #[default]
    Meters,
    Centimeters,
    Millimeters,
    Inches,
    Feet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCoordinateSystem {
    #[default]
    GltfYUpRightHanded,
    YUpLeftHanded,
    ZUpRightHanded,
    ZUpLeftHanded,
}

impl Scene {
    fn instantiate_with_parent_validated(
        &mut self,
        parent: NodeKey,
        scene_asset: &SceneAsset,
        options: ImportOptions,
    ) -> Result<SceneImport, InstantiateError> {
        let source_roots = scene_asset.root_indices();
        let unit_root = (!source_roots.is_empty())
            .then(|| options.unit_root_transform())
            .flatten()
            .map(|transform| {
                self.insert_node(parent, NodeKind::Empty, transform)
                    .expect("validated import parent accepts the unit root")
            });
        let source_parent = unit_root.unwrap_or(parent);
        let mut import = SceneImport {
            roots: Vec::new(),
            records: Vec::new(),
            anchors: Vec::new(),
            connectors: Vec::new(),
            clips: Vec::new(),
            diagnostic_overlays: Vec::new(),
            source_units: options.source_units(),
            source_coordinate_system: options.source_coordinate_system(),
            scene_identity: Arc::downgrade(&self.identity),
            live: Arc::new(AtomicBool::new(true)),
            material_variants: scene_asset.material_variants().to_vec(),
            active_variant: Arc::new(Mutex::new(None)),
            variant_records: Vec::new(),
        };
        let mut pending_skin_bindings = Vec::new();
        for source_index in source_roots {
            let mut build = ImportBuild {
                scene_asset,
                options,
                import_live: &import.live,
                records: &mut import.records,
                anchors: &mut import.anchors,
                connectors: &mut import.connectors,
                diagnostic_overlays: &mut import.diagnostic_overlays,
                pending_skin_bindings: &mut pending_skin_bindings,
                variant_records: &mut import.variant_records,
            };
            let node = self.instantiate_scene_asset_subtree(
                source_index,
                source_parent,
                unit_root,
                &mut build,
            )?;
            if unit_root.is_none() {
                import.roots.push(node);
            }
        }
        if let Some(unit_root) = unit_root {
            import.roots.push(unit_root);
        }
        let source_nodes = source_node_index::SourceNodeIndex::new(&import.records);
        self.resolve_import_skin_bindings(
            scene_asset,
            &source_nodes,
            pending_skin_bindings.as_slice(),
        )?;
        import.clips =
            animation_bindings::rebind_import_clips(scene_asset, &source_nodes, options)?;
        Ok(import)
    }
}
