use std::sync::atomic::Ordering;

use crate::diagnostics::{ImportDiagnosticOverlay, LookupError};
use crate::geometry::Aabb;

use super::bounds::{transform_aabb, union_optional};
use super::{ImportAnchor, ImportAnchorDebugMetadata, ImportClip, ImportPivot, SceneImport};
use crate::scene::{NodeKey, Scene, Transform};

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

    pub fn pivot(&self, node_name: &str) -> Result<ImportPivot, LookupError> {
        self.ensure_live()?;
        let node = self.node(node_name)?;
        let pivot = self
            .anchors
            .iter()
            .find(|anchor| anchor.node == node && anchor.name == "pivot");
        Ok(ImportPivot {
            name: pivot.map(|anchor| anchor.name.clone()),
            node,
            transform: pivot
                .map(|anchor| anchor.transform)
                .unwrap_or(Transform::IDENTITY),
        })
    }

    pub fn diagnostic_overlays(&self) -> Result<&[ImportDiagnosticOverlay], LookupError> {
        self.ensure_live()?;
        Ok(&self.diagnostic_overlays)
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

    pub fn anchors(&self) -> Result<&[ImportAnchor], LookupError> {
        self.ensure_live()?;
        Ok(&self.anchors)
    }

    pub fn anchors_for(&self, node: NodeKey) -> Result<Vec<&ImportAnchor>, LookupError> {
        self.ensure_live()?;
        Ok(self
            .anchors
            .iter()
            .filter(|anchor| anchor.node() == node)
            .collect())
    }

    pub fn anchor_debug_metadata(&self) -> Result<Vec<ImportAnchorDebugMetadata>, LookupError> {
        self.ensure_live()?;
        Ok(self
            .anchors
            .iter()
            .map(ImportAnchorDebugMetadata::from)
            .collect())
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

    pub(super) fn ensure_live(&self) -> Result<(), LookupError> {
        if self.is_live() {
            Ok(())
        } else {
            Err(LookupError::StaleImport)
        }
    }

    pub(super) fn is_live(&self) -> bool {
        self.live.load(Ordering::Acquire)
    }

    pub(super) fn mark_stale(&self) {
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
