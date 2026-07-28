use serde::{Deserialize, Serialize};

use super::{SceneHostCore, SceneHostError};
use crate::{
    AlphaMode, AssetFetcher, Color, GeometryDesc, GeometryTopology, GeometryVertex, MaterialDesc,
    NodeKey, ScreenSpaceAmbientOcclusionConfig, ScreenSpaceReflectionConfig, Transform, Vec3,
};

pub const PHOTOGRAPHIC_SURROUNDINGS_REPORT_SCHEMA_V1: &str =
    "scena.photographic_surroundings_report.v1";

pub(super) const GENERATED_SURROUNDING_TAG: &str = "scena_generated_photographic_surrounding";
const AUTHORED_SUPPORT_TAG: &str = "photographic_support";
const AUTHORED_BACKDROP_TAG: &str = "photographic_backdrop";
const AUTHORED_ROOM_TAG: &str = "photographic_room";
const SUSPENDED_TAG: &str = "photographic_suspended";
const WALL_MOUNTED_TAG: &str = "photographic_wall_mounted";
const ENVIRONMENT_SCALE_TAG: &str = "photographic_environment_scale";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotographicSurroundingsReportV1 {
    pub schema: String,
    pub source: String,
    pub subject: u64,
    pub support_class: String,
    pub support_height_m: Option<f32>,
    pub preserved_authored_surroundings: bool,
    pub preserved_authored_environment: bool,
    pub generated_floor: bool,
    pub generated_cyclorama: bool,
    pub generated_nodes: Vec<u64>,
    pub contact_shadow_nodes: Vec<u64>,
    pub grid_nodes: Vec<u64>,
    pub extent_m: f32,
    pub background_color: Color,
    pub background_luminance: f32,
    pub contact_shadow_strength: f32,
    pub reflection_strength: f32,
    pub reflection_roughness: f32,
    pub transient_render_only: bool,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn apply_photographic_surroundings(
        &mut self,
        subject: u64,
    ) -> Result<PhotographicSurroundingsReportV1, SceneHostError> {
        let subject_node = self.resolve_node(subject)?;
        let bounds = self
            .scene
            .node_world_bounds(subject_node, &self.assets)?
            .ok_or(crate::LookupError::ImportHasNoBounds)?;
        let subject_nodes = self.scene.subtree_nodes(subject_node)?;
        let material = subject_material_average(self, &subject_nodes);
        let derived_background = derived_background(material.mean_color, material.mean_luminance);
        let radius = bounds.bounding_sphere_radius().max(0.05);
        let camera_position = self
            .scene
            .camera_node(self.active_camera)
            .and_then(|node| self.scene.world_transform(node))
            .map(|transform| transform.translation)
            .unwrap_or(bounds.center() + Vec3::Z * radius * 4.0);
        let camera_distance = camera_position.distance(bounds.center());
        let extent_m = (radius * 5.0).max(camera_distance * 1.35).max(
            bounds
                .half_extent()
                .max_element()
                .mul_add(4.0, radius * 0.5),
        );

        let support_class = subject_support_class(self, subject_node, bounds, extent_m);
        let authored_support = authored_support_node(self, &subject_nodes, bounds);
        let authored_backdrop = self
            .scene
            .tagged(AUTHORED_BACKDROP_TAG)
            .chain(self.scene.tagged(AUTHORED_ROOM_TAG))
            .next()
            .is_some();
        // An environment the lighting solver derived is not an authored one, so
        // it must not suppress the generated cyclorama or the derived
        // background. This held before only because surroundings happened to run
        // first.
        let preserved_authored_environment = self.has_authored_environment();
        let preserved_authored_surroundings = authored_support.is_some() || authored_backdrop;
        let background_color = if authored_backdrop || preserved_authored_environment {
            self.renderer.background_color()
        } else {
            self.renderer.set_background_color(derived_background);
            derived_background
        };
        let background_luminance = linear_luminance(background_color);
        let support_height_m = if support_class == "supported" {
            authored_support
                .and_then(|node| {
                    self.scene
                        .node_world_bounds(node, &self.assets)
                        .ok()
                        .flatten()
                })
                .map_or(Some(bounds.min.y), |support| Some(support.max.y))
        } else {
            None
        };

        let mut generated_nodes = Vec::with_capacity(3);
        let generated_floor = support_height_m.is_some() && authored_support.is_none();
        if let Some(support_height) = support_height_m
            && generated_floor
        {
            let geometry = self
                .assets
                .create_geometry(GeometryDesc::plane(extent_m * 2.0, extent_m * 2.0));
            let floor_roughness = (0.72 + material.reflective_fraction * 0.18).clamp(0.72, 0.94);
            let floor_color = scale_color(background_color, 0.82);
            let material_handle = self.assets.create_material(
                MaterialDesc::pbr_metallic_roughness(floor_color, 0.0, floor_roughness)
                    .with_double_sided(true),
            );
            let node = self
                .scene
                .mesh(geometry, material_handle)
                .transform(Transform::at(Vec3::new(
                    bounds.center().x,
                    support_height,
                    bounds.center().z,
                )))
                .add()?;
            self.scene.add_tag(node, GENERATED_SURROUNDING_TAG)?;
            generated_nodes.push(self.register_node(node));
        }

        let generated_cyclorama = !authored_backdrop && !preserved_authored_environment;
        if generated_cyclorama {
            let support_height = support_height_m.unwrap_or(bounds.min.y - radius * 0.7);
            let geometry = self.assets.create_geometry(cyclorama_geometry(
                bounds.center(),
                camera_position,
                support_height,
                extent_m,
            ));
            let material_handle = self.assets.create_material(
                MaterialDesc::pbr_metallic_roughness(background_color, 0.0, 0.96)
                    .with_double_sided(true),
            );
            let node = self.scene.mesh(geometry, material_handle).add()?;
            self.scene.add_tag(node, GENERATED_SURROUNDING_TAG)?;
            generated_nodes.push(self.register_node(node));
        }

        let contact_shadow_strength = if support_height_m.is_some() {
            0.38
        } else {
            0.0
        };
        let mut contact_shadow_nodes = Vec::new();
        if let Some(support_height) = support_height_m {
            let footprint = bounds.half_extent();
            let geometry = self.assets.create_geometry(contact_shadow_geometry(
                footprint.x.max(radius * 0.08) * 1.08,
                footprint.z.max(radius * 0.08) * 1.08,
                contact_shadow_strength,
            ));
            let material = self.assets.create_material(
                MaterialDesc::unlit(Color::WHITE)
                    .with_alpha_mode(AlphaMode::Blend)
                    .with_double_sided(true),
            );
            let node = self
                .scene
                .mesh(geometry, material)
                .transform(Transform::at(Vec3::new(
                    bounds.center().x,
                    support_height + radius * 0.0005,
                    bounds.center().z,
                )))
                .add()?;
            self.scene.add_tag(node, GENERATED_SURROUNDING_TAG)?;
            let handle = self.register_node(node);
            generated_nodes.push(handle);
            contact_shadow_nodes.push(handle);
        }
        if contact_shadow_strength > 0.0 && self.renderer.screen_space_ambient_occlusion().is_none()
        {
            self.renderer.set_screen_space_ambient_occlusion(Some(
                ScreenSpaceAmbientOcclusionConfig::new(
                    5,
                    contact_shadow_strength,
                    (radius * 0.018).clamp(0.008, 0.08),
                ),
            ));
        }

        let reflection_strength = if support_height_m.is_some() {
            (material.reflective_fraction * 0.18).clamp(0.0, 0.20)
        } else {
            0.0
        };
        let reflection_roughness = (0.72 + material.mean_roughness * 0.22).clamp(0.72, 0.96);
        if reflection_strength > 0.035 && self.renderer.screen_space_reflections().is_none() {
            self.renderer
                .set_screen_space_reflections(Some(ScreenSpaceReflectionConfig::new(
                    reflection_strength,
                    reflection_roughness,
                    0.48,
                    0.72,
                )));
        }

        Ok(PhotographicSurroundingsReportV1 {
            schema: PHOTOGRAPHIC_SURROUNDINGS_REPORT_SCHEMA_V1.to_owned(),
            source: "subject_surroundings_solver".to_owned(),
            subject,
            support_class,
            support_height_m,
            preserved_authored_surroundings,
            preserved_authored_environment,
            generated_floor,
            generated_cyclorama,
            generated_nodes,
            contact_shadow_nodes,
            grid_nodes: Vec::new(),
            extent_m,
            background_color,
            background_luminance,
            contact_shadow_strength,
            reflection_strength,
            reflection_roughness,
            transient_render_only: true,
        })
    }

    pub fn remove_photographic_surroundings(
        &mut self,
        report: &PhotographicSurroundingsReportV1,
    ) -> Result<(), SceneHostError> {
        for handle in report.generated_nodes.iter().rev().copied() {
            let node = self.resolve_node(handle)?;
            if self.scene.has_tag(node, GENERATED_SURROUNDING_TAG) {
                self.scene.remove_node(node)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct SubjectMaterialAverage {
    mean_color: Color,
    mean_luminance: f32,
    reflective_fraction: f32,
    mean_roughness: f32,
}

fn subject_material_average<F: AssetFetcher>(
    host: &SceneHostCore<F>,
    subject_nodes: &[NodeKey],
) -> SubjectMaterialAverage {
    let inspection = host.scene.inspect_with_assets(&host.assets);
    let mut material_handles = Vec::new();
    for draw in inspection.draw_list() {
        if subject_nodes.contains(&draw.node()) && !material_handles.contains(&draw.material()) {
            material_handles.push(draw.material());
        }
    }
    let mut color = Vec3::ZERO;
    let mut reflective = 0.0;
    let mut roughness = 0.0;
    let mut count = 0.0;
    for handle in material_handles {
        let Some(material) = host.assets.material(handle) else {
            continue;
        };
        let base = material.base_color();
        color += Vec3::new(base.r, base.g, base.b);
        roughness += material.roughness_factor();
        reflective += material
            .metallic_factor()
            .max((1.0 - material.roughness_factor()) * 0.65)
            .max(material.transmission_factor());
        count += 1.0;
    }
    if count <= 0.0 {
        return SubjectMaterialAverage {
            mean_color: Color::GRAY,
            mean_luminance: linear_luminance(Color::GRAY),
            reflective_fraction: 0.0,
            mean_roughness: 1.0,
        };
    }
    color /= count;
    let mean_color = Color::from_linear_rgb(color.x, color.y, color.z);
    SubjectMaterialAverage {
        mean_color,
        mean_luminance: linear_luminance(mean_color),
        reflective_fraction: (reflective / count).clamp(0.0, 1.0),
        mean_roughness: (roughness / count).clamp(0.0, 1.0),
    }
}

fn subject_support_class<F: AssetFetcher>(
    host: &SceneHostCore<F>,
    subject: NodeKey,
    bounds: crate::Aabb,
    extent_m: f32,
) -> String {
    if host.scene.has_tag(subject, SUSPENDED_TAG) {
        return "suspended".to_owned();
    }
    if host.scene.has_tag(subject, WALL_MOUNTED_TAG) {
        return "wall_mounted".to_owned();
    }
    if host.scene.has_tag(subject, ENVIRONMENT_SCALE_TAG)
        || bounds.bounding_sphere_radius() > extent_m * 3.0
    {
        return "environment_scale".to_owned();
    }
    "supported".to_owned()
}

fn authored_support_node<F: AssetFetcher>(
    host: &SceneHostCore<F>,
    subject_nodes: &[NodeKey],
    subject_bounds: crate::Aabb,
) -> Option<NodeKey> {
    if let Some(node) = host.scene.tagged(AUTHORED_SUPPORT_TAG).next() {
        return Some(node);
    }
    let subject_size = subject_bounds.half_extent() * 2.0;
    host.scene.mesh_nodes().find_map(|(node, mesh, world)| {
        let geometry = host.assets.geometry(mesh.geometry())?;
        let local = geometry.bounds().half_extent() * 2.0;
        let world_size = local * world.scale.abs();
        let thin = world_size.y <= subject_size.y.max(0.01) * 0.04;
        let broad = world_size.x >= subject_size.x * 0.8 && world_size.z >= subject_size.z * 0.8;
        let near_base = host
            .scene
            .node_world_bounds(node, &host.assets)
            .ok()
            .flatten()
            .is_some_and(|bounds| {
                (bounds.max.y - subject_bounds.min.y).abs() <= subject_size.y.max(0.01) * 0.06
            });
        (thin && broad && near_base && !subject_nodes.is_empty()).then_some(node)
    })
}

fn derived_background(subject: Color, subject_luminance: f32) -> Color {
    let target_luminance = if subject_luminance < 0.18 {
        0.32
    } else if subject_luminance > 0.52 {
        0.16
    } else {
        0.10
    };
    let neutral = Vec3::splat(target_luminance);
    let subject_rgb = Vec3::new(subject.r, subject.g, subject.b);
    let complement = Vec3::ONE - subject_rgb.clamp(Vec3::ZERO, Vec3::ONE);
    let mut rgb = neutral * 0.88 + complement * target_luminance * 0.12;
    let luma = rgb.dot(Vec3::new(0.2126, 0.7152, 0.0722)).max(1.0e-5);
    rgb *= target_luminance / luma;
    rgb = rgb.clamp(Vec3::splat(0.025), Vec3::splat(0.62));
    Color::from_linear_rgb(rgb.x, rgb.y, rgb.z)
}

fn cyclorama_geometry(
    center: Vec3,
    camera_position: Vec3,
    support_height: f32,
    extent: f32,
) -> GeometryDesc {
    let toward_camera = (camera_position - center).with_y(0.0).normalize_or_zero();
    let toward_camera = if toward_camera.length_squared() > 1.0e-8 {
        toward_camera
    } else {
        Vec3::Z
    };
    let away = -toward_camera;
    let right = Vec3::Y.cross(away).normalize_or_zero();
    let half_width = extent;
    let curve_radius = extent * 0.18;
    let curve_start = center + away * extent * 0.54;
    let segments = 10usize;
    let mut vertices = Vec::with_capacity((segments + 2) * 2);
    for segment in 0..=segments {
        let angle = segment as f32 / segments as f32 * std::f32::consts::FRAC_PI_2;
        let row_center = curve_start
            + away * (curve_radius * angle.sin())
            + Vec3::Y * (support_height - center.y + curve_radius * (1.0 - angle.cos()));
        let normal = (toward_camera * angle.cos() + Vec3::Y * angle.sin()).normalize_or_zero();
        vertices.push(GeometryVertex {
            position: row_center - right * half_width,
            normal,
        });
        vertices.push(GeometryVertex {
            position: row_center + right * half_width,
            normal,
        });
    }
    let wall_center =
        curve_start + away * curve_radius + Vec3::Y * (support_height - center.y + extent * 1.8);
    vertices.push(GeometryVertex {
        position: wall_center - right * half_width,
        normal: toward_camera,
    });
    vertices.push(GeometryVertex {
        position: wall_center + right * half_width,
        normal: toward_camera,
    });
    let rows = segments + 2;
    let mut indices = Vec::with_capacity((rows - 1) * 6);
    for row in 0..rows - 1 {
        let base = (row * 2) as u32;
        indices.extend_from_slice(&[base, base + 2, base + 1, base + 1, base + 2, base + 3]);
    }
    GeometryDesc::try_new(GeometryTopology::Triangles, vertices, indices)
        .expect("generated cyclorama geometry is valid")
}

fn contact_shadow_geometry(radius_x: f32, radius_z: f32, opacity: f32) -> GeometryDesc {
    const SEGMENTS: usize = 32;
    let mut vertices = Vec::with_capacity(SEGMENTS + 1);
    let mut colors = Vec::with_capacity(SEGMENTS + 1);
    vertices.push(GeometryVertex {
        position: Vec3::ZERO,
        normal: Vec3::Y,
    });
    colors.push(Color::from_linear_rgba(
        0.0,
        0.0,
        0.0,
        opacity.clamp(0.0, 0.45),
    ));
    for segment in 0..SEGMENTS {
        let angle = segment as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        vertices.push(GeometryVertex {
            position: Vec3::new(angle.cos() * radius_x, 0.0, angle.sin() * radius_z),
            normal: Vec3::Y,
        });
        colors.push(Color::TRANSPARENT);
    }
    let mut indices = Vec::with_capacity(SEGMENTS * 3);
    for segment in 0..SEGMENTS {
        let current = segment as u32 + 1;
        let next = (segment + 1) as u32 % SEGMENTS as u32 + 1;
        indices.extend_from_slice(&[0, next, current]);
    }
    GeometryDesc::try_new_with_vertex_colors(GeometryTopology::Triangles, vertices, indices, colors)
        .expect("generated contact-shadow geometry is valid")
}

fn linear_luminance(color: Color) -> f32 {
    0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b
}

fn scale_color(color: Color, scale: f32) -> Color {
    Color::from_linear_rgb(color.r * scale, color.g * scale, color.b * scale)
}
