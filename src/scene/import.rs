use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, Weak};

use serde::{Deserialize, Serialize};

use self::bounds::union_optional;
use self::diagnostic_overlays::diagnostic_overlay;
use self::instancing::instanced_bounds;
use self::types::{ImportBuild, ImportedNode, PendingSkinBinding, mesh_node_kind};
use self::units::convert_marker_units;
pub(super) use self::variants::MeshVariantRecord;
use super::transforms::compose_transform;
use super::{
    ConnectorMetadata, ConnectorPolarity, ConnectorRollPolicy, NodeKey, NodeKind, Scene, Transform,
};
use crate::animation::AnimationClip;
use crate::assets::SceneAsset;
use crate::diagnostics::{ImportDiagnosticOverlay, ImportDiagnosticOverlayKind, InstantiateError};

mod accessors;
mod animation_bindings;
mod bounds;
mod diagnostic_overlays;
mod handedness;
mod instancing;
mod instantiate;
mod load;
mod lookups;
mod options;
mod prevalidation;
mod skin_bindings;
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
    placement_transform: Transform,
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
    placement_transform: Transform,
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
        let nodes = scene_asset.nodes();
        let mut child_indices = BTreeSet::new();
        for node in nodes {
            child_indices.extend(node.children().iter().copied());
        }

        let source_roots = (0..nodes.len())
            .filter(|index| !child_indices.contains(index))
            .collect::<Vec<_>>();
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
            let node = self.instantiate_scene_asset_node(
                source_index,
                source_parent,
                None,
                unit_root,
                Transform::IDENTITY,
                &mut build,
            )?;
            if unit_root.is_none() {
                import.roots.push(node);
            }
        }
        if let Some(unit_root) = unit_root {
            import.roots.push(unit_root);
        }
        self.resolve_import_skin_bindings(
            scene_asset,
            &import.records,
            pending_skin_bindings.as_slice(),
        )?;
        import.clips =
            animation_bindings::rebind_import_clips(scene_asset, &import.records, options)?;
        Ok(import)
    }

    fn instantiate_scene_asset_node(
        &mut self,
        source_index: usize,
        parent: NodeKey,
        imported_parent: Option<NodeKey>,
        import_root: Option<NodeKey>,
        root_from_parent: Transform,
        build: &mut ImportBuild<'_>,
    ) -> Result<NodeKey, InstantiateError> {
        let source_node = build.scene_asset.nodes().get(source_index).ok_or(
            InstantiateError::InvalidChildIndex {
                parent: source_index,
                child: source_index,
            },
        )?;
        let transform = build.options.convert_transform(source_node.transform());
        let meshes = source_node.meshes();
        let skin = source_node.skin();
        let mesh_bounds = meshes.iter().fold(None, |bounds, mesh| {
            Some(union_optional(bounds, mesh.bounds()))
        });
        let instance_transforms = source_node.instance_transforms();
        let bounds = match (mesh_bounds, instance_transforms.is_empty()) {
            (Some(bounds), false) => {
                Some(instanced_bounds(bounds, instance_transforms, build.options))
            }
            (bounds, true) => bounds,
            (None, false) => None,
        };
        let node = match (meshes, source_node.light()) {
            ([mesh], _) if !instance_transforms.is_empty() => Ok(self
                .instantiate_single_import_instance_set(
                    parent,
                    mesh,
                    transform,
                    instance_transforms,
                    build.options,
                )),
            ([_, _, ..], _) if !instance_transforms.is_empty() => Ok(self
                .instantiate_multi_import_instance_sets(
                    parent,
                    meshes,
                    transform,
                    instance_transforms,
                    build.options,
                )),
            ([mesh], _) => {
                let node = self.insert_node(parent, mesh_node_kind(mesh), transform);
                if let Ok(node) = node {
                    self.set_initial_morph_weights(node, mesh.morph_weights());
                    if let Some(skin) = skin {
                        build.pending_skin_bindings.push(PendingSkinBinding {
                            node,
                            source_node: source_index,
                            skin,
                        });
                    }
                    if !mesh.material_variant_bindings().is_empty() {
                        build.variant_records.push(MeshVariantRecord {
                            node,
                            default_material: mesh.material(),
                            bindings: mesh.material_variant_bindings().to_vec(),
                        });
                    }
                }
                node
            }
            ([_, _, ..], _) => {
                let node = self.insert_node(parent, NodeKind::Empty, transform);
                if let Ok(parent) = node {
                    for mesh in meshes {
                        let child = self
                            .insert_node(parent, mesh_node_kind(mesh), Transform::IDENTITY)
                            .expect("multi-primitive parent was inserted by this scene");
                        self.node_bounds.insert(child, mesh.bounds());
                        self.set_initial_morph_weights(child, mesh.morph_weights());
                        if let Some(skin) = skin {
                            build.pending_skin_bindings.push(PendingSkinBinding {
                                node: child,
                                source_node: source_index,
                                skin,
                            });
                        }
                        if !mesh.material_variant_bindings().is_empty() {
                            build.variant_records.push(MeshVariantRecord {
                                node: child,
                                default_material: mesh.material(),
                                bindings: mesh.material_variant_bindings().to_vec(),
                            });
                        }
                    }
                }
                node
            }
            ([], Some(light)) => match light.light() {
                super::Light::Directional(light) => self
                    .directional_light(light)
                    .parent(parent)
                    .transform(transform)
                    .add(),
                super::Light::Point(light) => self
                    .point_light(light)
                    .parent(parent)
                    .transform(transform)
                    .add(),
                super::Light::Spot(light) => self
                    .spot_light(light)
                    .parent(parent)
                    .transform(transform)
                    .add(),
                super::Light::Area(light) => self
                    .area_light(light)
                    .parent(parent)
                    .transform(transform)
                    .add(),
            },
            ([], None) => self.insert_node(parent, NodeKind::Empty, transform),
        }
        .expect("import parent was inserted by this scene");
        let morph_nodes = match meshes {
            [] => Vec::new(),
            [_] => vec![node],
            [_, _, ..] => self
                .node(node)
                .map(|parent| parent.children().to_vec())
                .unwrap_or_default(),
        };
        build.records.push(ImportedNode {
            source_index,
            node,
            morph_nodes,
            parent: imported_parent,
            name: source_node.name().map(str::to_string),
            bounds,
        });
        let placement_node = import_root.unwrap_or(node);
        let root_from_node = match import_root {
            Some(_) => compose_transform(root_from_parent, transform),
            None => Transform::IDENTITY,
        };
        let label = source_node.name().map(str::to_string);
        let overlay_options = build.options;
        build.diagnostic_overlays.push(diagnostic_overlay(
            overlay_options,
            ImportDiagnosticOverlayKind::Origin,
            node,
            transform,
            None,
            label.clone(),
        ));
        build.diagnostic_overlays.push(diagnostic_overlay(
            overlay_options,
            ImportDiagnosticOverlayKind::Axes,
            node,
            transform,
            None,
            label.clone(),
        ));
        if let Some(bounds) = bounds {
            self.node_bounds.insert(node, bounds);
            build.diagnostic_overlays.push(diagnostic_overlay(
                overlay_options,
                ImportDiagnosticOverlayKind::Bounds,
                node,
                Transform::IDENTITY,
                Some(bounds),
                label.clone(),
            ));
        }
        let mut anchor_names = BTreeSet::new();
        for anchor in source_node.anchors() {
            if let Some(reason) = anchor.invalid_reason() {
                return Err(InstantiateError::InvalidAnchorExtras {
                    node: source_node.name().unwrap_or("<unnamed>").to_string(),
                    reason: reason.to_string(),
                });
            }
            if !anchor_names.insert(anchor.name()) {
                return Err(InstantiateError::InvalidAnchorExtras {
                    node: source_node.name().unwrap_or("<unnamed>").to_string(),
                    reason: format!("duplicate anchor '{}'", anchor.name()),
                });
            }
            let anchor_units = anchor
                .source_units()
                .unwrap_or(build.options.source_units());
            let anchor_transform = convert_marker_units(
                anchor.transform(),
                anchor_units,
                build.options.source_units(),
            );
            let anchor_connection_transform = build
                .options
                .source_coordinate_system()
                .convert_connector_transform(anchor_transform);
            build.anchors.push(ImportAnchor {
                name: anchor.name().to_string(),
                node,
                placement_node,
                transform: anchor_transform,
                placement_transform: compose_transform(root_from_node, anchor_connection_transform),
                tags: anchor.tags().clone(),
                label: anchor.label().map(str::to_string),
                source_units: anchor_units,
                source_coordinate_system: build.options.source_coordinate_system(),
                live: Arc::clone(build.import_live),
            });
            build.diagnostic_overlays.push(diagnostic_overlay(
                overlay_options,
                ImportDiagnosticOverlayKind::Anchor,
                node,
                anchor_transform,
                None,
                Some(anchor.name().to_string()),
            ));
            if anchor.name() == "pivot" {
                build.diagnostic_overlays.push(diagnostic_overlay(
                    overlay_options,
                    ImportDiagnosticOverlayKind::Pivot,
                    node,
                    anchor_transform,
                    None,
                    Some(anchor.name().to_string()),
                ));
            }
        }
        let mut connector_names = BTreeSet::new();
        for connector in source_node.connectors() {
            if let Some(reason) = connector.invalid_reason() {
                return Err(InstantiateError::InvalidConnectorExtras {
                    node: source_node.name().unwrap_or("<unnamed>").to_string(),
                    reason: reason.to_string(),
                });
            }
            if !connector_names.insert(connector.name()) {
                return Err(InstantiateError::InvalidConnectorExtras {
                    node: source_node.name().unwrap_or("<unnamed>").to_string(),
                    reason: format!("duplicate connector '{}'", connector.name()),
                });
            }
            let connector_transform = connector.transform();
            let connector_connection_transform = build
                .options
                .source_coordinate_system()
                .convert_connector_transform(connector_transform);
            build.connectors.push(ImportConnector {
                name: connector.name().to_string(),
                kind: connector.kind().map(str::to_string),
                allowed_mates: connector
                    .allowed_mates()
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                tags: connector.tags().clone(),
                snap_tolerance: connector.snap_tolerance(),
                clearance_hint: connector.clearance_hint(),
                roll_policy: connector.roll_policy(),
                polarity: connector.polarity(),
                metadata: connector.metadata().cloned(),
                node,
                placement_node,
                transform: connector_transform,
                placement_transform: compose_transform(
                    root_from_node,
                    connector_connection_transform,
                ),
                source_units: build.options.source_units(),
                source_coordinate_system: build.options.source_coordinate_system(),
                live: Arc::clone(build.import_live),
            });
            build.diagnostic_overlays.push(diagnostic_overlay(
                overlay_options,
                ImportDiagnosticOverlayKind::Connector,
                node,
                connector_transform,
                None,
                Some(connector.name().to_string()),
            ));
        }
        for child in source_node.children() {
            if build.scene_asset.nodes().get(*child).is_none() {
                return Err(InstantiateError::InvalidChildIndex {
                    parent: source_index,
                    child: *child,
                });
            }
            self.instantiate_scene_asset_node(
                *child,
                node,
                Some(node),
                Some(placement_node),
                root_from_node,
                build,
            )?;
        }
        Ok(node)
    }
}
