use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::animation::AnimationClipKey;
use crate::assets::{AssetFetcher, AssetPath, Assets, SceneAsset};
use crate::diagnostics::{ImportError, InstantiateError, LookupError};
use crate::geometry::Aabb;

use self::bounds::{transform_aabb, union_optional};
use super::{MeshNode, NodeKey, NodeKind, Scene, Transform, Vec3};

mod bounds;

#[derive(Debug, Clone)]
pub struct SceneImport {
    roots: Vec<NodeKey>,
    records: Vec<ImportedNode>,
    anchors: Vec<ImportAnchor>,
    clips: Vec<ImportClip>,
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
            live: Arc::new(AtomicBool::new(true)),
        };
        for source_index in roots {
            let mut build = ImportBuild {
                scene_asset,
                options,
                records: &mut import.records,
                anchors: &mut import.anchors,
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
        let kind = source_node
            .mesh()
            .map(|mesh| {
                NodeKind::Mesh(MeshNode {
                    geometry: mesh.geometry(),
                    material: mesh.material(),
                })
            })
            .unwrap_or(NodeKind::Empty);
        let bounds = source_node.mesh().map(|mesh| mesh.bounds());
        let node = self
            .insert_node(
                parent,
                kind,
                build.options.convert_transform(source_node.transform()),
            )
            .expect("import parent was inserted by this scene");
        build.records.push(ImportedNode {
            node,
            parent: imported_parent,
            name: source_node.name().map(str::to_string),
            bounds,
        });
        let mut anchor_names = BTreeSet::new();
        for anchor in source_node.anchors() {
            if !anchor_names.insert(anchor.name()) {
                return Err(InstantiateError::InvalidAnchorExtras {
                    node: source_node.name().unwrap_or("<unnamed>").to_string(),
                    reason: format!("duplicate anchor '{}'", anchor.name()),
                });
            }
            build.anchors.push(ImportAnchor {
                name: anchor.name().to_string(),
                node,
                transform: build.options.convert_transform(anchor.transform()),
            });
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

impl SceneImport {
    pub fn node(&self, name: &str) -> Result<NodeKey, LookupError> {
        self.ensure_live()?;
        let matches = self.nodes_named(name).collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Err(LookupError::NodeNameNotFound {
                name: name.to_string(),
            }),
            [node] => Ok(*node),
            _ => Err(LookupError::AmbiguousNodeName {
                name: name.to_string(),
                matches,
            }),
        }
    }

    pub fn first_node(&self, name: &str) -> Option<NodeKey> {
        if !self.is_live() {
            return None;
        }
        self.nodes_named(name).next()
    }

    pub fn nodes_named<'import>(
        &'import self,
        name: &'import str,
    ) -> impl Iterator<Item = NodeKey> + 'import {
        self.records
            .iter()
            .filter(move |record| record.name.as_deref() == Some(name))
            .map(|record| record.node)
    }

    pub fn path(&self, path: &str) -> Result<NodeKey, LookupError> {
        self.ensure_live()?;
        let segments = path_segments(path).ok_or_else(|| LookupError::PathNotFound {
            path: path.to_string(),
        })?;
        let Some((first, rest)) = segments.split_first() else {
            return Err(LookupError::PathNotFound {
                path: path.to_string(),
            });
        };
        let mut current = self
            .records
            .iter()
            .find(|record| record.parent.is_none() && record.name.as_deref() == Some(first))
            .map(|record| record.node)
            .ok_or_else(|| LookupError::PathNotFound {
                path: path.to_string(),
            })?;

        for segment in rest {
            current = self
                .records
                .iter()
                .find(|record| {
                    record.parent == Some(current) && record.name.as_deref() == Some(segment)
                })
                .map(|record| record.node)
                .ok_or_else(|| LookupError::PathNotFound {
                    path: path.to_string(),
                })?;
        }
        Ok(current)
    }

    pub fn roots(&self) -> &[NodeKey] {
        &self.roots
    }

    pub fn clip(&self, name: &str) -> Result<&ImportClip, LookupError> {
        self.ensure_live()?;
        let matches = self.clips_named(name).collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Err(LookupError::ClipNotFound {
                name: name.to_string(),
            }),
            [clip] => Ok(*clip),
            _ => Err(LookupError::AmbiguousClipName {
                name: name.to_string(),
                matches: matches.iter().map(|clip| clip.key()).collect(),
            }),
        }
    }

    pub fn first_clip(&self, name: &str) -> Option<&ImportClip> {
        if !self.is_live() {
            return None;
        }
        self.clips_named(name).next()
    }

    pub fn clips_named<'import>(
        &'import self,
        name: &str,
    ) -> impl Iterator<Item = &'import ImportClip> + 'import {
        let name = name.to_string();
        self.clips
            .iter()
            .filter(move |clip| clip.name() == Some(name.as_str()))
    }

    pub fn anchor(&self, name: &str) -> Result<&ImportAnchor, LookupError> {
        self.ensure_live()?;
        let matches = self.anchors_named(name).collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Err(LookupError::AnchorNotFound {
                name: name.to_string(),
            }),
            [anchor] => Ok(*anchor),
            _ => Err(LookupError::AmbiguousAnchorName {
                name: name.to_string(),
                hosts: matches.iter().map(|anchor| anchor.node()).collect(),
            }),
        }
    }

    pub fn first_anchor(&self, name: &str) -> Option<&ImportAnchor> {
        if !self.is_live() {
            return None;
        }
        self.anchors_named(name).next()
    }

    pub fn anchors_named<'import>(
        &'import self,
        name: &str,
    ) -> impl Iterator<Item = &'import ImportAnchor> + 'import {
        let name = name.to_string();
        self.anchors
            .iter()
            .filter(move |anchor| anchor.name() == name.as_str())
    }

    pub fn bounds_local(&self) -> Option<Aabb> {
        if !self.is_live() {
            return None;
        }
        self.records
            .iter()
            .filter_map(|record| record.bounds)
            .fold(None, |bounds, next| Some(union_optional(bounds, next)))
    }

    pub fn bounds_world(&self, scene: &Scene) -> Option<Aabb> {
        if !self.is_live() {
            return None;
        }
        self.records
            .iter()
            .filter_map(|record| {
                let bounds = record.bounds?;
                let transform = scene.node(record.node)?.transform();
                Some(transform_aabb(bounds, transform))
            })
            .fold(None, |bounds, next| Some(union_optional(bounds, next)))
    }

    fn ensure_live(&self) -> Result<(), LookupError> {
        if self.is_live() {
            Ok(())
        } else {
            Err(LookupError::StaleImport)
        }
    }

    fn is_live(&self) -> bool {
        self.live.load(Ordering::Acquire)
    }

    fn mark_stale(&self) {
        self.live.store(false, Ordering::Release);
    }
}

fn path_segments(path: &str) -> Option<Vec<String>> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut escaped = false;

    for character in path.chars() {
        if escaped {
            current.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '/' {
            if current.is_empty() {
                return None;
            }
            segments.push(std::mem::take(&mut current));
        } else {
            current.push(character);
        }
    }

    if escaped || current.is_empty() {
        return None;
    }
    segments.push(current);
    Some(segments)
}
