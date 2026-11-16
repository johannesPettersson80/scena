//! Instantiation of glTF asset nodes into scene nodes.

use std::collections::BTreeSet;
use std::sync::Arc;

use super::bounds::union_optional;
use super::diagnostic_overlays::diagnostic_overlay;
use super::instancing::instanced_bounds;
use super::types::{ImportBuild, ImportedNode, PendingSkinBinding, mesh_node_kind};
use super::units::convert_marker_units;
use super::{ImportAnchor, ImportConnector, MeshVariantRecord};
use crate::diagnostics::{ImportDiagnosticOverlayKind, InstantiateError};
use crate::scene::{Light, NodeKey, NodeKind, Scene, Transform, WorldTransform};

/// One asset node waiting to be instantiated, together with the scene context
/// its parent established for it.
struct PendingNode {
    source_index: usize,
    source_parent: Option<usize>,
    parent: NodeKey,
    imported_parent: Option<NodeKey>,
    import_root: Option<NodeKey>,
    root_from_parent: WorldTransform,
}

/// What instantiating one asset node hands down to its children.
struct InstantiatedNode {
    node: NodeKey,
    placement_node: NodeKey,
    root_from_node: WorldTransform,
}

impl Scene {
    /// Instantiates the asset subtree rooted at `source_index` under `parent`
    /// and returns the scene node created for that root.
    ///
    /// The walk keeps its own stack of pending nodes instead of recursing, so
    /// hierarchy depth never becomes call-stack depth. Children are pushed in
    /// reverse, which keeps node creation in depth-first document order.
    pub(super) fn instantiate_scene_asset_subtree(
        &mut self,
        source_index: usize,
        parent: NodeKey,
        import_root: Option<NodeKey>,
        build: &mut ImportBuild<'_>,
    ) -> Result<NodeKey, InstantiateError> {
        let scene_asset = build.scene_asset;
        let mut subtree_root = None;
        let mut pending = vec![PendingNode {
            source_index,
            source_parent: None,
            parent,
            imported_parent: None,
            import_root,
            root_from_parent: WorldTransform::IDENTITY,
        }];
        while let Some(next) = pending.pop() {
            let instantiated = self.instantiate_scene_asset_node(&next, build)?;
            subtree_root.get_or_insert(instantiated.node);
            let children = scene_asset
                .nodes()
                .get(next.source_index)
                .map(|source_node| source_node.children())
                .unwrap_or_default();
            pending.extend(children.iter().rev().map(|child| PendingNode {
                source_index: *child,
                source_parent: Some(next.source_index),
                parent: instantiated.node,
                imported_parent: Some(instantiated.node),
                import_root: Some(instantiated.placement_node),
                root_from_parent: instantiated.root_from_node,
            }));
        }
        Ok(subtree_root.expect("the subtree root is always instantiated first"))
    }

    fn instantiate_scene_asset_node(
        &mut self,
        pending: &PendingNode,
        build: &mut ImportBuild<'_>,
    ) -> Result<InstantiatedNode, InstantiateError> {
        let source_index = pending.source_index;
        let parent = pending.parent;
        let source_node = build.scene_asset.nodes().get(source_index).ok_or(
            InstantiateError::InvalidChildIndex {
                parent: pending.source_parent.unwrap_or(source_index),
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
                    self.record_imported_pick_bounds(node, mesh);
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
                        self.record_imported_pick_bounds(child, mesh);
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
                Light::Directional(light) => self
                    .directional_light(light)
                    .parent(parent)
                    .transform(transform)
                    .add(),
                Light::Point(light) => self
                    .point_light(light)
                    .parent(parent)
                    .transform(transform)
                    .add(),
                Light::Spot(light) => self
                    .spot_light(light)
                    .parent(parent)
                    .transform(transform)
                    .add(),
                Light::Area(light) => self
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
            parent: pending.imported_parent,
            name: source_node.name().map(str::to_string),
            bounds,
        });
        let placement_node = pending.import_root.unwrap_or(node);
        // Exact placement of this node relative to the import's placement
        // root, so anchors and connectors below a non-uniformly scaled node
        // report where they are drawn.
        let root_from_node = match pending.import_root {
            Some(_) => pending.root_from_parent.child(transform),
            None => WorldTransform::IDENTITY,
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
                placement_transform: root_from_node.child(anchor_connection_transform).trs(),
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
                placement_transform: root_from_node.child(connector_connection_transform).trs(),
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
        Ok(InstantiatedNode {
            node,
            placement_node,
            root_from_node,
        })
    }
}
