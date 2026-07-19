use super::camera::CameraProjection;
use super::prepare::PreparedPrimitive;
use super::target::RasterTarget;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CulledPrimitives {
    pub(super) visible: Vec<PreparedPrimitive>,
    pub(super) culled: u64,
}

pub(super) fn cull_prepared_primitives(
    primitives: Vec<PreparedPrimitive>,
    camera: Option<&CameraProjection>,
    target: RasterTarget,
    occlusion_enabled: bool,
    gpu_active: bool,
) -> CulledPrimitives {
    let frustum = cull_cpu_frustum(primitives, camera);
    if !occlusion_enabled || !should_run_occlusion_prepass(frustum.visible.len(), gpu_active) {
        return frustum;
    }
    let Some(camera) = camera else {
        return frustum;
    };
    let occlusion = cull_cpu_occluded_primitives(frustum.visible, camera, target);
    CulledPrimitives {
        visible: occlusion.visible,
        culled: frustum.culled.saturating_add(occlusion.culled),
    }
}

pub(super) fn cull_cpu_frustum(
    primitives: Vec<PreparedPrimitive>,
    camera: Option<&CameraProjection>,
) -> CulledPrimitives {
    let mut visible = Vec::with_capacity(primitives.len());
    let mut culled = 0_u64;
    for primitive in primitives {
        if camera.is_some_and(|camera| outside_camera_clip_box(&primitive, camera)) {
            culled = culled.saturating_add(1);
        } else {
            visible.push(primitive);
        }
    }
    CulledPrimitives { visible, culled }
}

const OCCLUSION_BUFFER_MAX_DIMENSION: u32 = 256;
const OCCLUSION_OVERLAP_TILE_DIMENSION: u32 = 16;
const OCCLUSION_DEPTH_EPSILON: f32 = 1.0e-4;
const CPU_OCCLUSION_MIN_PRIMITIVES: usize = 64;

fn should_run_occlusion_prepass(primitive_count: usize, gpu_active: bool) -> bool {
    !gpu_active && primitive_count >= CPU_OCCLUSION_MIN_PRIMITIVES
}

fn cull_cpu_occluded_primitives(
    primitives: Vec<PreparedPrimitive>,
    camera: &CameraProjection,
    target: RasterTarget,
) -> CulledPrimitives {
    if primitives.len() < 2 || target.width == 0 || target.height == 0 {
        return CulledPrimitives {
            visible: primitives,
            culled: 0,
        };
    }
    let buffer_target = occlusion_buffer_target(target);
    let projected = primitives
        .iter()
        .map(|primitive| ProjectedPrimitive::from_prepared(primitive, camera, buffer_target))
        .collect::<Vec<_>>();
    if projected.iter().filter(|item| item.is_some()).count() < 2 {
        return CulledPrimitives {
            visible: primitives,
            culled: 0,
        };
    }
    if !has_projected_tile_overlap(&projected, buffer_target) {
        return CulledPrimitives {
            visible: primitives,
            culled: 0,
        };
    }

    let mut order = projected
        .iter()
        .enumerate()
        .filter_map(|(index, projected)| projected.as_ref().map(|item| (index, item.min_depth)))
        .collect::<Vec<_>>();
    order.sort_by(|left, right| {
        left.1
            .total_cmp(&right.1)
            .then_with(|| left.0.cmp(&right.0))
    });

    let mut depth = vec![f32::INFINITY; buffer_target.pixel_len()];
    let mut culled = vec![false; primitives.len()];
    for (index, _) in order {
        let Some(projected) = projected[index].as_ref() else {
            continue;
        };
        if projected.is_occluded(&depth, buffer_target) {
            culled[index] = true;
        } else {
            projected.write_depth(&mut depth, buffer_target);
        }
    }

    let culled_count = culled.iter().filter(|item| **item).count() as u64;
    let visible = primitives
        .into_iter()
        .enumerate()
        .filter_map(|(index, primitive)| (!culled[index]).then_some(primitive))
        .collect();
    CulledPrimitives {
        visible,
        culled: culled_count,
    }
}

fn has_projected_tile_overlap(
    projected: &[Option<ProjectedPrimitive>],
    target: RasterTarget,
) -> bool {
    let width = target.width.max(1);
    let height = target.height.max(1);
    let mut occupied =
        [false; (OCCLUSION_OVERLAP_TILE_DIMENSION * OCCLUSION_OVERLAP_TILE_DIMENSION) as usize];
    for primitive in projected.iter().flatten() {
        let min_tile_x = primitive
            .min_x
            .saturating_mul(OCCLUSION_OVERLAP_TILE_DIMENSION)
            / width;
        let max_tile_x = primitive
            .max_x
            .saturating_mul(OCCLUSION_OVERLAP_TILE_DIMENSION)
            / width;
        let min_tile_y = primitive
            .min_y
            .saturating_mul(OCCLUSION_OVERLAP_TILE_DIMENSION)
            / height;
        let max_tile_y = primitive
            .max_y
            .saturating_mul(OCCLUSION_OVERLAP_TILE_DIMENSION)
            / height;
        for tile_y in min_tile_y..=max_tile_y.min(OCCLUSION_OVERLAP_TILE_DIMENSION - 1) {
            for tile_x in min_tile_x..=max_tile_x.min(OCCLUSION_OVERLAP_TILE_DIMENSION - 1) {
                let index = (tile_y * OCCLUSION_OVERLAP_TILE_DIMENSION + tile_x) as usize;
                if occupied[index] {
                    return true;
                }
                occupied[index] = true;
            }
        }
    }
    false
}

fn occlusion_buffer_target(target: RasterTarget) -> RasterTarget {
    let longest = target.width.max(target.height).max(1);
    if longest <= OCCLUSION_BUFFER_MAX_DIMENSION {
        return target;
    }
    let scale = longest as f32 / OCCLUSION_BUFFER_MAX_DIMENSION as f32;
    RasterTarget {
        width: ((target.width as f32 / scale).ceil() as u32).max(1),
        height: ((target.height as f32 / scale).ceil() as u32).max(1),
        backend: target.backend,
    }
}

#[derive(Debug, Clone, Copy)]
struct ProjectedPrimitive {
    vertices: [ScreenVertex; 3],
    min_x: u32,
    max_x: u32,
    min_y: u32,
    max_y: u32,
    area: f32,
    min_depth: f32,
}

impl ProjectedPrimitive {
    fn from_prepared(
        primitive: &PreparedPrimitive,
        camera: &CameraProjection,
        target: RasterTarget,
    ) -> Option<Self> {
        if !primitive.occlusion_culling_eligible() {
            return None;
        }
        let [a, b, c] = primitive.vertices();
        let vertices = [
            ScreenVertex::from_position(a.position, target, camera)?,
            ScreenVertex::from_position(b.position, target, camera)?,
            ScreenVertex::from_position(c.position, target, camera)?,
        ];
        let area = edge(vertices[0], vertices[1], vertices[2].x, vertices[2].y);
        if area.abs() <= f32::EPSILON || (!primitive.double_sided() && area < 0.0) {
            return None;
        }
        let min_x = vertices
            .iter()
            .map(|vertex| vertex.x)
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as u32;
        let max_x = vertices
            .iter()
            .map(|vertex| vertex.x)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .min(target.width.saturating_sub(1) as f32) as u32;
        let min_y = vertices
            .iter()
            .map(|vertex| vertex.y)
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as u32;
        let max_y = vertices
            .iter()
            .map(|vertex| vertex.y)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .min(target.height.saturating_sub(1) as f32) as u32;
        if min_x > max_x || min_y > max_y {
            return None;
        }
        Some(Self {
            vertices,
            min_x,
            max_x,
            min_y,
            max_y,
            area,
            min_depth: vertices
                .iter()
                .map(|vertex| vertex.depth)
                .fold(f32::INFINITY, f32::min),
        })
    }

    fn is_occluded(&self, depth: &[f32], target: RasterTarget) -> bool {
        let mut covered_samples = 0_u32;
        for y in self.min_y..=self.max_y {
            for x in self.min_x..=self.max_x {
                let Some(sample_depth) = self.depth_at_pixel(x, y) else {
                    continue;
                };
                covered_samples = covered_samples.saturating_add(1);
                if sample_depth <= depth[target.pixel_index(x, y)] + OCCLUSION_DEPTH_EPSILON {
                    return false;
                }
            }
        }
        covered_samples > 0
    }

    fn write_depth(&self, depth: &mut [f32], target: RasterTarget) {
        for y in self.min_y..=self.max_y {
            for x in self.min_x..=self.max_x {
                let Some(sample_depth) = self.depth_at_pixel(x, y) else {
                    continue;
                };
                let index = target.pixel_index(x, y);
                if sample_depth < depth[index] {
                    depth[index] = sample_depth;
                }
            }
        }
    }

    fn depth_at_pixel(&self, x: u32, y: u32) -> Option<f32> {
        let px = x as f32 + 0.5;
        let py = y as f32 + 0.5;
        let w0 = edge(self.vertices[1], self.vertices[2], px, py) / self.area;
        let w1 = edge(self.vertices[2], self.vertices[0], px, py) / self.area;
        let w2 = edge(self.vertices[0], self.vertices[1], px, py) / self.area;
        if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
            return None;
        }
        Some(
            self.vertices[0].depth * w0 + self.vertices[1].depth * w1 + self.vertices[2].depth * w2,
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct ScreenVertex {
    x: f32,
    y: f32,
    depth: f32,
}

impl ScreenVertex {
    fn from_position(
        position: crate::scene::Vec3,
        target: RasterTarget,
        camera: &CameraProjection,
    ) -> Option<Self> {
        let projected = camera.project(position)?;
        if projected.ndc_x < -1.0
            || projected.ndc_x > 1.0
            || projected.ndc_y < -1.0
            || projected.ndc_y > 1.0
        {
            return None;
        }
        let width = target.width.saturating_sub(1) as f32;
        let height = target.height.saturating_sub(1) as f32;
        Some(Self {
            x: (projected.ndc_x * 0.5 + 0.5) * width,
            y: (1.0 - (projected.ndc_y * 0.5 + 0.5)) * height,
            depth: projected.depth,
        })
    }
}

fn edge(a: ScreenVertex, b: ScreenVertex, x: f32, y: f32) -> f32 {
    (x - a.x) * (b.y - a.y) - (y - a.y) * (b.x - a.x)
}

fn outside_camera_clip_box(primitive: &PreparedPrimitive, camera: &CameraProjection) -> bool {
    let vertices = primitive.vertices();
    let projected = vertices.map(|vertex| camera.project(vertex.position));
    if projected.iter().all(Option::is_none) {
        return true;
    }
    let [Some(a), Some(b), Some(c)] = projected else {
        return false;
    };
    all(&[a, b, c], |coordinate| coordinate.ndc_x < -1.0)
        || all(&[a, b, c], |coordinate| coordinate.ndc_x > 1.0)
        || all(&[a, b, c], |coordinate| coordinate.ndc_y < -1.0)
        || all(&[a, b, c], |coordinate| coordinate.ndc_y > 1.0)
}

fn all<T>(items: &[T; 3], predicate: impl Fn(&T) -> bool) -> bool {
    predicate(&items[0]) && predicate(&items[1]) && predicate(&items[2])
}

#[cfg(test)]
mod tests {
    use crate::diagnostics::Backend;
    use crate::geometry::{Primitive, Vertex};
    use crate::material::Color;
    use crate::render::prepare::PreparedPrimitive;
    use crate::render::target::RasterTarget;
    use crate::scene::Vec3;

    use super::{
        CPU_OCCLUSION_MIN_PRIMITIVES, ProjectedPrimitive, ScreenVertex, cull_cpu_frustum,
        has_projected_tile_overlap, should_run_occlusion_prepass,
    };

    #[test]
    fn cpu_frustum_culling_without_camera_keeps_world_space_primitives() {
        let visible = PreparedPrimitive::new(Primitive::unlit_triangle(), None, Color::WHITE);
        let culled = PreparedPrimitive::new(
            Primitive::triangle([
                vertex(2.0, -0.5, 0.0),
                vertex(3.0, -0.5, 0.0),
                vertex(2.5, 0.5, 0.0),
            ]),
            None,
            Color::WHITE,
        );

        let result = cull_cpu_frustum(vec![visible.clone(), culled.clone()], None);

        assert_eq!(result.visible, vec![visible, culled]);
        assert_eq!(result.culled, 0);
    }

    #[test]
    fn gpu_path_uses_canonical_prepared_primitive_culling() {
        let visible = PreparedPrimitive::new(Primitive::unlit_triangle(), None, Color::WHITE);
        let culled = PreparedPrimitive::new(
            Primitive::triangle([
                vertex(2.0, -0.5, 0.0),
                vertex(3.0, -0.5, 0.0),
                vertex(2.5, 0.5, 0.0),
            ]),
            None,
            Color::WHITE,
        );

        let result = super::cull_prepared_primitives(
            vec![visible.clone(), culled.clone()],
            None,
            RasterTarget {
                width: 32,
                height: 32,
                backend: Backend::HeadlessGpu,
            },
            true,
            true,
        );

        assert_eq!(result.visible, vec![visible, culled]);
        assert_eq!(result.culled, 0);
    }

    #[test]
    fn pf10_occlusion_prepass_is_disabled_for_gpu_and_thresholded_for_cpu() {
        assert!(!should_run_occlusion_prepass(10_000, true));
        assert!(!should_run_occlusion_prepass(
            CPU_OCCLUSION_MIN_PRIMITIVES - 1,
            false
        ));
        assert!(should_run_occlusion_prepass(
            CPU_OCCLUSION_MIN_PRIMITIVES,
            false
        ));
    }

    #[test]
    fn pf10_occlusion_prepass_requires_projected_overlap() {
        let target = RasterTarget {
            width: 128,
            height: 128,
            backend: Backend::Headless,
        };
        let separated = [
            projected_bounds(4, 11, 4, 11),
            projected_bounds(20, 27, 4, 11),
        ];
        let overlapping = [
            projected_bounds(4, 11, 4, 11),
            projected_bounds(8, 15, 4, 11),
        ];

        assert!(!has_projected_tile_overlap(&separated, target));
        assert!(has_projected_tile_overlap(&overlapping, target));
    }

    fn projected_bounds(
        min_x: u32,
        max_x: u32,
        min_y: u32,
        max_y: u32,
    ) -> Option<ProjectedPrimitive> {
        Some(ProjectedPrimitive {
            vertices: [
                ScreenVertex {
                    x: min_x as f32,
                    y: min_y as f32,
                    depth: 0.5,
                },
                ScreenVertex {
                    x: max_x as f32,
                    y: min_y as f32,
                    depth: 0.5,
                },
                ScreenVertex {
                    x: min_x as f32,
                    y: max_y as f32,
                    depth: 0.5,
                },
            ],
            min_x,
            max_x,
            min_y,
            max_y,
            area: 1.0,
            min_depth: 0.5,
        })
    }

    fn vertex(x: f32, y: f32, z: f32) -> Vertex {
        Vertex {
            position: Vec3::new(x, y, z),
            color: Color::WHITE,
        }
    }
}
