use std::collections::BTreeMap;

use crate::diagnostics::PrepareError;
use crate::geometry::{GeometryDesc, GeometryTopology, Primitive, Vertex};
use crate::material::{
    AlphaMode, Color, DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES, MaterialDesc, MaterialKind,
};
use crate::scene::{NodeKey, Vec3};

use super::super::RasterTarget;
use super::primitives::draw_uniform_tint;
use super::transforms::world_from_model_matrix;
use super::types::{PreparedPrimitive, PreparedStrokeSegment, PrimitiveBakeParams, PrimitiveSinks};

pub(super) struct StrokeBakeInputs<'a, 'out> {
    pub(super) tint: Option<Color>,
    pub(super) params: PrimitiveBakeParams<'a>,
    pub(super) sinks: PrimitiveSinks<'out>,
}

struct StrokeSegmentStyle {
    color: Color,
    tint: Option<Color>,
    width_px: f32,
    world_from_model: [f32; 16],
}

pub(super) fn append_wireframe_primitives(
    node: NodeKey,
    geometry: &GeometryDesc,
    material: &MaterialDesc,
    inputs: StrokeBakeInputs<'_, '_>,
) -> Result<(), PrepareError> {
    let (color, width_px) = technical_stroke_material(node, material)?;
    let width_px = scale_screen_space_width(width_px, inputs.params.screen_space_scale);
    let style = StrokeSegmentStyle {
        color,
        tint: inputs.tint,
        width_px,
        world_from_model: world_from_model_matrix(
            inputs.params.transform,
            inputs.params.origin_shift,
        ),
    };
    let vertices = geometry.vertices();
    for triangle in geometry.indices().chunks_exact(3) {
        for (start, end) in triangle_edges(triangle) {
            append_line_segment(
                node,
                vertices[start as usize].position,
                vertices[end as usize].position,
                &style,
                inputs.params.target,
                inputs.sinks.primitives,
                inputs.sinks.strokes,
            );
        }
    }
    Ok(())
}

pub(super) fn append_edge_primitives(
    node: NodeKey,
    geometry: &GeometryDesc,
    material: &MaterialDesc,
    inputs: StrokeBakeInputs<'_, '_>,
) -> Result<(), PrepareError> {
    let (color, width_px) = technical_stroke_material(node, material)?;
    let width_px = scale_screen_space_width(width_px, inputs.params.screen_space_scale);
    let style = StrokeSegmentStyle {
        color,
        tint: inputs.tint,
        width_px,
        world_from_model: world_from_model_matrix(
            inputs.params.transform,
            inputs.params.origin_shift,
        ),
    };
    let threshold = material
        .edge_angle_threshold_degrees()
        .unwrap_or(DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES);
    let vertices = geometry.vertices();
    for edge in visible_edge_segments(geometry, threshold) {
        append_line_segment(
            node,
            vertices[edge.start as usize].position,
            vertices[edge.end as usize].position,
            &style,
            inputs.params.target,
            inputs.sinks.primitives,
            inputs.sinks.strokes,
        );
    }
    Ok(())
}

fn visible_edge_segments(geometry: &GeometryDesc, threshold_degrees: f32) -> Vec<EdgeSegment> {
    let mut edges: BTreeMap<(PositionKey, PositionKey), EdgeCandidate> = BTreeMap::new();
    for triangle in geometry.indices().chunks_exact(3) {
        let normal = triangle_normal(geometry, triangle);
        for (start, end) in triangle_edges(triangle) {
            let key = ordered_position_edge_key(geometry, start, end);
            let endpoint_normals = EdgeEndpointNormals::from_edge(geometry, start, end);
            edges
                .entry(key)
                .and_modify(|edge| edge.add_face(normal, endpoint_normals))
                .or_insert_with(|| EdgeCandidate::new(start, end, normal, endpoint_normals));
        }
    }
    edges
        .values()
        .filter(|edge| edge.is_visible(threshold_degrees))
        .map(|edge| EdgeSegment {
            start: edge.start,
            end: edge.end,
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EdgeSegment {
    start: u32,
    end: u32,
}

pub(super) fn append_line_primitives(
    node: NodeKey,
    geometry: &GeometryDesc,
    material: &MaterialDesc,
    inputs: StrokeBakeInputs<'_, '_>,
) -> Result<(), PrepareError> {
    let (color, width_px) = line_material(node, material)?;
    let width_px = scale_screen_space_width(width_px, inputs.params.screen_space_scale);
    let style = StrokeSegmentStyle {
        color,
        tint: inputs.tint,
        width_px,
        world_from_model: world_from_model_matrix(
            inputs.params.transform,
            inputs.params.origin_shift,
        ),
    };
    let vertices = geometry.vertices();
    for segment in geometry.indices().chunks_exact(2) {
        append_line_segment(
            node,
            vertices[segment[0] as usize].position,
            vertices[segment[1] as usize].position,
            &style,
            inputs.params.target,
            inputs.sinks.primitives,
            inputs.sinks.strokes,
        );
    }
    Ok(())
}

fn technical_stroke_material(
    node: NodeKey,
    material: &MaterialDesc,
) -> Result<(Color, f32), PrepareError> {
    if !matches!(
        material.kind(),
        MaterialKind::Line | MaterialKind::Wireframe | MaterialKind::Edge
    ) {
        return Err(PrepareError::UnsupportedMaterialKind {
            node,
            kind: material.kind(),
        });
    }

    let mut color = material.base_color();
    match material.alpha_mode() {
        AlphaMode::Opaque => color.a = 1.0,
        AlphaMode::Mask { .. } | AlphaMode::Blend => {
            return Err(PrepareError::UnsupportedAlphaMode {
                node,
                alpha_mode: material.alpha_mode(),
            });
        }
    }
    Ok((color, material.stroke_width_px().unwrap_or(1.0)))
}

fn line_material(node: NodeKey, material: &MaterialDesc) -> Result<(Color, f32), PrepareError> {
    match material.kind() {
        MaterialKind::Line => {}
        MaterialKind::Unlit | MaterialKind::PbrMetallicRoughness => {
            return Err(PrepareError::UnsupportedGeometryTopology {
                node,
                topology: GeometryTopology::Lines,
            });
        }
        MaterialKind::Wireframe | MaterialKind::Edge => {
            return Err(PrepareError::UnsupportedMaterialKind {
                node,
                kind: material.kind(),
            });
        }
    }

    technical_stroke_material(node, material)
}

fn scale_screen_space_width(width_px: f32, scale: f32) -> f32 {
    width_px * scale.max(1.0)
}

fn append_line_segment(
    node: NodeKey,
    start: Vec3,
    end: Vec3,
    style: &StrokeSegmentStyle,
    target: RasterTarget,
    primitives: &mut Vec<PreparedPrimitive>,
    strokes: &mut Vec<PreparedStrokeSegment>,
) {
    strokes.push(PreparedStrokeSegment::new(
        Some(node),
        start,
        end,
        style.color,
        style.width_px,
        style.world_from_model,
        draw_uniform_tint(style.tint),
    ));

    let start = ScreenPoint::from_vec3(start, target);
    let end = ScreenPoint::from_vec3(end, target);
    let delta_x = end.x - start.x;
    let delta_y = end.y - start.y;
    let length = (delta_x * delta_x + delta_y * delta_y).sqrt();
    if length <= f32::EPSILON {
        return;
    }

    let half_width = style.width_px * 0.5;
    let normal_x = -delta_y / length * half_width;
    let normal_y = delta_x / length * half_width;
    let a = start.offset(normal_x, normal_y).to_vec3(target);
    let b = end.offset(normal_x, normal_y).to_vec3(target);
    let c = end.offset(-normal_x, -normal_y).to_vec3(target);
    let d = start.offset(-normal_x, -normal_y).to_vec3(target);

    primitives.push(
        PreparedPrimitive::new(
            Primitive::triangle([
                Vertex {
                    position: a,
                    color: style.color,
                },
                Vertex {
                    position: b,
                    color: style.color,
                },
                Vertex {
                    position: c,
                    color: style.color,
                },
            ])
            .without_depth_prepass(),
            Some(node),
            draw_uniform_tint(style.tint),
        )
        .without_semantic_attribution()
        .without_gpu_triangle_path(),
    );
    primitives.push(
        PreparedPrimitive::new(
            Primitive::triangle([
                Vertex {
                    position: a,
                    color: style.color,
                },
                Vertex {
                    position: c,
                    color: style.color,
                },
                Vertex {
                    position: d,
                    color: style.color,
                },
            ])
            .without_depth_prepass(),
            Some(node),
            draw_uniform_tint(style.tint),
        )
        .without_semantic_attribution()
        .without_gpu_triangle_path(),
    );
}

fn triangle_edges(triangle: &[u32]) -> [(u32, u32); 3] {
    [
        (triangle[0], triangle[1]),
        (triangle[1], triangle[2]),
        (triangle[2], triangle[0]),
    ]
}

fn ordered_position_edge_key(
    geometry: &GeometryDesc,
    start: u32,
    end: u32,
) -> (PositionKey, PositionKey) {
    let vertices = geometry.vertices();
    let start = PositionKey::from_vec3(vertices[start as usize].position);
    let end = PositionKey::from_vec3(vertices[end as usize].position);
    if start <= end {
        (start, end)
    } else {
        (end, start)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PositionKey([u32; 3]);

impl PositionKey {
    fn from_vec3(position: Vec3) -> Self {
        Self([
            scalar_position_key(position.x),
            scalar_position_key(position.y),
            scalar_position_key(position.z),
        ])
    }
}

fn scalar_position_key(value: f32) -> u32 {
    if value == 0.0 {
        0.0f32.to_bits()
    } else {
        value.to_bits()
    }
}

struct EdgeCandidate {
    start: u32,
    end: u32,
    first_normal: Vec3,
    second_normal: Option<Vec3>,
    endpoint_normals: EdgeEndpointNormals,
    smooth_vertex_normals: bool,
    face_count: u8,
}

impl EdgeCandidate {
    fn new(start: u32, end: u32, normal: Vec3, endpoint_normals: EdgeEndpointNormals) -> Self {
        Self {
            start,
            end,
            first_normal: normal,
            second_normal: None,
            endpoint_normals,
            smooth_vertex_normals: true,
            face_count: 1,
        }
    }

    fn add_face(&mut self, normal: Vec3, endpoint_normals: EdgeEndpointNormals) {
        self.face_count = self.face_count.saturating_add(1);
        if self.second_normal.is_none() {
            self.second_normal = Some(normal);
        }
        if !self.endpoint_normals.matches_smooth(endpoint_normals) {
            self.smooth_vertex_normals = false;
        }
    }

    fn is_visible(&self, threshold_degrees: f32) -> bool {
        if self.face_count != 2 {
            return true;
        }
        if self.smooth_vertex_normals {
            return false;
        }
        let Some(second_normal) = self.second_normal else {
            return true;
        };
        angle_degrees(self.first_normal, second_normal) > threshold_degrees
    }
}

#[derive(Clone, Copy)]
struct EdgeEndpointNormals {
    start_key: PositionKey,
    end_key: PositionKey,
    start_normal: Vec3,
    end_normal: Vec3,
}

impl EdgeEndpointNormals {
    fn from_edge(geometry: &GeometryDesc, start: u32, end: u32) -> Self {
        let vertices = geometry.vertices();
        let start_vertex = &vertices[start as usize];
        let end_vertex = &vertices[end as usize];
        let start_key = PositionKey::from_vec3(start_vertex.position);
        let end_key = PositionKey::from_vec3(end_vertex.position);
        if start_key <= end_key {
            Self {
                start_key,
                end_key,
                start_normal: start_vertex.normal,
                end_normal: end_vertex.normal,
            }
        } else {
            Self {
                start_key: end_key,
                end_key: start_key,
                start_normal: end_vertex.normal,
                end_normal: start_vertex.normal,
            }
        }
    }

    fn matches_smooth(self, other: Self) -> bool {
        self.start_key == other.start_key
            && self.end_key == other.end_key
            && normals_close(self.start_normal, other.start_normal)
            && normals_close(self.end_normal, other.end_normal)
    }
}

fn triangle_normal(geometry: &GeometryDesc, triangle: &[u32]) -> Vec3 {
    let vertices = geometry.vertices();
    let a = vertices[triangle[0] as usize].position;
    let b = vertices[triangle[1] as usize].position;
    let c = vertices[triangle[2] as usize].position;
    normalize(cross(sub(b, a), sub(c, a))).unwrap_or(vertices[triangle[0] as usize].normal)
}

fn angle_degrees(left: Vec3, right: Vec3) -> f32 {
    dot(left, right).clamp(-1.0, 1.0).acos().to_degrees()
}

fn sub(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

fn cross(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(
        left.y * right.z - left.z * right.y,
        left.z * right.x - left.x * right.z,
        left.x * right.y - left.y * right.x,
    )
}

fn dot(left: Vec3, right: Vec3) -> f32 {
    left.x * right.x + left.y * right.y + left.z * right.z
}

fn normalize(value: Vec3) -> Option<Vec3> {
    let length = dot(value, value).sqrt();
    (length > f32::EPSILON).then(|| Vec3::new(value.x / length, value.y / length, value.z / length))
}

fn normals_close(left: Vec3, right: Vec3) -> bool {
    dot(left, right) > 0.995
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{GeometryDesc, GeometryTopology, GeometryVertex};

    #[test]
    fn edge_segments_merge_duplicate_positions_and_hide_coplanar_triangulation_diagonal() {
        let z = Vec3::new(0.0, 0.0, 1.0);
        let vertices = vec![
            vertex(0.0, 0.0, z),
            vertex(1.0, 0.0, z),
            vertex(1.0, 1.0, z),
            vertex(0.0, 0.0, z),
            vertex(1.0, 1.0, z),
            vertex(0.0, 1.0, z),
        ];
        let geometry = GeometryDesc::try_new(
            GeometryTopology::Triangles,
            vertices,
            vec![0, 1, 2, 3, 4, 5],
        )
        .expect("duplicated-vertex quad geometry is valid");

        let segments = visible_edge_segments(&geometry, 18.0);

        assert_eq!(
            segments.len(),
            4,
            "CAD edge emphasis must keep only feature/boundary edges, not the duplicated triangle edge set: {segments:?}"
        );
        assert!(
            !segments.iter().any(|segment| {
                let a = geometry.vertices()[segment.start as usize].position;
                let b = geometry.vertices()[segment.end as usize].position;
                same_position_pair(a, b, Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0))
            }),
            "coplanar triangulation diagonal must not be drawn as a CAD edge: {segments:?}"
        );
    }

    #[test]
    fn edge_segments_hide_smooth_cylinder_facets_but_keep_cap_feature_rings() {
        let segments = 12;
        let geometry = GeometryDesc::cylinder(1.0, 2.0, segments);

        let edge_segments = visible_edge_segments(&geometry, 18.0);

        assert_eq!(
            edge_segments.len(),
            (segments * 2) as usize,
            "CAD edge emphasis must keep the two cap feature rings only, not side facet or cap triangulation edges: {edge_segments:?}"
        );
        assert!(
            !edge_segments.iter().any(|segment| is_vertical_side_edge(
                &geometry,
                segment.start,
                segment.end
            )),
            "smooth cylinder side tessellation edges must not be drawn as CAD feature edges: {edge_segments:?}"
        );
    }

    fn vertex(x: f32, y: f32, normal: Vec3) -> GeometryVertex {
        GeometryVertex {
            position: Vec3::new(x, y, 0.0),
            normal,
        }
    }

    fn same_position_pair(a: Vec3, b: Vec3, left: Vec3, right: Vec3) -> bool {
        (a.abs_diff_eq(left, 1.0e-6) && b.abs_diff_eq(right, 1.0e-6))
            || (a.abs_diff_eq(right, 1.0e-6) && b.abs_diff_eq(left, 1.0e-6))
    }

    fn is_vertical_side_edge(geometry: &GeometryDesc, start: u32, end: u32) -> bool {
        let vertices = geometry.vertices();
        let start = vertices[start as usize].position;
        let end = vertices[end as usize].position;
        (start.x - end.x).abs() < 1.0e-6
            && (start.z - end.z).abs() < 1.0e-6
            && (start.y - end.y).abs() > 1.0e-6
    }
}

#[derive(Clone, Copy)]
struct ScreenPoint {
    x: f32,
    y: f32,
    z: f32,
}

impl ScreenPoint {
    fn from_vec3(position: Vec3, target: RasterTarget) -> Self {
        let width = target.width.saturating_sub(1) as f32;
        let height = target.height.saturating_sub(1) as f32;
        Self {
            x: (position.x * 0.5 + 0.5) * width,
            y: (1.0 - (position.y * 0.5 + 0.5)) * height,
            z: position.z,
        }
    }

    fn offset(self, x: f32, y: f32) -> Self {
        Self {
            x: self.x + x,
            y: self.y + y,
            z: self.z,
        }
    }

    fn to_vec3(self, target: RasterTarget) -> Vec3 {
        Vec3::new(
            screen_x_to_ndc(self.x, target),
            screen_y_to_ndc(self.y, target),
            self.z,
        )
    }
}

fn screen_x_to_ndc(x: f32, target: RasterTarget) -> f32 {
    if target.width <= 1 {
        0.0
    } else {
        (x / target.width.saturating_sub(1) as f32 - 0.5) * 2.0
    }
}

fn screen_y_to_ndc(y: f32, target: RasterTarget) -> f32 {
    if target.height <= 1 {
        0.0
    } else {
        ((1.0 - y / target.height.saturating_sub(1) as f32) - 0.5) * 2.0
    }
}
