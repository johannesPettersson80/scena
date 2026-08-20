mod backdrop;
mod ground;
mod material;
mod planar_reflection;

use backdrop::{
    BACKDROP_HALF_WIDTH_FRACTION, BackdropCamera, BackdropPlane, cyclorama_geometry,
    cyclorama_wall_cover_geometry, horizontal_toward_camera, surroundings_extent,
};
use ground::{contact_shadow_geometry, photographic_floor_geometry};
use material::subject_material_average;
use serde::{Deserialize, Serialize};

pub use planar_reflection::PhotographicPlanarReflectionCaptureV1;

use super::{SceneHostCore, SceneHostError};
use crate::{
    AlphaMode, AssetFetcher, Camera, Color, MaterialDesc, NodeKey,
    ScreenSpaceAmbientOcclusionConfig, ScreenSpaceReflectionConfig, TextureMemoryDesc,
    TextureMemoryId, TextureSlot, TextureTransform, Transform, Vec3,
};

pub const PHOTOGRAPHIC_SURROUNDINGS_REPORT_SCHEMA_V1: &str =
    "scena.photographic_surroundings_report.v1";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhotographicGroundV1 {
    #[default]
    Matte,
    Reflective,
}

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
    #[serde(default)]
    pub ground: PhotographicGroundV1,
    pub support_class: String,
    pub support_height_m: Option<f32>,
    pub preserved_authored_surroundings: bool,
    pub preserved_authored_environment: bool,
    pub generated_floor: bool,
    pub generated_cyclorama: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub support_nodes: Vec<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub backdrop_nodes: Vec<u64>,
    pub generated_nodes: Vec<u64>,
    pub contact_shadow_nodes: Vec<u64>,
    pub grid_nodes: Vec<u64>,
    pub extent_m: f32,
    pub background_color: Color,
    pub background_luminance: f32,
    pub contact_shadow_strength: f32,
    pub reflection_strength: f32,
    pub reflection_roughness: f32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub planar_reflection_capture_count: u32,
    pub transient_render_only: bool,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn apply_photographic_surroundings(
        &mut self,
        subject: u64,
    ) -> Result<PhotographicSurroundingsReportV1, SceneHostError> {
        self.apply_photographic_surroundings_with_ground(subject, PhotographicGroundV1::Matte)
    }

    pub fn apply_photographic_surroundings_with_ground(
        &mut self,
        subject: u64,
        ground: PhotographicGroundV1,
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
        let camera_rotation = self
            .scene
            .camera_node(self.active_camera)
            .and_then(|node| self.scene.world_transform(node))
            .map(|transform| transform.rotation);
        let viewport_size = {
            let (width, height) = self.viewport_size();
            (width as f32, height as f32)
        };
        // The same basis `cyclorama_geometry` builds the wall in, so the solve
        // and the geometry cannot disagree about where the backdrop stands.
        let backdrop_plane = BackdropPlane {
            center: bounds.center(),
            toward_camera: horizontal_toward_camera(bounds.center(), camera_position),
            right: Vec3::Y
                .cross(-horizontal_toward_camera(bounds.center(), camera_position))
                .normalize_or_zero(),
            // The wall's base is the support height, which is only resolved
            // after the extent. Assume the lower of the two possible bases: a
            // lower base puts the wall's top edge lower, which is the case that
            // needs the most extent to still cover the frame.
            floor_y: bounds.min.y - radius * 0.7,
        };
        let extent_m = surroundings_extent(
            radius,
            camera_distance,
            bounds.half_extent().max_element(),
            backdrop_plane,
            camera_rotation.and_then(|rotation| {
                self.scene
                    .camera(self.active_camera)
                    .and_then(|camera| match camera {
                        // An orthographic camera has no frustum divergence, so
                        // the frustum solve does not apply and the subject floor
                        // governs.
                        Camera::Orthographic(_) => None,
                        Camera::Perspective(perspective) => {
                            // `aspect` is 0 when it follows the viewport, so
                            // resolve it the way the view pipeline does rather
                            // than clamping a sentinel into a real number.
                            let aspect = if perspective.aspect > 0.0 {
                                perspective.aspect
                            } else {
                                let (width, height) = viewport_size;
                                if height > 0.0 { width / height } else { 1.0 }
                            };
                            Some(BackdropCamera {
                                position: camera_position,
                                rotation,
                                vertical_fov: perspective.vertical_fov.radians(),
                                aspect,
                            })
                        }
                    })
            }),
        );

        let support_class = subject_support_class(self, subject_node, bounds, extent_m);
        let authored_support = authored_support_node(self, &subject_nodes, bounds);
        let authored_backdrop_node = self
            .scene
            .tagged(AUTHORED_BACKDROP_TAG)
            .chain(self.scene.tagged(AUTHORED_ROOM_TAG))
            .next();
        let authored_backdrop = authored_backdrop_node.is_some();
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

        let mut generated_nodes = Vec::with_capacity(4);
        let mut support_nodes = authored_support
            .map(|node| self.register_node(node))
            .into_iter()
            .collect::<Vec<_>>();
        let mut backdrop_nodes = authored_backdrop_node
            .map(|node| self.register_node(node))
            .into_iter()
            .collect::<Vec<_>>();
        let generated_floor = support_height_m.is_some() && authored_support.is_none();
        let generated_cyclorama = !authored_backdrop && !preserved_authored_environment;
        let generated_floor_material = if generated_floor {
            let material = match ground {
                PhotographicGroundV1::Matte => {
                    let normal = matte_ground_normal_texture(self)?;
                    MaterialDesc::pbr_metallic_roughness(background_color, 0.0, 0.96)
                        .with_normal_texture(normal)
                        .with_normal_texture_transform(TextureTransform::new(
                            [0.0, 0.0],
                            0.0,
                            [12.0, 12.0],
                        ))
                        .with_normal_scale(0.18)
                }
                PhotographicGroundV1::Reflective => {
                    MaterialDesc::pbr_metallic_roughness(background_color, 0.0, 0.34)
                }
            };
            Some(
                self.assets
                    .create_material(material.with_double_sided(true)),
            )
        } else {
            None
        };
        let generated_sweep_material = generated_cyclorama.then(|| {
            self.assets.create_material(
                MaterialDesc::pbr_metallic_roughness(background_color, 0.0, 0.96)
                    .with_double_sided(true),
            )
        });
        if let Some(support_height) = support_height_m
            && generated_floor
        {
            let geometry = self.assets.create_geometry(photographic_floor_geometry(
                extent_m * BACKDROP_HALF_WIDTH_FRACTION,
            ));
            let material_handle =
                generated_floor_material.expect("generated floor material exists");
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
            let handle = self.register_node(node);
            generated_nodes.push(handle);
            support_nodes.push(handle);
        }

        if generated_cyclorama {
            let support_height = support_height_m.unwrap_or(bounds.min.y - radius * 0.7);
            let geometry = self.assets.create_geometry(cyclorama_geometry(
                bounds.center(),
                camera_position,
                support_height,
                extent_m,
                false,
            ));
            let material_handle =
                generated_sweep_material.expect("generated cyclorama material exists");
            let node = self.scene.mesh(geometry, material_handle).add()?;
            self.scene.add_tag(node, GENERATED_SURROUNDING_TAG)?;
            let handle = self.register_node(node);
            generated_nodes.push(handle);
            backdrop_nodes.push(handle);

            let wall_geometry = self.assets.create_geometry(cyclorama_wall_cover_geometry(
                bounds.center(),
                camera_position,
                support_height,
                extent_m,
            ));
            let wall_material = self.assets.create_material(
                MaterialDesc::pbr_metallic_roughness(background_color, 0.0, 0.96)
                    .with_double_sided(true),
            );
            let wall_node = self.scene.mesh(wall_geometry, wall_material).add()?;
            self.scene.add_tag(wall_node, GENERATED_SURROUNDING_TAG)?;
            let wall_handle = self.register_node(wall_node);
            generated_nodes.push(wall_handle);
            backdrop_nodes.push(wall_handle);
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
        if contact_shadow_strength > 0.0
            && self.renderer.screen_space_ambient_occlusion().is_none()
            && !debug_disabled("SCENA_DEBUG_DISABLE_SSAO")
        {
            self.renderer.set_screen_space_ambient_occlusion(Some(
                ScreenSpaceAmbientOcclusionConfig::new(
                    5,
                    contact_shadow_strength,
                    (radius * 0.018).clamp(0.008, 0.08),
                ),
            ));
        }

        let reflection_strength = match (ground, support_height_m) {
            (PhotographicGroundV1::Reflective, Some(_)) => 0.28,
            _ => 0.0,
        };
        let reflection_roughness = match ground {
            PhotographicGroundV1::Matte => 0.96,
            PhotographicGroundV1::Reflective => 0.34,
        };
        // The photographic path no longer enables the screen-space reflection
        // pass, because that pass does not compute reflections. It picks a
        // horizontal scanline at a fixed fraction of the *frame* - nothing to do
        // with where the floor is - mirrors every pixel below it about that
        // line, and blends the flipped copy in, using `1 - luma()` as its guess
        // at "is this floor". It has no depth buffer, no surface normal and no
        // ray march, so a subject's own lower body receives an upside-down copy
        // of its upper body. That is the translucent duplicate visible over the
        // baseplate in every render this staging produced.
        //
        // Disabling it here is deliberate and narrow: the setting stays on the
        // renderer for anyone who wants that stylised effect, and the strength
        // the solver computed is still reported, so the decision is visible
        // rather than silently dropped. A real screen-space reflection needs the
        // depth buffer and a ray march against it, which is its own piece of
        // work, not a tweak to this one.
        let _ = (reflection_strength, reflection_roughness);
        if debug_enabled("SCENA_DEBUG_ENABLE_MIRROR_SSR")
            && reflection_strength > 0.035
            && self.renderer.screen_space_reflections().is_none()
        {
            self.renderer
                .set_screen_space_reflections(Some(ScreenSpaceReflectionConfig::new(
                    reflection_strength,
                    reflection_roughness,
                    SCREEN_SPACE_REFLECTION_HORIZON_FRACTION,
                    0.72,
                )));
        }
        if debug_enabled("SCENA_DEBUG_LOG_STAGING") {
            let (viewport_width, viewport_height) = self.viewport_size();
            let horizon_row =
                SCREEN_SPACE_REFLECTION_HORIZON_FRACTION * viewport_height.max(1) as f32;
            eprintln!(
                "[staging] viewport={viewport_width}x{viewport_height} extent_m={extent_m:.4} \
                 support_class={support_class} floor={generated_floor} \
                 cyclorama={generated_cyclorama} contact_shadow={contact_shadow_strength:.3}"
            );
            eprintln!(
                "[staging] ssr strength={reflection_strength:.4} roughness={reflection_roughness:.4} \
                 horizon_fraction={SCREEN_SPACE_REFLECTION_HORIZON_FRACTION} \
                 -> mirror line at row {horizon_row:.1} of {viewport_height}; \
                 every row below it is blended with the row mirrored about it \
                 (blur radius {} px)",
                (reflection_roughness * 8.0).round().clamp(0.0, 8.0)
            );
            eprintln!(
                "[staging] ssao enabled_before_caller_policy={} ssr enabled={}",
                self.renderer.screen_space_ambient_occlusion().is_some(),
                self.renderer.screen_space_reflections().is_some()
            );
        }

        Ok(PhotographicSurroundingsReportV1 {
            schema: PHOTOGRAPHIC_SURROUNDINGS_REPORT_SCHEMA_V1.to_owned(),
            source: "subject_surroundings_solver".to_owned(),
            subject,
            ground,
            support_class,
            support_height_m,
            preserved_authored_surroundings,
            preserved_authored_environment,
            generated_floor,
            generated_cyclorama,
            support_nodes,
            backdrop_nodes,
            generated_nodes,
            contact_shadow_nodes,
            grid_nodes: Vec::new(),
            extent_m,
            background_color,
            background_luminance,
            contact_shadow_strength,
            reflection_strength,
            reflection_roughness,
            planar_reflection_capture_count: 0,
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

const fn is_zero_u32(value: &u32) -> bool {
    *value == 0
}

fn matte_ground_normal_texture<F: AssetFetcher>(
    host: &SceneHostCore<F>,
) -> Result<crate::TextureHandle, SceneHostError> {
    const SIZE: u32 = 32;
    let mut rgba8 = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let wave_x = ((x * 17 + y * 7 + 3) % 11) as i16 - 5;
            let wave_y = ((x * 5 + y * 19 + 1) % 13) as i16 - 6;
            rgba8.extend_from_slice(&[
                (128_i16 + wave_x).clamp(0, 255) as u8,
                (128_i16 + wave_y).clamp(0, 255) as u8,
                255,
                255,
            ]);
        }
    }
    host.assets
        .create_texture_for_slot(
            TextureMemoryDesc::rgba8_for_slot(
                TextureMemoryId::new("photographic/matte-ground-normal-v1")?,
                SIZE,
                SIZE,
                rgba8,
                TextureSlot::Normal,
            ),
            TextureSlot::Normal,
        )
        .map_err(Into::into)
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

/// Never let a derived backdrop crush toward black. A surround this dark reads
/// as a void rather than a room, and it is what made automatic renders look like
/// a part floating in space instead of a photograph of an object.
const MIN_SURROUND_LUMINANCE: f32 = 0.06;
const MAX_SURROUND_LUMINANCE: f32 = 0.42;
/// How far the surround sits from the subject in luminance. Separation is a
/// *ratio*, not a difference, because that is what survives an exposure change.
const SURROUND_SEPARATION: f32 = 2.6;

/// Grade the surround relative to the subject.
///
/// This used to pick from three hardcoded buckets, which was both discontinuous
/// and non-monotonic: a subject at 0.17 got a 0.32 backdrop and one at 0.19 got
/// 0.10 - a 3.2x step for a 0.02 change - and mid-tone subjects, the common
/// case, got a *darker* surround than bright ones. Now the surround is placed a
/// fixed ratio from the subject, below it when that still clears the crush
/// floor and above it when it does not, which is the only way to separate a dark
/// subject from its background.
fn derived_background(subject: Color, subject_luminance: f32) -> Color {
    let subject_luminance = subject_luminance.max(0.0);
    let dropped = subject_luminance / SURROUND_SEPARATION;
    let target_luminance = if dropped >= MIN_SURROUND_LUMINANCE {
        dropped
    } else {
        subject_luminance * SURROUND_SEPARATION
    }
    .clamp(MIN_SURROUND_LUMINANCE, MAX_SURROUND_LUMINANCE);
    let neutral = Vec3::splat(target_luminance);
    let subject_rgb = Vec3::new(subject.r, subject.g, subject.b);
    let complement = Vec3::ONE - subject_rgb.clamp(Vec3::ZERO, Vec3::ONE);
    let mut rgb = neutral * 0.88 + complement * target_luminance * 0.12;
    let luma = rgb.dot(Vec3::new(0.2126, 0.7152, 0.0722)).max(1.0e-5);
    rgb *= target_luminance / luma;
    rgb = rgb.clamp(
        Vec3::splat(MIN_SURROUND_LUMINANCE),
        Vec3::splat(MAX_SURROUND_LUMINANCE.max(0.62)),
    );
    Color::from_linear_rgb(rgb.x, rgb.y, rgb.z)
}

/// Screen-space row, as a fraction of frame height, that the reflection pass
/// mirrors about. It is a fixed fraction of the *frame*, not a horizon derived
/// from the scene, which is why it does not track where the floor actually is.
const SCREEN_SPACE_REFLECTION_HORIZON_FRACTION: f32 = 0.48;

/// Diagnostic-only switch: is this debug variable set to `1`?
///
/// These exist to isolate which post pass produces a given artifact. They are
/// not a product surface and must never be set on a release lane.
fn debug_enabled(name: &str) -> bool {
    std::env::var(name).as_deref() == Ok("1")
}

fn debug_disabled(name: &str) -> bool {
    debug_enabled(name)
}

fn linear_luminance(color: Color) -> f32 {
    0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b
}

#[cfg(test)]
mod tests;
