use crate::geometry::{PrimitiveVertexAttributes, Vertex};
use crate::material::Color;
use crate::scene::Vec3;

use super::RasterTarget;
use super::camera::{CameraProjection, ProjectedVertex};
use super::prepare::PreparedPrimitive;

const MAX_CLIPPED_VERTICES: usize = 5;
const MAX_CLIPPED_TRIANGLES: usize = MAX_CLIPPED_VERTICES - 2;

#[derive(Debug, Clone, Copy)]
struct ClipVertex {
    vertex: Vertex,
    attributes: PrimitiveVertexAttributes,
    view_depth: f32,
}

impl Default for ClipVertex {
    fn default() -> Self {
        Self {
            vertex: Vertex {
                position: Vec3::ZERO,
                color: Color::BLACK,
            },
            attributes: PrimitiveVertexAttributes::default(),
            view_depth: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CpuScreenVertex {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) projected: ProjectedVertex,
    pub(super) position: Vec3,
    pub(super) color: Color,
    pub(super) attributes: PrimitiveVertexAttributes,
}

impl Default for CpuScreenVertex {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            projected: ProjectedVertex {
                ndc_x: 0.0,
                ndc_y: 0.0,
                depth: 0.0,
                view_depth: 1.0,
            },
            position: Vec3::ZERO,
            color: Color::BLACK,
            attributes: PrimitiveVertexAttributes::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct CpuScreenTriangle {
    vertices: [CpuScreenVertex; 3],
}

impl CpuScreenTriangle {
    pub(super) const fn vertices(&self) -> [CpuScreenVertex; 3] {
        self.vertices
    }

    pub(super) fn row_bounds(&self, target: RasterTarget) -> Option<(u32, u32)> {
        let [a, b, c] = self.vertices;
        let min = a.y.min(b.y).min(c.y).floor().max(0.0) as u32;
        let max =
            a.y.max(b.y)
                .max(c.y)
                .ceil()
                .min(target.height.saturating_sub(1) as f32) as u32;
        (min <= max).then_some((min, max))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct CpuProjectedPrimitive {
    triangles: [CpuScreenTriangle; MAX_CLIPPED_TRIANGLES],
    triangle_count: u8,
    row_bounds: Option<(u32, u32)>,
}

impl CpuProjectedPrimitive {
    pub(super) fn triangles(&self) -> &[CpuScreenTriangle] {
        &self.triangles[..usize::from(self.triangle_count)]
    }

    pub(super) const fn row_bounds(&self) -> Option<(u32, u32)> {
        self.row_bounds
    }
}

pub(super) fn project_clipped_primitive(
    primitive: &PreparedPrimitive,
    target: RasterTarget,
    camera: &CameraProjection,
) -> CpuProjectedPrimitive {
    project_clipped_triangle(
        *primitive.vertices(),
        *primitive.vertex_attributes(),
        target,
        camera,
    )
}

pub(super) fn project_clipped_triangle(
    vertices: [Vertex; 3],
    attributes: [PrimitiveVertexAttributes; 3],
    target: RasterTarget,
    camera: &CameraProjection,
) -> CpuProjectedPrimitive {
    let mut polygon = [ClipVertex::default(); MAX_CLIPPED_VERTICES];
    for index in 0..3 {
        let Some(view_depth) = camera.camera_depth(vertices[index].position) else {
            return CpuProjectedPrimitive::default();
        };
        if !view_depth.is_finite() {
            return CpuProjectedPrimitive::default();
        }
        polygon[index] = ClipVertex {
            vertex: vertices[index],
            attributes: attributes[index],
            view_depth,
        };
    }
    let mut polygon_len = 3;
    let mut scratch = [ClipVertex::default(); MAX_CLIPPED_VERTICES];
    let [near, far] = camera.near_far();
    polygon_len = clip_depth_plane(&polygon, polygon_len, &mut scratch, near, true);
    if polygon_len < 3 {
        return CpuProjectedPrimitive::default();
    }
    polygon[..polygon_len].copy_from_slice(&scratch[..polygon_len]);
    polygon_len = clip_depth_plane(&polygon, polygon_len, &mut scratch, far, false);
    if polygon_len < 3 {
        return CpuProjectedPrimitive::default();
    }

    let mut projected = [CpuScreenVertex::default(); MAX_CLIPPED_VERTICES];
    for index in 0..polygon_len {
        let clipped = scratch[index];
        let Some(vertex) = camera.project_clipped(clipped.vertex.position, clipped.view_depth)
        else {
            return CpuProjectedPrimitive::default();
        };
        projected[index] = CpuScreenVertex {
            x: (vertex.ndc_x * 0.5 + 0.5) * target.width.saturating_sub(1) as f32,
            y: (1.0 - (vertex.ndc_y * 0.5 + 0.5)) * target.height.saturating_sub(1) as f32,
            projected: vertex,
            position: clipped.vertex.position,
            color: clipped.vertex.color,
            attributes: clipped.attributes,
        };
    }

    let mut output = CpuProjectedPrimitive::default();
    for triangle_index in 0..polygon_len - 2 {
        let triangle = CpuScreenTriangle {
            vertices: [
                projected[0],
                projected[triangle_index + 1],
                projected[triangle_index + 2],
            ],
        };
        output.triangles[triangle_index] = triangle;
        output.triangle_count += 1;
        if let Some((min, max)) = triangle.row_bounds(target) {
            output.row_bounds = Some(output.row_bounds.map_or((min, max), |(old_min, old_max)| {
                (old_min.min(min), old_max.max(max))
            }));
        }
    }
    output
}

fn clip_depth_plane(
    input: &[ClipVertex; MAX_CLIPPED_VERTICES],
    input_len: usize,
    output: &mut [ClipVertex; MAX_CLIPPED_VERTICES],
    plane_depth: f32,
    keep_greater: bool,
) -> usize {
    if input_len == 0 || !plane_depth.is_finite() {
        return 0;
    }
    let inside = |depth: f32| {
        if keep_greater {
            depth >= plane_depth
        } else {
            depth <= plane_depth
        }
    };
    let mut output_len = 0;
    let mut previous = input[input_len - 1];
    let mut previous_inside = inside(previous.view_depth);
    for current in input.iter().copied().take(input_len) {
        let current_inside = inside(current.view_depth);
        if current_inside != previous_inside {
            if output_len >= MAX_CLIPPED_VERTICES {
                return 0;
            }
            output[output_len] = intersect_depth(previous, current, plane_depth);
            output_len += 1;
        }
        if current_inside {
            if output_len >= MAX_CLIPPED_VERTICES {
                return 0;
            }
            output[output_len] = current;
            output_len += 1;
        }
        previous = current;
        previous_inside = current_inside;
    }
    output_len
}

fn intersect_depth(start: ClipVertex, end: ClipVertex, plane_depth: f32) -> ClipVertex {
    let denominator = end.view_depth - start.view_depth;
    let t = if denominator.abs() <= f32::EPSILON {
        0.0
    } else {
        ((plane_depth - start.view_depth) / denominator).clamp(0.0, 1.0)
    };
    ClipVertex {
        vertex: Vertex {
            position: mix_vec3(start.vertex.position, end.vertex.position, t),
            color: mix_color(start.vertex.color, end.vertex.color, t),
        },
        attributes: PrimitiveVertexAttributes {
            normal: mix_vec3(start.attributes.normal, end.attributes.normal, t),
            tex_coord0: [
                mix_f32(
                    start.attributes.tex_coord0[0],
                    end.attributes.tex_coord0[0],
                    t,
                ),
                mix_f32(
                    start.attributes.tex_coord0[1],
                    end.attributes.tex_coord0[1],
                    t,
                ),
            ],
            tangent: mix_vec3(start.attributes.tangent, end.attributes.tangent, t),
            tangent_handedness: mix_f32(
                start.attributes.tangent_handedness,
                end.attributes.tangent_handedness,
                t,
            ),
            shadow_visibility: mix_f32(
                start.attributes.shadow_visibility,
                end.attributes.shadow_visibility,
                t,
            ),
        },
        view_depth: plane_depth,
    }
}

pub(super) fn edge(a: CpuScreenVertex, b: CpuScreenVertex, x: f32, y: f32) -> f32 {
    (x - a.x) * (b.y - a.y) - (y - a.y) * (b.x - a.x)
}

pub(super) fn perspective_weights(
    camera: &CameraProjection,
    vertices: [CpuScreenVertex; 3],
    affine: [f32; 3],
) -> [f32; 3] {
    let projected = [
        vertices[0].projected,
        vertices[1].projected,
        vertices[2].projected,
    ];
    camera.interpolation_weights(projected, affine)
}

pub(super) fn weighted_vec3(vertices: [Vec3; 3], weights: [f32; 3]) -> Vec3 {
    vertices[0] * weights[0] + vertices[1] * weights[1] + vertices[2] * weights[2]
}

fn mix_vec3(start: Vec3, end: Vec3, t: f32) -> Vec3 {
    start + (end - start) * t
}

fn mix_color(start: Color, end: Color, t: f32) -> Color {
    Color::from_linear_rgba(
        mix_f32(start.r, end.r, t),
        mix_f32(start.g, end.g, t),
        mix_f32(start.b, end.b, t),
        mix_f32(start.a, end.a, t),
    )
}

fn mix_f32(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Backend;
    use crate::scene::{DepthRange, PerspectiveCamera, Scene, Transform};

    fn projection() -> (Scene, crate::scene::CameraKey, RasterTarget) {
        let mut scene = Scene::new();
        let camera = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::default().with_depth_range(DepthRange::new(0.5, 5.0)),
                Transform::default(),
            )
            .expect("camera inserts");
        (
            scene,
            camera,
            RasterTarget {
                width: 96,
                height: 96,
                backend: Backend::Headless,
            },
        )
    }

    #[test]
    fn clipped_intersections_interpolate_the_complete_vertex_payload() {
        let (scene, camera, target) = projection();
        let camera = CameraProjection::from_scene(&scene, camera, target).expect("projection");
        let vertices = [
            vertex(
                Vec3::new(-0.4, -0.4, -1.0),
                Color::from_linear_rgb(1.0, 0.0, 0.0),
            ),
            vertex(
                Vec3::new(0.4, -0.4, -1.0),
                Color::from_linear_rgb(0.0, 1.0, 0.0),
            ),
            vertex(
                Vec3::new(0.0, 0.4, -0.25),
                Color::from_linear_rgb(0.0, 0.0, 1.0),
            ),
        ];
        let attributes = [attributes(0.0), attributes(1.0), attributes(3.0)];

        let clipped = project_clipped_triangle(vertices, attributes, target, &camera);
        assert_eq!(clipped.triangles().len(), 2);
        let near_vertices = clipped
            .triangles()
            .iter()
            .flat_map(|triangle| triangle.vertices())
            .filter(|vertex| (vertex.projected.view_depth - 0.5).abs() < 1.0e-5)
            .collect::<Vec<_>>();
        assert!(near_vertices.len() >= 2);
        let mut clipped_attribute_values = near_vertices
            .iter()
            .map(|vertex| vertex.attributes.tex_coord0[0])
            .collect::<Vec<_>>();
        clipped_attribute_values.sort_by(f32::total_cmp);
        clipped_attribute_values.dedup_by(|left, right| (*left - *right).abs() < 1.0e-5);
        assert_eq!(clipped_attribute_values.len(), 2);
        assert_approx(clipped_attribute_values[0], 2.0);
        assert_approx(clipped_attribute_values[1], 7.0 / 3.0);
        for vertex in near_vertices {
            let value = vertex.attributes.tex_coord0[0];
            assert!(vertex.position.is_finite());
            if (value - 2.0).abs() < 1.0e-5 {
                assert_approx(vertex.color.r, 1.0 / 3.0);
                assert_approx(vertex.color.g, 0.0);
                assert_approx(vertex.color.b, 2.0 / 3.0);
            } else {
                assert_approx(value, 7.0 / 3.0);
                assert_approx(vertex.color.r, 0.0);
                assert_approx(vertex.color.g, 1.0 / 3.0);
                assert_approx(vertex.color.b, 2.0 / 3.0);
            }
            assert_approx(vertex.color.a, 1.0);
            assert!(vertex.attributes.normal.is_finite());
            assert!(vertex.attributes.tangent.is_finite());
            assert!(
                vertex
                    .attributes
                    .tex_coord0
                    .iter()
                    .all(|value| value.is_finite())
            );
            assert!(vertex.attributes.tangent_handedness.is_finite());
            assert!(vertex.attributes.shadow_visibility.is_finite());
            assert!(vertex.attributes.tex_coord0[0] > 0.0);
            assert!(vertex.attributes.tex_coord0[0] < 3.0);
            assert_approx(vertex.attributes.normal.x, value);
            assert_approx(vertex.attributes.normal.y, value + 1.0);
            assert_approx(vertex.attributes.normal.z, value + 2.0);
            assert_approx(vertex.attributes.tex_coord0[1], value + 0.5);
            assert_approx(vertex.attributes.tangent.x, value + 3.0);
            assert_approx(vertex.attributes.tangent.y, value + 4.0);
            assert_approx(vertex.attributes.tangent.z, value + 5.0);
            assert_approx(vertex.attributes.tangent_handedness, value + 6.0);
            assert_approx(vertex.attributes.shadow_visibility, value + 7.0);
        }
    }

    #[test]
    fn clipping_preserves_source_winding_for_every_generated_triangle() {
        let (scene, camera, target) = projection();
        let camera = CameraProjection::from_scene(&scene, camera, target).expect("projection");
        let clipped = project_clipped_triangle(
            [
                vertex(Vec3::new(-0.4, -0.4, -1.0), Color::WHITE),
                vertex(Vec3::new(0.4, -0.4, -1.0), Color::WHITE),
                vertex(Vec3::new(0.0, 0.4, -0.25), Color::WHITE),
            ],
            [PrimitiveVertexAttributes::default(); 3],
            target,
            &camera,
        );
        let signs = clipped
            .triangles()
            .iter()
            .map(|triangle| {
                let [a, b, c] = triangle.vertices();
                edge(a, b, c.x, c.y).signum()
            })
            .collect::<Vec<_>>();
        assert!(signs.len() > 1);
        assert!(signs.iter().all(|sign| *sign == signs[0] && *sign != 0.0));
    }

    #[test]
    fn fully_visible_triangles_preserve_the_direct_projection_bits() {
        let mut scene = Scene::new();
        let camera = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::default(),
                Transform::at(Vec3::new(0.0, 0.0, 1.732_050_8)),
            )
            .expect("camera inserts");
        let target = RasterTarget {
            width: 16,
            height: 16,
            backend: Backend::Headless,
        };
        let camera = CameraProjection::from_scene(&scene, camera, target).expect("projection");
        let vertices = [
            vertex(Vec3::new(-1.0, -1.0, 0.0), Color::WHITE),
            vertex(Vec3::new(3.0, -1.0, 0.0), Color::WHITE),
            vertex(Vec3::new(-1.0, 3.0, 0.0), Color::WHITE),
        ];
        let projected = project_clipped_triangle(
            vertices,
            [PrimitiveVertexAttributes::default(); 3],
            target,
            &camera,
        );
        let [triangle] = projected.triangles() else {
            panic!("fully visible triangle projects without clipping")
        };

        for (actual, source) in triangle.vertices().into_iter().zip(vertices) {
            let expected = camera.project(source.position).expect("source projects");
            assert_eq!(actual.projected.ndc_x.to_bits(), expected.ndc_x.to_bits());
            assert_eq!(actual.projected.ndc_y.to_bits(), expected.ndc_y.to_bits());
            assert_eq!(actual.projected.depth.to_bits(), expected.depth.to_bits());
            assert_eq!(
                actual.projected.view_depth.to_bits(),
                expected.view_depth.to_bits()
            );
        }
    }

    fn vertex(position: Vec3, color: Color) -> Vertex {
        Vertex { position, color }
    }

    fn attributes(value: f32) -> PrimitiveVertexAttributes {
        PrimitiveVertexAttributes {
            normal: Vec3::new(value, value + 1.0, value + 2.0),
            tex_coord0: [value, value + 0.5],
            tangent: Vec3::new(value + 3.0, value + 4.0, value + 5.0),
            tangent_handedness: value + 6.0,
            shadow_visibility: value + 7.0,
        }
    }

    fn assert_approx(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1.0e-5,
            "expected {expected}, got {actual}"
        );
    }
}
