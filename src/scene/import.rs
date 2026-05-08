use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use self::bounds::union_optional;
use self::diagnostic_overlays::diagnostic_overlay;
use self::handedness::reject_unproven_left_handed_mesh_import;
use self::types::{ImportBuild, ImportedNode, PendingSkinBinding, mesh_node_kind};
use self::units::convert_marker_units;
use super::transforms::compose_transform;
use super::{
    ConnectorMetadata, ConnectorPolarity, ConnectorRollPolicy, NodeKey, NodeKind, Scene,
    SceneSkinBinding, Transform,
};
use crate::animation::{AnimationClip, AnimationClipKey};
use crate::assets::{AssetFetcher, AssetPath, Assets, SceneAsset};
use crate::diagnostics::{
    ImportDiagnosticOverlay, ImportDiagnosticOverlayKind, ImportError, InstantiateError,
};

mod accessors;
mod bounds;
mod diagnostic_overlays;
mod handedness;
mod lookups;
mod options;
mod types;
mod units;

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
    live: Arc<AtomicBool>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceUnits {
    #[default]
    Meters,
    Centimeters,
    Millimeters,
    Inches,
    Feet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceCoordinateSystem {
    #[default]
    GltfYUpRightHanded,
    YUpLeftHanded,
    ZUpRightHanded,
    ZUpLeftHanded,
}

impl Scene {
    pub fn instantiate(
        &mut self,
        scene_asset: &SceneAsset,
    ) -> Result<SceneImport, InstantiateError> {
        self.instantiate_with(scene_asset, ImportOptions::gltf_default())
    }

    pub fn instantiate_with(
        &mut self,
        scene_asset: &SceneAsset,
        options: ImportOptions,
    ) -> Result<SceneImport, InstantiateError> {
        reject_unproven_left_handed_mesh_import(scene_asset, options)?;
        let nodes = scene_asset.nodes();
        let mut child_indices = BTreeSet::new();
        for node in nodes {
            child_indices.extend(node.children().iter().copied());
        }

        let roots = (0..nodes.len())
            .filter(|index| !child_indices.contains(index))
            .collect::<Vec<_>>();
        let mut import = SceneImport {
            roots: Vec::new(),
            records: Vec::new(),
            anchors: Vec::new(),
            connectors: Vec::new(),
            clips: Vec::new(),
            diagnostic_overlays: Vec::new(),
            source_units: options.source_units(),
            source_coordinate_system: options.source_coordinate_system(),
            live: Arc::new(AtomicBool::new(true)),
        };
        let mut pending_skin_bindings = Vec::new();
        for source_index in roots {
            let mut build = ImportBuild {
                scene_asset,
                options,
                import_live: &import.live,
                records: &mut import.records,
                anchors: &mut import.anchors,
                connectors: &mut import.connectors,
                diagnostic_overlays: &mut import.diagnostic_overlays,
                pending_skin_bindings: &mut pending_skin_bindings,
            };
            let node = self.instantiate_scene_asset_node(
                source_index,
                self.root,
                None,
                None,
                Transform::IDENTITY,
                &mut build,
            )?;
            import.roots.push(node);
        }
        self.resolve_import_skin_bindings(
            scene_asset,
            &import.records,
            pending_skin_bindings.as_slice(),
        )?;
        import.clips = scene_asset
            .clips()
            .iter()
            .map(|clip| {
                let rebased = clip.clip().rebind(
                    AnimationClipKey::fresh(),
                    |source_index| {
                        import
                            .records
                            .iter()
                            .find(|record| record.source_index == source_index)
                            .map(|record| record.node)
                    },
                    |target, value| options.convert_animation_vec3(target, value),
                );
                ImportClip { clip: rebased }
            })
            .collect();
        Ok(import)
    }

    pub async fn import<F: AssetFetcher>(
        &mut self,
        assets: &Assets<F>,
        path: impl Into<AssetPath>,
    ) -> Result<SceneImport, ImportError> {
        self.import_with(assets, path, ImportOptions::gltf_default())
            .await
    }

    pub async fn import_with<F: AssetFetcher>(
        &mut self,
        assets: &Assets<F>,
        path: impl Into<AssetPath>,
        options: ImportOptions,
    ) -> Result<SceneImport, ImportError> {
        let scene_asset = assets.load_scene(path).await?;
        self.instantiate_with(&scene_asset, options)
            .map_err(Into::into)
    }

    pub fn replace_import(
        &mut self,
        import: &SceneImport,
        scene_asset: &SceneAsset,
    ) -> Result<SceneImport, InstantiateError> {
        let options = ImportOptions::gltf_default()
            .with_source_units(import.source_units)
            .with_source_coordinate_system(import.source_coordinate_system);
        import.mark_stale();
        self.instantiate_with(scene_asset, options)
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
        let bounds = meshes.iter().fold(None, |bounds, mesh| {
            Some(union_optional(bounds, mesh.bounds()))
        });
        let node = match (meshes, source_node.light()) {
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
            },
            ([], None) => self.insert_node(parent, NodeKind::Empty, transform),
        }
        .expect("import parent was inserted by this scene");
        build.records.push(ImportedNode {
            source_index,
            node,
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
                return Err(InstantiateError::InvalidAnchorExtras {
                    node: source_node.name().unwrap_or("<unnamed>").to_string(),
                    reason: reason.to_string(),
                });
            }
            if !connector_names.insert(connector.name()) {
                return Err(InstantiateError::InvalidAnchorExtras {
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

    fn resolve_import_skin_bindings(
        &mut self,
        scene_asset: &SceneAsset,
        records: &[ImportedNode],
        pending: &[PendingSkinBinding],
    ) -> Result<(), InstantiateError> {
        for pending in pending {
            let skin = scene_asset.skins().get(pending.skin).ok_or(
                InstantiateError::InvalidSkinIndex {
                    node: pending.source_node,
                    skin: pending.skin,
                },
            )?;
            let joints = skin
                .joints()
                .iter()
                .map(|source_joint| {
                    records
                        .iter()
                        .find(|record| record.source_index == *source_joint)
                        .map(|record| record.node)
                        .ok_or(InstantiateError::InvalidSkinJointIndex {
                            skin: pending.skin,
                            joint: *source_joint,
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            self.set_initial_skin_binding(
                pending.node,
                SceneSkinBinding::new(joints, skin.inverse_bind_matrices().to_vec()),
            );
        }
        Ok(())
    }
}
