use crate::assets::{Assets, GeometryHandle, MaterialHandle, TextureHandle};
use crate::diagnostics::PrepareError;
use crate::material::MaterialKind;
use crate::scene::{Scene, Vec3};

use super::lighting::PreparedLights;
use super::materials::{PreparedMaterialTextures, validate_material_texture_handles};
use super::primitives::append_geometry_primitives;
use super::shadows;
use super::transforms;
use super::types::{
    DeformationInputs, GeometryPrimitiveSource, PreparedInstanceRecord, PreparedInstanceSet,
    PreparedStrokeSegment, PrimitiveBakeParams, PrimitiveSinks,
};
use super::{PreparedEnvironmentLighting, draw_uniform_tint};
use crate::render::{PrepareWorkCounter, RasterTarget, camera::CameraProjection};

const MIN_AUTO_INSTANCE_GROUP_SIZE: usize = 4;

#[derive(Debug, Clone, Copy)]
struct AutoInstanceCandidate {
    node: crate::scene::NodeKey,
    transform: crate::scene::Transform,
}

#[derive(Debug)]
struct AutoInstanceGroup {
    geometry: GeometryHandle,
    material: MaterialHandle,
    candidates: Vec<AutoInstanceCandidate>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn append_auto_instanced_mesh_groups<F>(
    target: RasterTarget,
    screen_space_scale: f32,
    scene: &Scene,
    assets: Option<&Assets<F>>,
    origin_shift: Vec3,
    lights: &PreparedLights,
    shadow_occluders: &shadows::ShadowOccluderSet,
    shadow_visibility_cache: &shadows::ShadowVisibilityCache,
    baked_ambient_occlusion: Option<crate::BakedAmbientOcclusionConfig>,
    camera_projection: Option<&CameraProjection>,
    backend_sampled_base_color_textures: &[TextureHandle],
    backend_material_slots: &[crate::assets::MaterialHandle],
    environment_lighting: PreparedEnvironmentLighting,
    work: Option<&PrepareWorkCounter>,
    instances: &mut Vec<PreparedInstanceSet>,
    strokes: &mut Vec<PreparedStrokeSegment>,
) -> Result<Vec<crate::scene::NodeKey>, PrepareError> {
    let Some(assets) = assets else {
        return Ok(Vec::new());
    };
    if scene.reflection_probes().next().is_some() {
        return Ok(Vec::new());
    }
    let mut groups: Vec<AutoInstanceGroup> = Vec::new();
    for (node, mesh, transform) in scene.mesh_nodes() {
        if scene.mesh_lods(node).is_some() {
            continue;
        }
        if scene.morph_weights(node).is_some() || scene.skin_matrices(node).is_some() {
            continue;
        }
        let tint = scene.node_tint(node).unwrap_or(None);
        if tint.is_some_and(|tint| tint.a < 1.0) {
            continue;
        }
        let candidate = AutoInstanceCandidate { node, transform };
        if let Some(group) = groups
            .iter_mut()
            .find(|group| group.geometry == mesh.geometry() && group.material == mesh.material())
        {
            group.candidates.push(candidate);
        } else {
            groups.push(AutoInstanceGroup {
                geometry: mesh.geometry(),
                material: mesh.material(),
                candidates: vec![candidate],
            });
        }
    }

    let mut auto_instanced_nodes = Vec::new();
    for group in groups
        .into_iter()
        .filter(|group| group.candidates.len() >= MIN_AUTO_INSTANCE_GROUP_SIZE)
    {
        let source_node = group.candidates[0].node;
        let geometry =
            assets
                .geometry_snapshot(group.geometry)
                .ok_or(PrepareError::GeometryNotFound {
                    node: source_node,
                    geometry: group.geometry,
                })?;
        let material =
            assets
                .material_snapshot(group.material)
                .ok_or(PrepareError::MaterialNotFound {
                    node: source_node,
                    material: group.material,
                })?;
        if matches!(
            material.kind(),
            MaterialKind::Line | MaterialKind::Wireframe | MaterialKind::Edge
        ) {
            continue;
        }
        if material.photographic_surface_tile_size_m().is_some() {
            // Physical UV scale depends on each node's world transform. Keep
            // these nodes on the ordinary per-draw prepare path so automatic
            // batching cannot silently reuse the first instance's scale.
            continue;
        }
        validate_material_texture_handles(source_node, group.material, &material, assets)?;
        let material_textures = PreparedMaterialTextures::new(assets, &material);

        let mut retained = Vec::new();
        let mut instance_strokes = Vec::new();
        let mut transparent = Vec::new();
        append_geometry_primitives(
            GeometryPrimitiveSource {
                node: source_node,
                clip_with_scene: !scene.is_overlay_owned_node(source_node),
                instance: None,
                material_handle: group.material,
                geometry: &geometry,
                material: &material,
                textures: &material_textures,
                tint: None,
            },
            DeformationInputs::default(),
            PrimitiveBakeParams {
                target,
                screen_space_scale,
                transform: crate::scene::Transform::IDENTITY,
                origin_shift,
                lights,
                shadow_occluders,
                shadow_visibility_cache,
                baked_ambient_occlusion,
                camera_projection,
                backend_sampled_base_color_textures,
                backend_material_slots,
                environment_lighting: environment_lighting.clone(),
                reflection_probe: None,
                work,
            },
            PrimitiveSinks {
                primitives: &mut retained,
                strokes: &mut instance_strokes,
                transparent_primitives: &mut transparent,
            },
        )?;
        retained.extend(
            transparent
                .into_iter()
                .map(|transparent| transparent.primitive),
        );
        if retained.is_empty() {
            continue;
        }

        let records = group
            .candidates
            .iter()
            .map(|candidate| {
                PreparedInstanceRecord::auto_batched(
                    transforms::world_from_model_matrix(candidate.transform, Vec3::ZERO),
                    transforms::normal_from_model_matrix(candidate.transform),
                    draw_uniform_tint(scene.node_tint(candidate.node).unwrap_or(None)),
                )
            })
            .collect::<Vec<_>>();
        strokes.extend(instance_strokes);
        auto_instanced_nodes.extend(group.candidates.iter().map(|candidate| candidate.node));
        instances.push(PreparedInstanceSet::new_auto_batched(
            source_node,
            retained,
            records,
        ));
    }
    Ok(auto_instanced_nodes)
}
