use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::animation::AnimationClipKey;
use crate::assets::{AssetFetcher, AssetPath, Assets, SceneAsset, SceneAssetMesh};
use crate::diagnostics::{
    ImportDiagnosticOverlay, ImportDiagnosticOverlayKind, ImportError, InstantiateError,
};
use crate::geometry::Aabb;

use self::bounds::union_optional;
use super::{MeshNode, NodeKey, NodeKind, Scene, Transform, Vec3};

mod bounds;
mod lookups;

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
pub struct ImportClip {
    key: AnimationClipKey,
    name: Option<String>,
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
    ZUpRightHanded,
}

#[derive(Debug, Clone, PartialEq)]
struct ImportedNode {
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
            clips: scene_asset
                .clips()
                .iter()
                .map(|clip| ImportClip {
                    key: AnimationClipKey::fresh(),
                    name: clip.name().map(str::to_string),
                })
                .collect(),
            diagnostic_overlays: Vec::new(),
            live: Arc::new(AtomicBool::new(true)),
        };
        for source_index in roots {
            let mut build = ImportBuild {
                scene_asset,
                options,
                records: &mut import.records,
                anchors: &mut import.anchors,
                diagnostic_overlays: &mut import.diagnostic_overlays,
            };
            let node =
                self.instantiate_scene_asset_node(source_index, self.root, None, &mut build)?;
            import.roots.push(node);
        }
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
        let bounds = meshes.iter().fold(None, |bounds, mesh| {
            Some(union_optional(bounds, mesh.bounds()))
        });
        let node = match (meshes, source_node.light()) {
            ([mesh], _) => self.insert_node(parent, mesh_node_kind(*mesh), transform),
            ([_, _, ..], _) => {
                let node = self.insert_node(parent, NodeKind::Empty, transform);
                if let Ok(parent) = node {
                    for mesh in meshes {
                        self.insert_node(parent, mesh_node_kind(*mesh), Transform::IDENTITY)
                            .expect("multi-primitive parent was inserted by this scene");
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
}

fn mesh_node_kind(mesh: SceneAssetMesh) -> NodeKind {
    NodeKind::Mesh(MeshNode {
        geometry: mesh.geometry(),
        material: mesh.material(),
    })
}

impl ImportAnchor {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn node(&self) -> NodeKey {
        self.node
    }

    pub const fn transform(&self) -> Transform {
        self.transform
    }
}

impl ImportClip {
    pub const fn key(&self) -> AnimationClipKey {
        self.key
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

impl ImportPivot {
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub const fn node(&self) -> NodeKey {
        self.node
    }

    pub const fn transform(&self) -> Transform {
        self.transform
    }
}

impl ImportOptions {
    pub const fn gltf_default() -> Self {
        Self {
            source_units: SourceUnits::Meters,
            source_coordinate_system: SourceCoordinateSystem::GltfYUpRightHanded,
        }
    }

    pub const fn source_units(self) -> SourceUnits {
        self.source_units
    }

    pub const fn with_source_units(mut self, units: SourceUnits) -> Self {
        self.source_units = units;
        self
    }

    pub const fn source_coordinate_system(self) -> SourceCoordinateSystem {
        self.source_coordinate_system
    }

    pub const fn with_source_coordinate_system(
        mut self,
        coordinate_system: SourceCoordinateSystem,
    ) -> Self {
        self.source_coordinate_system = coordinate_system;
        self
    }

    fn convert_transform(self, transform: Transform) -> Transform {
        let unit_scale = self.source_units.meters_per_unit();
        Transform {
            translation: self
                .source_coordinate_system
                .convert_vec3(scale_vec3(transform.translation, unit_scale)),
            rotation: transform.rotation,
            scale: self
                .source_coordinate_system
                .convert_scale(scale_vec3(transform.scale, unit_scale)),
        }
    }
}

impl SourceUnits {
    const fn meters_per_unit(self) -> f32 {
        match self {
            Self::Meters => 1.0,
            Self::Centimeters => 0.01,
            Self::Millimeters => 0.001,
        }
    }
}

impl SourceCoordinateSystem {
    const fn convert_vec3(self, value: Vec3) -> Vec3 {
        match self {
            Self::GltfYUpRightHanded => value,
            Self::ZUpRightHanded => Vec3::new(value.x, value.z, -value.y),
        }
    }

    const fn convert_scale(self, value: Vec3) -> Vec3 {
        match self {
            Self::GltfYUpRightHanded => value,
            Self::ZUpRightHanded => Vec3::new(value.x, value.z, value.y),
        }
    }
}

const fn scale_vec3(value: Vec3, scale: f32) -> Vec3 {
    Vec3::new(value.x * scale, value.y * scale, value.z * scale)
}
