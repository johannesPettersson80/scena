use std::collections::{BTreeMap, BTreeSet};

use crate::diagnostics::{Backend, RenderError};
use crate::scene::{CameraKey, InstanceId, NodeKey, Scene};

use super::camera::CameraProjection;
use super::prepare::{PreparedInstanceSet, PreparedPrimitive};
use super::{RasterTarget, Renderer};

const MAX_PALETTE_INDEX: usize = 0x00ff_ffff;

#[derive(Debug)]
pub(crate) enum RawSemanticAovError {
    Render(RenderError),
    UnsupportedBackend(Backend),
    PaletteExhausted { entries: usize },
}

impl From<RenderError> for RawSemanticAovError {
    fn from(error: RenderError) -> Self {
        Self::Render(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RawSemanticIdentity {
    pub(crate) node: NodeKey,
    pub(crate) instance: Option<InstanceId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawSemanticLegendEntry {
    pub(crate) palette_index: u32,
    pub(crate) identity: RawSemanticIdentity,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RawSemanticAovExclusions {
    pub(crate) transparent_triangle_count: usize,
    pub(crate) overlay_triangle_count: usize,
    pub(crate) unattributed_triangle_count: usize,
    pub(crate) stroke_segment_count: usize,
    pub(crate) label_quad_count: usize,
    pub(crate) gpu_instance_record_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawSemanticAovCapture {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) near: f32,
    pub(crate) far: f32,
    pub(crate) id_indices: Vec<u32>,
    pub(crate) depth_meters: Vec<f32>,
    pub(crate) world_normals: Vec<[f32; 3]>,
    pub(crate) legend: Vec<RawSemanticLegendEntry>,
    pub(crate) exclusions: RawSemanticAovExclusions,
}

#[derive(Debug, Clone)]
pub(crate) struct GpuSemanticAttribution {
    pub(crate) legend: Vec<RawSemanticLegendEntry>,
    pub(crate) exclusions: RawSemanticAovExclusions,
    palette: BTreeMap<RawSemanticIdentity, u32>,
}

impl GpuSemanticAttribution {
    pub(crate) fn palette_index(&self, node: NodeKey, instance: Option<InstanceId>) -> u32 {
        self.palette
            .get(&RawSemanticIdentity { node, instance })
            .copied()
            .unwrap_or(0)
    }
}

pub(in crate::render) fn build_gpu_semantic_attribution(
    primitives: &[PreparedPrimitive],
    instances: &[PreparedInstanceSet],
    stroke_segment_count: usize,
    label_quad_count: usize,
) -> Result<GpuSemanticAttribution, usize> {
    let mut exclusions = RawSemanticAovExclusions {
        stroke_segment_count,
        label_quad_count,
        gpu_instance_record_count: instances.iter().map(|set| set.instances().len()).sum(),
        ..RawSemanticAovExclusions::default()
    };
    let mut identities = BTreeSet::new();
    for primitive in primitives {
        collect_primitive_identity(primitive, 1, &mut identities, &mut exclusions);
    }
    for set in instances {
        let instance_count = set.instances().len();
        if instance_count == 0 {
            continue;
        }
        let attributable = set
            .primitives()
            .iter()
            .any(|primitive| primitive.semantic_opaque());
        for primitive in set.primitives() {
            if primitive.semantic_opaque() {
                continue;
            }
            if primitive.semantic_overlay() {
                exclusions.overlay_triangle_count = exclusions
                    .overlay_triangle_count
                    .saturating_add(instance_count);
            } else {
                exclusions.transparent_triangle_count = exclusions
                    .transparent_triangle_count
                    .saturating_add(instance_count);
            }
        }
        if !attributable {
            continue;
        }
        for record in set.instances() {
            identities.insert(RawSemanticIdentity {
                node: set.source_node(),
                instance: record.source_instance(),
            });
        }
    }
    if identities.len() > MAX_PALETTE_INDEX {
        return Err(identities.len());
    }
    let legend = identities
        .into_iter()
        .enumerate()
        .map(|(index, identity)| RawSemanticLegendEntry {
            palette_index: (index + 1) as u32,
            identity,
        })
        .collect::<Vec<_>>();
    let palette = legend
        .iter()
        .map(|entry| (entry.identity, entry.palette_index))
        .collect();
    Ok(GpuSemanticAttribution {
        legend,
        exclusions,
        palette,
    })
}

fn collect_primitive_identity(
    primitive: &PreparedPrimitive,
    multiplier: usize,
    identities: &mut BTreeSet<RawSemanticIdentity>,
    exclusions: &mut RawSemanticAovExclusions,
) {
    if !primitive.semantic_opaque() {
        if primitive.semantic_overlay() {
            exclusions.overlay_triangle_count =
                exclusions.overlay_triangle_count.saturating_add(multiplier);
        } else {
            exclusions.transparent_triangle_count = exclusions
                .transparent_triangle_count
                .saturating_add(multiplier);
        }
        return;
    }
    let Some(node) = primitive.source_node() else {
        exclusions.unattributed_triangle_count = exclusions
            .unattributed_triangle_count
            .saturating_add(multiplier);
        return;
    };
    identities.insert(RawSemanticIdentity {
        node,
        instance: primitive.source_instance(),
    });
}

impl Renderer {
    pub(crate) fn semantic_aov_raw(
        &self,
        scene: &Scene,
        camera: CameraKey,
    ) -> Result<RawSemanticAovCapture, RawSemanticAovError> {
        let prepared = self.prepared_state(scene)?;
        if self.target.backend != Backend::Headless {
            return Err(RawSemanticAovError::UnsupportedBackend(self.target.backend));
        }
        let projection = CameraProjection::from_scene(scene, camera, self.target)?;
        let [near, far] = projection.near_far();

        let attribution = build_gpu_semantic_attribution(
            &prepared.primitives,
            &prepared.instances,
            prepared.strokes.len(),
            prepared.labels.quads().len(),
        )
        .map_err(|entries| RawSemanticAovError::PaletteExhausted { entries })?;
        let palette = &attribution.palette;

        let mut capture = RawSemanticAovCapture {
            width: self.target.width,
            height: self.target.height,
            near,
            far,
            id_indices: vec![0; self.target.pixel_len()],
            depth_meters: vec![f32::INFINITY; self.target.pixel_len()],
            world_normals: vec![[0.0; 3]; self.target.pixel_len()],
            legend: attribution.legend,
            exclusions: attribution.exclusions,
        };
        for primitive in prepared.primitives.iter() {
            if !primitive.semantic_opaque() {
                continue;
            }
            let Some(node) = primitive.source_node() else {
                continue;
            };
            let identity = RawSemanticIdentity {
                node,
                instance: primitive.source_instance(),
            };
            let Some(palette_index) = palette.get(&identity).copied() else {
                continue;
            };
            rasterize_primitive(
                &mut capture,
                primitive,
                palette_index,
                self.target,
                &projection,
                &prepared.clipping_planes,
                prepared.section_box,
            );
        }
        Ok(capture)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn semantic_aov_gpu_raw(
        &mut self,
        scene: &Scene,
        camera: CameraKey,
    ) -> Result<RawSemanticAovCapture, RawSemanticAovError> {
        if !matches!(
            self.target.backend,
            Backend::HeadlessGpu | Backend::NativeSurface
        ) {
            return Err(RawSemanticAovError::UnsupportedBackend(self.target.backend));
        }
        let projection = CameraProjection::from_scene(scene, camera, self.target)?;
        let (clipping_planes, section_box) = {
            let prepared = self.prepared_state(scene)?;
            (prepared.clipping_planes.clone(), prepared.section_box)
        };
        let gpu = self
            .gpu
            .as_mut()
            .ok_or(RenderError::GpuResourcesNotPrepared {
                backend: self.target.backend,
            })?;
        gpu.capture_semantic_aov(self.target, &projection, &clipping_planes, section_box)
            .map_err(RawSemanticAovError::Render)
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) async fn semantic_aov_gpu_raw(
        &mut self,
        scene: &Scene,
        camera: CameraKey,
    ) -> Result<RawSemanticAovCapture, RawSemanticAovError> {
        if !matches!(self.target.backend, Backend::WebGpu | Backend::WebGl2) {
            return Err(RawSemanticAovError::UnsupportedBackend(self.target.backend));
        }
        let projection = CameraProjection::from_scene(scene, camera, self.target)?;
        let (clipping_planes, section_box) = {
            let prepared = self.prepared_state(scene)?;
            (prepared.clipping_planes.clone(), prepared.section_box)
        };
        let gpu = self
            .gpu
            .as_mut()
            .ok_or(RenderError::GpuResourcesNotPrepared {
                backend: self.target.backend,
            })?;
        gpu.capture_semantic_aov(self.target, &projection, &clipping_planes, section_box)
            .await
            .map_err(RawSemanticAovError::Render)
    }
}

#[allow(clippy::too_many_arguments)]
fn rasterize_primitive(
    capture: &mut RawSemanticAovCapture,
    primitive: &PreparedPrimitive,
    palette_index: u32,
    target: RasterTarget,
    camera: &CameraProjection,
    clipping_planes: &[crate::scene::ClippingPlane],
    section_box: Option<crate::scene::SectionBox>,
) {
    let projected = super::cpu_geometry::project_clipped_primitive(primitive, target, camera);
    for triangle in projected.triangles() {
        rasterize_projected_primitive(
            capture,
            primitive,
            palette_index,
            *triangle,
            target,
            camera,
            clipping_planes,
            section_box,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn rasterize_projected_primitive(
    capture: &mut RawSemanticAovCapture,
    primitive: &PreparedPrimitive,
    palette_index: u32,
    triangle: super::cpu_geometry::CpuScreenTriangle,
    target: RasterTarget,
    camera: &CameraProjection,
    clipping_planes: &[crate::scene::ClippingPlane],
    section_box: Option<crate::scene::SectionBox>,
) {
    let vertices = triangle.vertices();
    let [a, b, c] = vertices;
    let area = super::cpu_geometry::edge(a, b, c.x, c.y);
    if area.abs() <= f32::EPSILON || (!primitive.double_sided() && area < 0.0) {
        return;
    }
    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as u32;
    let max_x =
        a.x.max(b.x)
            .max(c.x)
            .ceil()
            .min(target.width.saturating_sub(1) as f32) as u32;
    let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as u32;
    let max_y =
        a.y.max(b.y)
            .max(c.y)
            .ceil()
            .min(target.height.saturating_sub(1) as f32) as u32;
    if min_x > max_x || min_y > max_y {
        return;
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let affine = [
                super::cpu_geometry::edge(b, c, px, py) / area,
                super::cpu_geometry::edge(c, a, px, py) / area,
                super::cpu_geometry::edge(a, b, px, py) / area,
            ];
            if !super::cpu_geometry::barycentric_sample_is_inside(affine) {
                continue;
            }
            let weights = super::cpu_geometry::perspective_weights(camera, vertices, affine);
            let world =
                super::cpu_geometry::weighted_vec3([a.position, b.position, c.position], weights);
            if super::cpu_geometry::point_is_clipped(world, clipping_planes, section_box) {
                continue;
            }
            if let Some(cutoff) = primitive.semantic_alpha_cutoff() {
                let alpha = a.color.a * primitive.tint().a * weights[0]
                    + b.color.a * primitive.tint().a * weights[1]
                    + c.color.a * primitive.tint().a * weights[2];
                if !alpha.is_finite() || alpha < cutoff {
                    continue;
                }
            }
            let depth = a.projected.view_depth * weights[0]
                + b.projected.view_depth * weights[1]
                + c.projected.view_depth * weights[2];
            if !depth.is_finite() || depth <= 0.0 {
                continue;
            }
            let pixel = target.pixel_index(x, y);
            let current_depth = capture.depth_meters[pixel];
            let current_id = capture.id_indices[pixel];
            let closer = depth < current_depth - f32::EPSILON;
            let tied = (depth - current_depth).abs() <= f32::EPSILON;
            if !closer && !(tied && (current_id == 0 || palette_index < current_id)) {
                continue;
            }
            let mut normal = super::cpu_geometry::weighted_vec3(
                [
                    a.attributes.normal,
                    b.attributes.normal,
                    c.attributes.normal,
                ],
                weights,
            )
            .normalize_or_zero();
            if normal.length_squared() <= f32::EPSILON {
                normal = (b.position - a.position)
                    .cross(c.position - a.position)
                    .normalize_or_zero();
            }
            capture.id_indices[pixel] = palette_index;
            capture.depth_meters[pixel] = depth;
            capture.world_normals[pixel] = [normal.x, normal.y, normal.z];
        }
    }
}

#[cfg(test)]
mod depth_clipping_tests {
    use super::*;
    use crate::geometry::{Primitive, Vertex};
    use crate::material::Color;
    use crate::scene::{DepthRange, PerspectiveCamera, Transform};

    #[test]
    fn semantic_aov_preserves_identity_depth_and_normals_across_near_clip() {
        let target = RasterTarget {
            width: 64,
            height: 64,
            backend: Backend::Headless,
        };
        let mut scene = Scene::new();
        let camera_key = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::default().with_depth_range(DepthRange::new(0.5, 5.0)),
                Transform::default(),
            )
            .expect("camera inserts");
        let camera =
            CameraProjection::from_scene(&scene, camera_key, target).expect("projection builds");
        let primitive = PreparedPrimitive::new(
            Primitive::triangle([
                vertex(-0.4, -0.4, -1.0),
                vertex(0.4, -0.4, -1.0),
                vertex(0.0, 0.4, -0.25),
            ]),
            None,
            Color::WHITE,
        );
        let mut capture = RawSemanticAovCapture {
            width: target.width,
            height: target.height,
            near: 0.5,
            far: 5.0,
            id_indices: vec![0; target.pixel_len()],
            depth_meters: vec![f32::INFINITY; target.pixel_len()],
            world_normals: vec![[0.0; 3]; target.pixel_len()],
            legend: Vec::new(),
            exclusions: RawSemanticAovExclusions::default(),
        };

        rasterize_primitive(&mut capture, &primitive, 7, target, &camera, &[], None);

        let covered = capture
            .id_indices
            .iter()
            .enumerate()
            .filter(|(_, identity)| **identity == 7)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        assert!(covered.len() > 100);
        assert!(covered.iter().all(|index| {
            capture.depth_meters[*index].is_finite()
                && capture.depth_meters[*index] >= 0.5
                && capture.depth_meters[*index] <= 5.0
                && capture.world_normals[*index]
                    .iter()
                    .all(|value| value.is_finite())
        }));
    }

    fn vertex(x: f32, y: f32, z: f32) -> Vertex {
        Vertex {
            position: crate::scene::Vec3::new(x, y, z),
            color: Color::WHITE,
        }
    }
}
