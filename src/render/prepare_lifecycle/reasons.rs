//! Why a prepare has to bake the scene again.
//!
//! A prepare first asks whether the retained template still describes the
//! scene. It does when everything that changed is something the retained
//! path re-derives: per-draw matrices and tints, hidden sources, culling, the
//! order of blended triangles. Each variant below names something the retained
//! path cannot re-derive, and the decision in [`Renderer::full_prepare_reason`]
//! only raises it when the change at hand actually touches it.

use crate::assets::Assets;
use crate::scene::Scene;

use super::super::prepare_retained::PreparedView;
use super::super::{Renderer, camera, prepare};

/// What keeps a prepare from reusing the retained template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render) enum FullPrepareReason {
    /// Nothing has been prepared yet.
    NoTemplate,
    /// The template was baked from another scene.
    DifferentScene,
    /// Nodes, geometry, materials, lights or other scene structure changed.
    SceneStructure,
    /// The environment the template was lit with changed.
    Environment,
    /// The render target or a setting baked into the template changed.
    Target,
    /// Output settings that own GPU resources changed.
    OutputResources,
    /// A model node is visible. Preparing one is a structured error, which
    /// only the full path reports.
    ModelNode,
    /// The CPU renderer draws world-space triangles, so content that moved
    /// has to be baked where it is now.
    CpuPlacement,
    /// Section caps are cut from where each mesh meets the section box.
    SectionCaps,
    /// A reflection probe was added, removed or changed, or a mesh moved
    /// and may now sit in the bounds of another one. Probe environments
    /// are uploaded with the template and every mesh keeps the probe it
    /// was baked with.
    ReflectionProbes,
    /// Skinned vertices are deformed by joint placement at bake time.
    Skinning,
    /// Vertex data was computed from where things are in the world (see
    /// [`prepare::BakeDependencies::placement`]) and something moved, was
    /// hidden or was shown.
    PlacementDependentBake,
    /// Vertex data was computed from the camera (see
    /// [`prepare::BakeDependencies::view`]) and the view changed.
    ViewDependentBake,
    /// A mesh with LOD levels would now be baked from another level.
    MeshLod,
    /// Label quads are laid out for the camera and for the set of visible
    /// labels, and uploaded once.
    Labels,
    /// More punctual lights than the fixed uniform holds are binned into
    /// screen tiles for the camera, in buffers sized for that binning.
    TiledLights,
    /// The template cannot draw the scene as it is now: a visible source is
    /// missing from it, a source was baked per instance, a node changed
    /// handedness or lost its inverse, or an automatically batched group
    /// would no longer form.
    TemplateCoverage,
    /// The directional shadow projection could not be rebuilt.
    ShadowProjection,
    /// The GPU resources of the last full prepare cannot hold or draw the
    /// re-derived state.
    Gpu(&'static str),
}

impl FullPrepareReason {
    pub(in crate::render) const fn as_str(self) -> &'static str {
        match self {
            Self::NoTemplate => "no prepared template",
            Self::DifferentScene => "scene identity changed",
            Self::SceneStructure => "structure revision changed",
            Self::Environment => "environment revision changed",
            Self::Target => "target revision changed",
            Self::OutputResources => "output resources revision changed",
            Self::ModelNode => "model nodes present",
            Self::CpuPlacement => "CPU triangles are baked in world space",
            Self::SectionCaps => "section caps follow mesh placement",
            Self::ReflectionProbes => "reflection probes changed or content moved among them",
            Self::Skinning => "skinned joints may have moved",
            Self::PlacementDependentBake => "vertex data was baked from scene placement",
            Self::ViewDependentBake => "vertex data was baked from the camera",
            Self::MeshLod => "mesh LOD selection changed",
            Self::Labels => "labels are laid out for another view or another visible set",
            Self::TiledLights => "tiled light assignment may have moved",
            Self::TemplateCoverage => "retained template cannot draw a visible source",
            Self::ShadowProjection => "dynamic shadow projection failed",
            Self::Gpu(reason) => reason,
        }
    }
}

impl Renderer {
    /// [`Self::full_prepare_reason`] for the camera a prepare would use.
    #[cfg(test)]
    pub(in crate::render) fn full_prepare_reason_for_test(
        &self,
        scene: &Scene,
        assets: &Assets,
    ) -> Option<FullPrepareReason> {
        let target = self.prepare_target().ok()?;
        let camera = scene
            .active_camera()
            .and_then(|camera| camera::CameraProjection::from_scene(scene, camera, target).ok());
        self.full_prepare_reason(scene, Some(assets), camera.as_ref())
    }

    /// Whether, and why, preparing `scene` now has to bake it again.
    ///
    /// `None` means the retained template is still a faithful bake of the
    /// scene for everything the retained path does not re-derive.
    pub(in crate::render) fn full_prepare_reason<F>(
        &self,
        scene: &Scene,
        assets: Option<&Assets<F>>,
        camera: Option<&camera::CameraProjection>,
    ) -> Option<FullPrepareReason> {
        use FullPrepareReason as Reason;
        let Some(prepared) = self.prepared.as_ref() else {
            return Some(Reason::NoTemplate);
        };
        if !prepared.scene.ptr_eq(&scene.identity()) {
            return Some(Reason::DifferentScene);
        }
        if prepared.structure_revision != scene.structure_revision() {
            return Some(Reason::SceneStructure);
        }
        if prepared.environment_revision != self.environment_revision {
            return Some(Reason::Environment);
        }
        if prepared.target_revision != self.target_revision {
            return Some(Reason::Target);
        }
        if prepared.output_resources_revision != self.output_resources_revision {
            return Some(Reason::OutputResources);
        }
        if scene.model_nodes().next().is_some() {
            return Some(Reason::ModelNode);
        }

        // The scene bumps one transform revision for the camera and for
        // everything else. Telling the two apart is what lets a camera move
        // keep a template that only moved content would invalidate.
        let view = PreparedView::of(scene);
        let view_changed = view != prepared.view;
        let content_moved = prepared.transform_revision != scene.transform_revision()
            && !prepared.content.matches(scene);
        // Which nodes are visible also follows from the active camera's layer
        // mask, and switching cameras bumps no revision.
        let visibility_changed = prepared.visibility_revision != scene.visibility_revision()
            || view.camera() != prepared.view.camera();

        if content_moved {
            if self.gpu.is_none() {
                return Some(Reason::CpuPlacement);
            }
            if scene.section_box().is_some() {
                return Some(Reason::SectionCaps);
            }
            // Hidden meshes count too: the template keeps their deformed
            // vertices for when they are shown again.
            if scene
                .node_transforms()
                .any(|(node, _transform)| scene.skin_binding(node).is_some())
            {
                return Some(Reason::Skinning);
            }
        }
        if !prepared.probes.matches(scene)
            || (content_moved && prepared.probes.assign_by_placement())
        {
            return Some(Reason::ReflectionProbes);
        }
        if (content_moved || visibility_changed) && prepared.bake.placement {
            return Some(Reason::PlacementDependentBake);
        }
        if content_moved || view_changed {
            if prepared.bake.view {
                return Some(Reason::ViewDependentBake);
            }
            if scene.label_nodes().next().is_some() {
                return Some(Reason::Labels);
            }
            if prepare::collect_mesh_lod_selection(scene, assets, camera) != prepared.lod_selection
            {
                return Some(Reason::MeshLod);
            }
            if self.gpu.is_some() && prepare::gpu_tiled_light_assignment_required(scene) {
                return Some(Reason::TiledLights);
            }
        }
        None
    }
}
