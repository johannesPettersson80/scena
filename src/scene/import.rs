use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::animation::{AnimationClip, AnimationClipKey};
use crate::assets::{AssetFetcher, AssetPath, Assets, SceneAsset, SceneAssetMesh};
use crate::diagnostics::{
    ImportDiagnosticOverlay, ImportDiagnosticOverlayKind, ImportError, InstantiateError,
};
use crate::geometry::Aabb;

use self::bounds::union_optional;
use super::{MeshNode, NodeKey, NodeKind, Scene, SceneSkinBinding, Transform};

mod accessors;
mod bounds;
mod lookups;
mod options;

#[derive(Debug, Clone)]
pub struct SceneImport {
    roots: Vec<NodeKey>,
    records: Vec<ImportedNode>,
    anchors: Vec<ImportAnchor>,
    clips: Vec<ImportClip>,
    diagnostic_overlays: Vec<ImportDiagnosticOverlay>,
    live: Arc<AtomicBool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportAnchor {
    name: String,
    node: NodeKey,
    transform: Transform,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceCoordinateSystem {
    #[default]
    GltfYUpRightHanded,
    YUpLeftHanded,
    ZUpRightHanded,
    ZUpLeftHanded,
}

#[derive(Debug, Clone, PartialEq)]
struct ImportedNode {
    source_index: usize,
    node: NodeKey,
    parent: Option<NodeKey>,
    name: Option<String>,
    bounds: Option<Aabb>,
}

struct ImportBuild<'a> {
    scene_asset: &'a SceneAsset,
    options: ImportOptions,
    records: &'a mut Vec<ImportedNode>,
    anchors: &'a mut Vec<ImportAnchor>,
    diagnostic_overlays: &'a mut Vec<ImportDiagnosticOverlay>,
    pending_skin_bindings: &'a mut Vec<PendingSkinBinding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingSkinBinding {
    node: NodeKey,
    source_node: usize,
    skin: usize,
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
            clips: Vec::new(),
            diagnostic_overlays: Vec::new(),
            live: Arc::new(AtomicBool::new(true)),
        };
        let mut pending_skin_bindings = Vec::new();
        for source_index in roots {
            let mut build = ImportBuild {
                scene_asset,
                options,
                records: &mut import.records,
                anchors: &mut import.anchors,
                diagnostic_overlays: &mut import.diagnostic_overlays,
                pending_skin_bindings: &mut pending_skin_bindings,
            };
            let node =
                self.instantiate_scene_asset_node(source_index, self.root, None, &mut build)?;
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
        import.mark_stale();
        self.instantiate(scene_asset)
    }

    fn instantiate_scene_asset_node(
        &mut self,
        source_index: usize,
        parent: NodeKey,
        imported_parent: Option<NodeKey>,
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
        let label = source_node.name().map(str::to_string);
        build.diagnostic_overlays.push(ImportDiagnosticOverlay::new(
            ImportDiagnosticOverlayKind::Origin,
            node,
            transform,
            None,
            label.clone(),
        ));
        build.diagnostic_overlays.push(ImportDiagnosticOverlay::new(
            ImportDiagnosticOverlayKind::Axes,
            node,
            transform,
            None,
            label.clone(),
        ));
        if let Some(bounds) = bounds {
            build.diagnostic_overlays.push(ImportDiagnosticOverlay::new(
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
            let anchor_transform = build.options.convert_transform(anchor.transform());
            build.anchors.push(ImportAnchor {
                name: anchor.name().to_string(),
                node,
                transform: anchor_transform,
            });
            build.diagnostic_overlays.push(ImportDiagnosticOverlay::new(
                ImportDiagnosticOverlayKind::Anchor,
                node,
                anchor_transform,
                None,
                Some(anchor.name().to_string()),
            ));
            if anchor.name() == "pivot" {
                build.diagnostic_overlays.push(ImportDiagnosticOverlay::new(
                    ImportDiagnosticOverlayKind::Pivot,
                    node,
                    anchor_transform,
                    None,
                    Some(anchor.name().to_string()),
                ));
            }
        }
        for child in source_node.children() {
            if build.scene_asset.nodes().get(*child).is_none() {
                return Err(InstantiateError::InvalidChildIndex {
                    parent: source_index,
                    child: *child,
                });
            }
            self.instantiate_scene_asset_node(*child, node, Some(node), build)?;
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

impl SceneImport {
    pub(crate) fn live_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.live)
    }
}

fn mesh_node_kind(mesh: &SceneAssetMesh) -> NodeKind {
    NodeKind::Mesh(MeshNode {
        geometry: mesh.geometry(),
        material: mesh.material(),
    })
}
