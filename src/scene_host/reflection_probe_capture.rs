use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{
    Aabb, AlphaMode, AssetFetcher, Color, EnvironmentDesc, EnvironmentHandle, GeometryDesc,
    MAX_REFLECTION_PROBES, MaterialDesc, MaterialHandle, MaterialKind, NodeKey, PerspectiveCamera,
    ReflectionProbe, ReflectionProbeKey, SceneDirtyState, Transform, Vec3,
};

pub const PHOTOGRAPHIC_REFLECTION_PROBE_REPORT_SCHEMA_V1: &str =
    "scena.photographic_reflection_probe_report.v1";
const PHOTOGRAPHIC_BRIGHT_CARD_LINEAR_RADIANCE: f32 = 6.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotographicReflectionProbeEntryV1 {
    pub resolution: u32,
    pub assigned_draws: u32,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub capture_position: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotographicReflectionProbeReportV1 {
    pub schema: String,
    pub subject_handle: u64,
    pub probes: Vec<PhotographicReflectionProbeEntryV1>,
    pub captured_faces: u32,
    pub cache_hits: u32,
    pub peak_scene_linear_radiance: f32,
    #[serde(default)]
    pub reflection_card_radiance: f32,
}

#[derive(Debug, Clone)]
pub(super) struct PhotographicReflectionProbeBakeCache {
    subject: NodeKey,
    signature: ReflectionProbeBakeSignature,
    probe_keys: Vec<ReflectionProbeKey>,
    report: PhotographicReflectionProbeReportV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReflectionProbeBakeSignature {
    scene: SceneDirtyState,
    environment: Option<EnvironmentHandle>,
    environment_intensity_bits: u32,
    environment_rotation_bits: u32,
    background_bits: [u32; 4],
    materials: Vec<MaterialHandle>,
}

#[derive(Debug)]
struct ReflectionProbeGroup {
    material: MaterialHandle,
    nodes: BTreeSet<NodeKey>,
    bounds: Aabb,
    score: f32,
}

struct CapturedProbe {
    group: ReflectionProbeGroup,
    faces: [Vec<[f32; 3]>; 6],
}

#[derive(Debug, Clone, Copy)]
struct PhotographicReflectionCardSpec {
    #[cfg(test)]
    role: &'static str,
    position: Vec3,
    width_m: f32,
    height_m: f32,
    #[cfg(test)]
    distance_from_subject_m: f32,
    #[cfg(test)]
    angle_from_camera_axis_degrees: f32,
    linear_color: [f32; 3],
    emissive_strength: f32,
}

#[derive(Debug)]
struct PhotographicReflectionCards {
    nodes: [NodeKey; 2],
    #[cfg(test)]
    specs: [PhotographicReflectionCardSpec; 2],
    #[cfg(test)]
    subject_extent_m: Vec3,
    #[cfg(test)]
    subject_radius_m: f32,
}

#[derive(Debug, Clone, Copy)]
struct ReflectionCardViewBasis {
    right: Vec3,
    front: Vec3,
}

impl ReflectionCardViewBasis {
    fn from_camera(subject: Vec3, camera: Vec3) -> Self {
        let front = (camera - subject).normalize_or_zero();
        let front = if front.length_squared() > 1.0e-8 {
            front
        } else {
            Vec3::Z
        };
        let right = Vec3::Y.cross(front).normalize_or_zero();
        let right = if right.length_squared() > 1.0e-8 {
            right
        } else {
            Vec3::X
        };
        Self { right, front }
    }
}

type CapturedProbeFaces = ([Vec<[f32; 3]>; 6], f32);

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn bake_photographic_reflection_probes(
        &mut self,
        subject_handle: u64,
    ) -> Result<PhotographicReflectionProbeReportV1, SceneHostError> {
        let subject = self.resolve_node(subject_handle)?;
        let old_generated_count =
            self.photographic_reflection_probe_cache
                .as_ref()
                .map_or(0, |cache| {
                    cache
                        .probe_keys
                        .iter()
                        .filter(|probe| self.scene.reflection_probe(**probe).is_some())
                        .count()
                });
        let authored_count = self
            .scene
            .reflection_probes()
            .count()
            .saturating_sub(old_generated_count);
        let mut groups = reflection_probe_groups(self, subject)?;
        groups.truncate(MAX_REFLECTION_PROBES.saturating_sub(authored_count));
        let baked_materials = groups
            .iter()
            .map(|group| group.material)
            .collect::<Vec<_>>();
        let signature = reflection_probe_bake_signature(self, &baked_materials);
        if let Some(cache) = &self.photographic_reflection_probe_cache
            && cache.subject == subject
            && cache.signature == signature
            && cache
                .probe_keys
                .iter()
                .all(|probe| self.scene.reflection_probe(*probe).is_some())
        {
            let mut report = cache.report.clone();
            report.captured_faces = 0;
            report.cache_hits = report.probes.len() as u32;
            return Ok(report);
        }

        let cards = install_photographic_reflection_cards(self, subject)?;
        let capture_result = (|| {
            let mut peak_scene_linear_radiance = 0.0_f32;
            let mut captured = Vec::with_capacity(groups.len());
            for group in groups {
                let (faces, peak) = capture_probe_faces(self, &group)?;
                peak_scene_linear_radiance = peak_scene_linear_radiance.max(peak);
                captured.push(CapturedProbe { group, faces });
            }
            Ok::<_, SceneHostError>((captured, peak_scene_linear_radiance))
        })();
        let cleanup_result = remove_photographic_reflection_cards(self, &cards);
        let (captured, peak_scene_linear_radiance) = match capture_result {
            Ok(captured) => {
                cleanup_result?;
                captured
            }
            Err(error) => {
                let _ = cleanup_result;
                return Err(error);
            }
        };

        let old_probe_keys = self
            .photographic_reflection_probe_cache
            .as_ref()
            .map(|cache| cache.probe_keys.clone())
            .unwrap_or_default();
        for probe in old_probe_keys {
            if self.scene.reflection_probe(probe).is_some() {
                self.scene
                    .remove_reflection_probe(probe)
                    .map_err(reflection_probe_error)?;
            }
        }

        let mut probe_keys = Vec::with_capacity(captured.len());
        let mut entries = Vec::with_capacity(captured.len());
        for (index, captured) in captured.into_iter().enumerate() {
            let resolution = crate::DEFAULT_REFLECTION_PROBE_RESOLUTION;
            let environment =
                self.assets
                    .create_environment(EnvironmentDesc::from_cubemap_radiance(
                        format!("scena://generated/reflection-probe/{subject_handle}/{index}"),
                        resolution,
                        captured.faces,
                    )?);
            let bounds = expanded_probe_bounds(captured.group.bounds);
            let capture_position = captured.group.bounds.center();
            let probe = self
                .scene
                .add_reflection_probe(
                    ReflectionProbe::new(bounds)
                        .with_capture_position(capture_position)
                        .with_resolution(resolution)
                        .with_environment(environment)
                        .assign_node(subject)
                        .assign_material(captured.group.material),
                )
                .map_err(reflection_probe_error)?;
            probe_keys.push(probe);
            entries.push(PhotographicReflectionProbeEntryV1 {
                resolution,
                assigned_draws: captured.group.nodes.len() as u32,
                bounds_min: vec3_array(bounds.min),
                bounds_max: vec3_array(bounds.max),
                capture_position: vec3_array(capture_position),
            });
        }

        let report = PhotographicReflectionProbeReportV1 {
            schema: PHOTOGRAPHIC_REFLECTION_PROBE_REPORT_SCHEMA_V1.to_owned(),
            subject_handle,
            captured_faces: entries.len() as u32 * 6,
            cache_hits: 0,
            probes: entries,
            peak_scene_linear_radiance,
            reflection_card_radiance: PHOTOGRAPHIC_BRIGHT_CARD_LINEAR_RADIANCE,
        };
        self.photographic_reflection_probe_cache = Some(PhotographicReflectionProbeBakeCache {
            subject,
            signature: reflection_probe_bake_signature(self, &baked_materials),
            probe_keys,
            report: report.clone(),
        });
        Ok(report)
    }
}

fn install_photographic_reflection_cards<F: AssetFetcher>(
    host: &mut SceneHostCore<F>,
    subject: NodeKey,
) -> Result<PhotographicReflectionCards, SceneHostError> {
    let bounds = host
        .scene
        .node_world_bounds(subject, &host.assets)?
        .ok_or(crate::LookupError::ImportHasNoBounds)?;
    let center = bounds.center();
    let extent = bounds.half_extent() * 2.0;
    let radius = bounds.bounding_sphere_radius().max(0.05);
    let camera_position = host
        .scene
        .camera_node(host.active_camera)
        .and_then(|node| host.scene.world_transform(node))
        .map(|transform| transform.translation)
        .unwrap_or(center + Vec3::Z * radius * 4.0);
    let view = ReflectionCardViewBasis::from_camera(center, camera_position);
    let angle_degrees = 40.0_f32;
    let angle = angle_degrees.to_radians();
    let distance = radius * 2.0;
    let height = extent.y * 2.0;
    let width = extent.x.max(extent.z) * 2.0;
    let forward = distance * angle.cos();
    let lateral = distance * angle.sin();
    let bright_color = [1.0, 1.0, 1.0];
    let dark_color = [0.03, 0.03, 0.03];
    let specs = [
        PhotographicReflectionCardSpec {
            #[cfg(test)]
            role: "bright_strip",
            position: center + view.front * forward - view.right * lateral,
            width_m: width,
            height_m: height,
            #[cfg(test)]
            distance_from_subject_m: distance,
            #[cfg(test)]
            angle_from_camera_axis_degrees: angle_degrees,
            linear_color: bright_color,
            emissive_strength: PHOTOGRAPHIC_BRIGHT_CARD_LINEAR_RADIANCE,
        },
        PhotographicReflectionCardSpec {
            #[cfg(test)]
            role: "dark_flag",
            position: center + view.front * forward + view.right * lateral,
            width_m: width,
            height_m: height,
            #[cfg(test)]
            distance_from_subject_m: distance,
            #[cfg(test)]
            angle_from_camera_axis_degrees: angle_degrees,
            linear_color: dark_color,
            emissive_strength: 0.0,
        },
    ];
    let geometry = host.assets.create_geometry(GeometryDesc::box_xyz(
        specs[0].width_m,
        specs[0].height_m,
        radius * 0.01,
    ));
    let bright_material = host.assets.create_material(
        MaterialDesc::unlit(Color::BLACK)
            .with_emissive(Color::from_linear_rgb(
                specs[0].linear_color[0],
                specs[0].linear_color[1],
                specs[0].linear_color[2],
            ))
            .with_emissive_strength(specs[0].emissive_strength),
    );
    let dark_material = host
        .assets
        .create_material(MaterialDesc::unlit(Color::from_linear_rgb(
            specs[1].linear_color[0],
            specs[1].linear_color[1],
            specs[1].linear_color[2],
        )));
    let mut nodes = Vec::with_capacity(2);
    for (spec, material) in specs.iter().zip([bright_material, dark_material]) {
        let node = host
            .scene
            .mesh(geometry, material)
            .transform(Transform::at(spec.position).looking_at(center, Vec3::Y))
            .add()?;
        nodes.push(node);
    }
    Ok(PhotographicReflectionCards {
        nodes: [nodes[0], nodes[1]],
        #[cfg(test)]
        specs,
        #[cfg(test)]
        subject_extent_m: extent,
        #[cfg(test)]
        subject_radius_m: radius,
    })
}

fn remove_photographic_reflection_cards<F: AssetFetcher>(
    host: &mut SceneHostCore<F>,
    cards: &PhotographicReflectionCards,
) -> Result<(), SceneHostError> {
    for node in cards.nodes {
        if host.scene.visible(node).is_some() {
            host.scene.remove_node(node)?;
        }
    }
    Ok(())
}

fn reflection_probe_groups<F: AssetFetcher>(
    host: &SceneHostCore<F>,
    subject: NodeKey,
) -> Result<Vec<ReflectionProbeGroup>, SceneHostError> {
    let subtree = host
        .scene
        .subtree_nodes(subject)?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let inspection = host.scene.inspect_with_assets(&host.assets);
    let mut grouped = BTreeMap::<MaterialHandle, ReflectionProbeGroup>::new();
    for draw in inspection.draw_list() {
        if !subtree.contains(&draw.node()) {
            continue;
        }
        let Some(material) = host.assets.material(draw.material()) else {
            continue;
        };
        let Some(score) = reflection_probe_material_score(&host.assets, &material) else {
            continue;
        };
        let bounds = transformed_aabb(draw.local_bounds(), draw.world_transform());
        grouped
            .entry(draw.material())
            .and_modify(|group| {
                group.nodes.insert(draw.node());
                group.bounds = group.bounds.union(bounds);
            })
            .or_insert_with(|| ReflectionProbeGroup {
                material: draw.material(),
                nodes: BTreeSet::from([draw.node()]),
                bounds,
                score,
            });
    }
    let mut groups = grouped.into_values().collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.material.cmp(&right.material))
    });
    Ok(groups)
}

fn reflection_probe_material_score<F: AssetFetcher>(
    assets: &crate::Assets<F>,
    material: &crate::MaterialDesc,
) -> Option<f32> {
    if material.kind() != MaterialKind::PbrMetallicRoughness
        || matches!(material.alpha_mode(), AlphaMode::Blend)
        || material.transmission_factor() > 0.0
    {
        return None;
    }
    let effective = assets.effective_material_pbr(material);
    let metallic = effective.metallic_mean;
    let roughness = effective.roughness_mean;
    let clearcoat = material.clearcoat_factor();
    (metallic >= 0.25 || roughness <= 0.3 || clearcoat >= 0.2)
        .then_some(metallic * 2.0 + (1.0 - roughness) + clearcoat)
}

fn reflection_probe_bake_signature<F: AssetFetcher>(
    host: &SceneHostCore<F>,
    materials: &[MaterialHandle],
) -> ReflectionProbeBakeSignature {
    let background = host.renderer.background_color();
    ReflectionProbeBakeSignature {
        scene: host.scene.dirty_state(),
        environment: host.renderer.environment(),
        environment_intensity_bits: host.renderer.environment_intensity().to_bits(),
        environment_rotation_bits: host.renderer.environment_rotation_y_degrees().to_bits(),
        background_bits: [
            background.r.to_bits(),
            background.g.to_bits(),
            background.b.to_bits(),
            background.a.to_bits(),
        ],
        materials: materials.to_vec(),
    }
}

fn capture_probe_faces<F: AssetFetcher>(
    host: &mut SceneHostCore<F>,
    group: &ReflectionProbeGroup,
) -> Result<CapturedProbeFaces, SceneHostError> {
    let resolution = crate::DEFAULT_REFLECTION_PROBE_RESOLUTION;
    let original_viewport = host.viewport;
    let original_host_camera = host.active_camera;
    let original_scene_camera = host.scene.active_camera();
    let original_controls = host.camera_controls;
    let original_probe_state = host.scene.reflection_probes_enabled();
    let original_capture_state = host.renderer.scene_linear_capture_enabled();
    let original_supersample = host.renderer.supersample_factor();
    let original_filter = host.renderer.reconstruction_filter();
    let mut visibility = Vec::new();
    let mut hidden = group.nodes.clone();
    for draw in host.scene.inspect_with_assets(&host.assets).draw_list() {
        if host
            .assets
            .material(draw.material())
            .is_some_and(|material| {
                matches!(material.alpha_mode(), AlphaMode::Blend)
                    || material.transmission_factor() > 0.0
            })
        {
            hidden.insert(draw.node());
        }
    }
    for node in hidden {
        if let Some(visible) = host.scene.visible(node) {
            visibility.push((node, visible));
            if visible {
                host.scene.set_visible(node, false)?;
            }
        }
    }

    let capture_position = group.bounds.center();
    let camera = host.scene.add_perspective_camera(
        host.scene.root(),
        PerspectiveCamera::default()
            .with_fov_degrees(90.0)
            .with_aspect(1.0),
        Transform::at(capture_position),
    )?;
    let camera_node = host.scene.camera_node(camera).ok_or_else(|| {
        SceneHostError::new(SceneHostErrorCode::Lookup, "probe camera node missing")
    })?;

    let capture_result = (|| {
        host.scene.set_reflection_probes_enabled(false);
        host.scene.set_active_camera(camera)?;
        host.active_camera = camera;
        host.renderer.set_supersample_factor(1)?;
        host.renderer.set_scene_linear_capture_enabled(true);
        host.resize(resolution as f32, resolution as f32, 1.0)?;

        let mut peak = 0.0_f32;
        let mut faces: [Vec<[f32; 3]>; 6] = std::array::from_fn(|_| Vec::new());
        for (face_index, (direction, up)) in cubemap_capture_views().into_iter().enumerate() {
            host.scene.set_transform(
                camera_node,
                Transform::at(capture_position).looking_at(capture_position + direction, up),
            )?;
            host.prepare()?;
            host.render()?;
            let capture = host.renderer.scene_linear_capture()?;
            if capture.width() != resolution || capture.height() != resolution {
                return Err(SceneHostError::new(
                    SceneHostErrorCode::Capture,
                    format!(
                        "reflection probe face expected {resolution}x{resolution}, got {}x{}",
                        capture.width(),
                        capture.height()
                    ),
                ));
            }
            let rgba = capture.into_rgba32f();
            let mut face = Vec::with_capacity((resolution * resolution) as usize);
            for y in 0..resolution {
                for x in 0..resolution {
                    let source_x = resolution - 1 - x;
                    let pixel = rgba[(y * resolution + source_x) as usize];
                    let rgb = [pixel[0].max(0.0), pixel[1].max(0.0), pixel[2].max(0.0)];
                    peak = peak.max(rgb[0]).max(rgb[1]).max(rgb[2]);
                    face.push(rgb);
                }
            }
            faces[face_index] = face;
        }
        Ok((faces, peak))
    })();

    host.scene
        .set_reflection_probes_enabled(original_probe_state);
    let _ = host
        .scene
        .set_active_camera(original_scene_camera.unwrap_or(original_host_camera));
    host.active_camera = original_host_camera;
    let _ = host.scene.remove_node(camera_node);
    for (node, visible) in visibility {
        let _ = host.scene.set_visible(node, visible);
    }
    host.renderer
        .set_scene_linear_capture_enabled(original_capture_state);
    let restore_viewport = host.resize(
        original_viewport.logical_width(),
        original_viewport.logical_height(),
        original_viewport.device_pixel_ratio(),
    );
    let restore_supersample = host.renderer.set_supersample_factor(original_supersample);
    host.renderer.set_reconstruction_filter(original_filter);
    host.camera_controls = original_controls;

    match capture_result {
        Err(error) => Err(error),
        Ok(capture) => {
            restore_viewport?;
            restore_supersample?;
            Ok(capture)
        }
    }
}

fn cubemap_capture_views() -> [(Vec3, Vec3); 6] {
    [
        (Vec3::X, Vec3::Y),
        (Vec3::NEG_X, Vec3::Y),
        (Vec3::Y, Vec3::NEG_Z),
        (Vec3::NEG_Y, Vec3::Z),
        (Vec3::Z, Vec3::Y),
        (Vec3::NEG_Z, Vec3::Y),
    ]
}

fn expanded_probe_bounds(bounds: Aabb) -> Aabb {
    let center = bounds.center();
    let half = bounds.half_extent();
    let padding = bounds.bounding_sphere_radius().mul_add(0.15, 0.01);
    let half = Vec3::new(
        (half.x + padding).max(0.02),
        (half.y + padding).max(0.02),
        (half.z + padding).max(0.02),
    );
    Aabb::new(center - half, center + half)
}

fn transformed_aabb(bounds: Aabb, transform: Transform) -> Aabb {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for x in [bounds.min.x, bounds.max.x] {
        for y in [bounds.min.y, bounds.max.y] {
            for z in [bounds.min.z, bounds.max.z] {
                let point = transform.translation
                    + transform.rotation * (Vec3::new(x, y, z) * transform.scale);
                min = min.min(point);
                max = max.max(point);
            }
        }
    }
    Aabb::new(min, max)
}

fn reflection_probe_error(error: crate::ReflectionProbeError) -> SceneHostError {
    SceneHostError::new(SceneHostErrorCode::Prepare, error.to_string())
}

const fn vec3_array(value: Vec3) -> [f32; 3] {
    [value.x, value.y, value.z]
}

#[cfg(test)]
mod tests {
    use super::{
        SceneHostCore, install_photographic_reflection_cards, reflection_probe_groups,
        remove_photographic_reflection_cards,
    };
    use crate::{
        Color, GeometryDesc, MaterialDesc, TextureMemoryDesc, TextureMemoryId, TextureSlot,
        Transform, Vec3,
    };

    #[test]
    fn photographic_probe_groups_use_effective_orm_material_values() {
        let mut host = SceneHostCore::headless(64, 64).expect("headless host builds");
        let geometry = host
            .assets
            .create_geometry(GeometryDesc::box_xyz(0.4, 0.4, 0.4));
        let orm_texture = |identity: &str, roughness: u8, metallic: u8| {
            host.assets
                .create_texture_for_slot(
                    TextureMemoryDesc::rgba8_for_slot(
                        TextureMemoryId::new(identity).expect("texture identity is valid"),
                        2,
                        2,
                        [255, roughness, metallic, 255].repeat(4),
                        TextureSlot::MetallicRoughness,
                    ),
                    TextureSlot::MetallicRoughness,
                )
                .expect("ORM texture inserts")
        };
        let dielectric_orm = orm_texture("tests/reflection-probe/dielectric-orm", 153, 0);
        let metal_orm = orm_texture("tests/reflection-probe/metal-orm", 77, 230);
        let dielectric = host.assets.create_material(
            MaterialDesc::pbr_metallic_roughness(Color::WHITE, 1.0, 1.0)
                .with_metallic_roughness_texture(dielectric_orm),
        );
        let metal = host.assets.create_material(
            MaterialDesc::pbr_metallic_roughness(Color::WHITE, 1.0, 1.0)
                .with_metallic_roughness_texture(metal_orm),
        );
        let root = host
            .scene
            .add_empty(host.scene.root(), Transform::IDENTITY)
            .expect("subject root inserts");
        host.scene
            .mesh(geometry, dielectric)
            .parent(root)
            .add()
            .expect("dielectric component inserts");
        host.scene
            .mesh(geometry, metal)
            .parent(root)
            .transform(Transform::at(Vec3::new(0.6, 0.0, 0.0)))
            .add()
            .expect("metal component inserts");

        let groups = reflection_probe_groups(&host, root).expect("probe groups build");

        assert_eq!(
            groups.len(),
            1,
            "texture-backed dielectric must not consume a local reflection probe"
        );
        assert_eq!(groups[0].material, metal);
    }

    #[test]
    fn photographic_reflection_cards_are_fixed_and_temporary() {
        let mut host = SceneHostCore::headless(64, 64).expect("headless host builds");
        let geometry = host
            .assets
            .create_geometry(GeometryDesc::box_xyz(2.0, 1.0, 1.0));
        let material = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(
                Color::LIGHT_GRAY,
                1.0,
                0.12,
            ));
        let subject = host
            .scene
            .mesh(geometry, material)
            .add()
            .expect("subject inserts");
        let before = host
            .scene
            .inspect_with_assets(&host.assets)
            .draw_list()
            .len();

        let cards = install_photographic_reflection_cards(&mut host, subject)
            .expect("reflection cards install");

        assert_eq!(cards.specs.len(), 2);
        assert_eq!(cards.specs[0].role, "bright_strip");
        assert_eq!(cards.specs[1].role, "dark_flag");
        for spec in &cards.specs {
            assert!((spec.distance_from_subject_m - 2.0 * cards.subject_radius_m).abs() < 1.0e-5);
            assert!((spec.angle_from_camera_axis_degrees - 40.0).abs() < 1.0e-4);
            assert!((spec.height_m - 2.0 * cards.subject_extent_m.y).abs() < 1.0e-5);
            let expected_width = 2.0 * cards.subject_extent_m.x.max(cards.subject_extent_m.z);
            assert!((spec.width_m - expected_width).abs() < 1.0e-5);
        }
        assert_eq!(cards.specs[0].linear_color, [1.0, 1.0, 1.0]);
        assert_eq!(cards.specs[0].emissive_strength, 6.0);
        assert_eq!(cards.specs[1].linear_color, [0.03, 0.03, 0.03]);
        assert_eq!(cards.specs[1].emissive_strength, 0.0);
        assert_eq!(
            host.scene
                .inspect_with_assets(&host.assets)
                .draw_list()
                .len(),
            before + 2
        );

        remove_photographic_reflection_cards(&mut host, &cards).expect("reflection cards remove");
        assert_eq!(
            host.scene
                .inspect_with_assets(&host.assets)
                .draw_list()
                .len(),
            before,
            "capture-only cards must not survive into the beauty scene"
        );
    }

    #[test]
    fn photographic_probe_bake_captures_hdr_and_restores_host_state() {
        let Ok(mut host) = SceneHostCore::headless_gpu(120, 80) else {
            return;
        };
        let geometry = host
            .assets
            .create_geometry(GeometryDesc::box_xyz(0.6, 0.6, 0.6));
        let metal = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(
                Color::LIGHT_GRAY,
                1.0,
                0.08,
            ));
        let emitter = host.assets.create_material(
            MaterialDesc::unlit(Color::BLACK)
                .with_emissive(Color::from_linear_rgb(1.0, 0.1, 0.05))
                .with_emissive_strength(6.0),
        );
        let root = host
            .scene
            .add_empty(host.scene.root(), Transform::IDENTITY)
            .expect("subject root inserts");
        let subject = host
            .scene
            .mesh(geometry, metal)
            .parent(root)
            .add()
            .expect("metal component inserts");
        host.scene
            .mesh(geometry, emitter)
            .parent(root)
            .transform(Transform::at(Vec3::new(1.4, 0.0, 0.0)))
            .add()
            .expect("emissive neighbour inserts");
        let subject_handle = host.register_node(root);
        let active_camera = host.active_camera;
        let viewport = host.viewport_size();

        let report = host
            .bake_photographic_reflection_probes(subject_handle)
            .expect("photographic probe bake succeeds");

        assert_eq!(report.probes.len(), 1);
        assert_eq!(report.captured_faces, 6);
        assert_eq!(report.probes[0].resolution, 256);
        assert_eq!(report.reflection_card_radiance, 6.0);
        assert!(
            report.peak_scene_linear_radiance > 1.0,
            "probe source must retain HDR radiance, got {}",
            report.peak_scene_linear_radiance,
        );
        assert_eq!(host.active_camera, active_camera);
        assert_eq!(host.viewport_size(), viewport);
        assert_eq!(host.scene.visible(subject), Some(true));
        assert_eq!(host.scene.active_camera(), Some(active_camera));
        assert!(
            host.scene
                .reflection_probes()
                .all(|(_, probe)| probe.environment().is_some()),
            "every baked probe must own a prepared environment",
        );

        let cached = host
            .bake_photographic_reflection_probes(subject_handle)
            .expect("unchanged probe bake reuses cache");
        assert_eq!(cached.cache_hits, report.probes.len() as u32);
        assert_eq!(cached.captured_faces, 0);
    }

    #[test]
    #[ignore = "bounded real-asset photo diagnostic; run explicitly"]
    fn row7_valve_edge_probe_hides_red_wheel_and_compares_linear_with_final() {
        let recipe_path = "tests/assets/photo/final/recipes/valve_manifold.recipe.json";
        let repository_root = std::path::Path::new(".")
            .canonicalize()
            .expect("repository root canonicalizes");
        let material_root = repository_root.join("target/photo-materials");
        let recipe_text = std::fs::read_to_string(recipe_path)
            .expect("valve recipe reads")
            .replace(
                "../../../../../target/photo-materials",
                material_root.to_str().expect("material path is UTF-8"),
            );
        let build = pollster::block_on(SceneHostCore::build_recipe_json_prefer_gpu(
            recipe_path,
            &recipe_text,
            crate::RecipeBuildPolicy::testing().with_allowed_root(repository_root),
        ))
        .expect("valve diagnostic recipe builds");
        let import = build.manifest.imports[0].clone();
        let subject = import.primary_root.expect("valve primary root exists");
        let mut host = build.host;

        let mut hidden = 0_usize;
        for path in [
            "subject:/valve_manifold_root/handwheel_rim",
            "subject:/valve_manifold_root/handwheel_hub",
            "subject:/valve_manifold_root/spoke_0",
            "subject:/valve_manifold_root/spoke_1",
            "subject:/valve_manifold_root/spoke_2",
        ] {
            if let Some(handle) = import.nodes_by_path.get(path)
                && let Ok(node) = host.resolve_node(*handle)
            {
                host.scene
                    .set_visible(node, false)
                    .expect("red wheel source node hides");
                hidden += 1;
            }
        }
        if let Some(hub) = build
            .manifest
            .nodes
            .iter()
            .find(|node| node.id == "valve_hub")
        {
            let node = host
                .resolve_node(hub.handle)
                .expect("authored red hub resolves");
            host.scene
                .set_visible(node, false)
                .expect("authored red hub hides");
            hidden += 1;
        }
        assert!(
            hidden >= 4,
            "the complete red wheel must be hidden, got {hidden} nodes"
        );

        host.resize(320.0, 210.0, 1.0)
            .expect("diagnostic viewport resizes");
        host.apply_photographic_surroundings(subject)
            .expect("diagnostic surroundings apply");
        host.apply_final_photographic_lighting(subject)
            .expect("diagnostic lighting applies");
        host.frame_node_product_view(subject)
            .expect("valve diagnostic frames");
        host.renderer.set_scene_linear_capture_enabled(true);
        host.prepare().expect("valve diagnostic prepares");
        host.render().expect("valve diagnostic renders");
        let linear = host
            .renderer
            .scene_linear_capture()
            .expect("linear pre-tonemap capture reads");
        let final_capture = host.capture().expect("final capture reads");
        let linear_pairs = red_green_edge_pairs_linear(linear.rgba32f(), 320, 210);
        let final_pairs = red_green_edge_pairs_rgba8(&final_capture.rgba8, 320, 210);
        let artifact = serde_json::json!({
            "schema": "scena.photo_realism.row7.valve_edge_probe.v1",
            "red_wheel_hidden_nodes": hidden,
            "linear_pre_tonemap_red_green_pairs": linear_pairs,
            "final_output_red_green_pairs": final_pairs,
            "classification": if final_pairs == 0 {
                "artifact_follows_scene_source"
            } else if linear_pairs > 0 {
                "pair_present_before_tonemap"
            } else {
                "sampling_patch_required"
            }
        });
        let artifact_dir = std::path::Path::new("target/photo-realism-row7");
        std::fs::create_dir_all(artifact_dir).expect("row7 artifact directory creates");
        std::fs::write(
            artifact_dir.join("valve-edge-probe.json"),
            serde_json::to_vec_pretty(&artifact).expect("probe report serializes"),
        )
        .expect("probe report writes");
        eprintln!("valve edge probe: {artifact}");
        assert!(
            final_pairs == 0 || linear_pairs > 0,
            "a final-only red/green pair without the hidden scene source would require a sampling patch: {artifact}"
        );
    }

    #[test]
    #[ignore = "bounded real-asset photo diagnostic; run explicitly"]
    fn row7_mug_bowtie_probe_separates_environment_rotation_from_local_probes() {
        let recipe_path = "tests/assets/photo/final/recipes/colored_travel_mug.recipe.json";
        let repository_root = std::path::Path::new(".")
            .canonicalize()
            .expect("repository root canonicalizes");
        let material_root = repository_root.join("target/photo-materials");
        let recipe_text = std::fs::read_to_string(recipe_path)
            .expect("mug recipe reads")
            .replace(
                "../../../../../target/photo-materials",
                material_root.to_str().expect("material path is UTF-8"),
            );
        let build = pollster::block_on(SceneHostCore::build_recipe_json_prefer_gpu(
            recipe_path,
            &recipe_text,
            crate::RecipeBuildPolicy::testing().with_allowed_root(repository_root),
        ))
        .expect("mug diagnostic recipe builds");
        let subject = build.manifest.imports[0]
            .primary_root
            .expect("mug primary root exists");
        let mut host = build.host;
        host.resize(320.0, 210.0, 1.0)
            .expect("mug diagnostic viewport resizes");
        host.apply_photographic_surroundings(subject)
            .expect("diagnostic surroundings apply");
        host.apply_final_photographic_lighting(subject)
            .expect("diagnostic lighting applies");
        host.frame_node_product_view(subject)
            .expect("mug diagnostic frames");
        let probes = host
            .bake_photographic_reflection_probes(subject)
            .expect("mug local probes bake");
        let original_rotation = host.renderer.environment_rotation_y_degrees();
        let baseline = render_diagnostic_capture(&mut host);

        host.renderer
            .set_environment_rotation_y_degrees(original_rotation + 73.0);
        let rotated = render_diagnostic_capture(&mut host);

        host.renderer
            .set_environment_rotation_y_degrees(original_rotation);
        host.scene.set_reflection_probes_enabled(false);
        let no_local_probes = render_diagnostic_capture(&mut host);
        host.scene.set_reflection_probes_enabled(true);

        let environment_rotation_delta = rgba8_absolute_delta(&baseline.rgba8, &rotated.rgba8);
        let local_probe_delta = rgba8_absolute_delta(&baseline.rgba8, &no_local_probes.rgba8);
        let artifact_dir = std::path::Path::new("target/photo-realism-row7");
        std::fs::create_dir_all(artifact_dir).expect("row7 artifact directory creates");
        baseline
            .write_png(artifact_dir.join("mug-bowtie-baseline.png"))
            .expect("baseline PNG writes");
        rotated
            .write_png(artifact_dir.join("mug-bowtie-environment-rotated.png"))
            .expect("rotated PNG writes");
        no_local_probes
            .write_png(artifact_dir.join("mug-bowtie-no-local-probes.png"))
            .expect("probe-disabled PNG writes");
        let artifact = serde_json::json!({
            "schema": "scena.photo_realism.row7.mug_bowtie_probe.v1",
            "environment_rotation_degrees": 73.0,
            "environment_rotation_absolute_delta": environment_rotation_delta,
            "local_probe_absolute_delta": local_probe_delta,
            "local_probe_count": probes.probes.len(),
            "cube_seam_control": "cubemap_face_pixels_at_face_corners_blend_three_adjacent_faces"
        });
        std::fs::write(
            artifact_dir.join("mug-bowtie-probe.json"),
            serde_json::to_vec_pretty(&artifact).expect("probe report serializes"),
        )
        .expect("probe report writes");
        eprintln!("mug bowtie probe: {artifact}");
        assert!(
            environment_rotation_delta > 0,
            "rotating HDR content must change the controlled mug capture"
        );
        assert!(
            probes.probes.is_empty() || local_probe_delta > 0,
            "disabling populated local probes must be independently observable"
        );
    }

    fn render_diagnostic_capture(host: &mut SceneHostCore) -> crate::CaptureRgba8 {
        host.prepare().expect("diagnostic prepares");
        host.render().expect("diagnostic renders");
        host.capture().expect("diagnostic captures")
    }

    fn rgba8_absolute_delta(left: &[u8], right: &[u8]) -> u64 {
        left.iter()
            .zip(right)
            .enumerate()
            .filter(|(index, _)| index % 4 != 3)
            .map(|(_, (left, right))| u64::from(left.abs_diff(*right)))
            .sum()
    }

    fn red_green_edge_pairs_linear(pixels: &[[f32; 4]], width: u32, height: u32) -> usize {
        red_green_edge_pairs(width, height, |index| {
            let pixel = pixels[index];
            [pixel[0], pixel[1], pixel[2]]
        })
    }

    fn red_green_edge_pairs_rgba8(pixels: &[u8], width: u32, height: u32) -> usize {
        red_green_edge_pairs(width, height, |index| {
            let offset = index * 4;
            [
                f32::from(pixels[offset]) / 255.0,
                f32::from(pixels[offset + 1]) / 255.0,
                f32::from(pixels[offset + 2]) / 255.0,
            ]
        })
    }

    fn red_green_edge_pairs(width: u32, height: u32, color: impl Fn(usize) -> [f32; 3]) -> usize {
        let red = |value: [f32; 3]| {
            value[0] > 0.04 && value[0] > value[1] * 1.35 && value[0] > value[2] * 1.20
        };
        let green = |value: [f32; 3]| {
            value[1] > 0.04 && value[1] > value[0] * 1.35 && value[1] > value[2] * 1.20
        };
        let mut pairs = 0;
        for y in 0..height {
            for x in 0..width {
                let index = (y * width + x) as usize;
                let current = color(index);
                for neighbor in [
                    (x + 1 < width).then_some(index + 1),
                    (y + 1 < height).then_some(index + width as usize),
                ]
                .into_iter()
                .flatten()
                {
                    let adjacent = color(neighbor);
                    pairs += usize::from(
                        (red(current) && green(adjacent)) || (green(current) && red(adjacent)),
                    );
                }
            }
        }
        pairs
    }
}
