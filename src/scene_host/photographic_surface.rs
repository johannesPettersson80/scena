use serde::{Deserialize, Serialize};

use super::{SceneHostCore, SceneHostError};
use crate::{Aabb, AssetFetcher, MaterialDesc, MaterialKind, Transform, Vec3};

mod asset_health;

use asset_health::{
    append_component_layout_issues, material_values_are_physical, transform_bounds,
};

pub const PHOTOGRAPHIC_SURFACE_REPORT_SCHEMA_V1: &str = "scena.photographic_surface_report.v1";
const PRESERVE_SHARP_EDGES_TAG: &str = "photographic_preserve_sharp_edges";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotographicSurfaceReportV1 {
    pub schema: String,
    pub source: String,
    pub subject: u64,
    pub mesh_count: usize,
    pub repaired_normal_meshes: usize,
    pub reversed_winding_meshes: usize,
    pub disconnected_meshes: usize,
    pub maximum_disconnected_components: usize,
    pub removed_degenerate_triangles: usize,
    pub generated_tangent_frames: usize,
    pub micro_beveled_meshes: usize,
    pub preserved_sharp_meshes: usize,
    pub micro_surface_materials: usize,
    pub neutral_fallback_materials: usize,
    pub max_bevel_m: f32,
    pub boundary_edges: usize,
    pub nonmanifold_edges: usize,
    pub folded_edges: usize,
    pub self_intersections: usize,
    pub duplicate_vertices_removed: usize,
    pub minimum_texture_dimension: Option<u32>,
    pub inspection_scope: Vec<String>,
    pub coherent_visible_subject: bool,
    pub issues: Vec<PhotographicAssetIssueV1>,
    pub rejected_meshes: Vec<PhotographicSurfaceRejectedMeshV1>,
    pub substance_claims: Vec<String>,
    pub supported_promise: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhotographicAssetIssueClassV1 {
    SafeRepair,
    AppearanceChangeRequired,
    Unrecoverable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhotographicAssetIssueV1 {
    pub class: PhotographicAssetIssueClassV1,
    pub code: String,
    pub node: Option<u64>,
    pub message: String,
    pub required_input: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhotographicSurfaceRejectedMeshV1 {
    pub node: u64,
    pub reason: String,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn apply_photographic_surface(
        &mut self,
        subject: u64,
    ) -> Result<PhotographicSurfaceReportV1, SceneHostError> {
        let subject_node = self.resolve_node(subject)?;
        let subtree = self.scene.subtree_nodes(subject_node)?;
        let inspection = self.scene.inspect_with_assets(&self.assets);
        let mut meshes = Vec::new();
        for draw in inspection.draw_list() {
            if subtree.contains(&draw.node())
                && !meshes.iter().any(|(node, _, _)| *node == draw.node())
            {
                meshes.push((draw.node(), draw.geometry(), draw.material()));
            }
        }

        let subject_bounds = self
            .scene
            .node_world_bounds(subject_node, &self.assets)?
            .ok_or(crate::LookupError::ImportHasNoBounds)?;
        let subject_radius = subject_bounds.bounding_sphere_radius().max(1.0e-4);
        let micro_scale_m = (subject_radius * 0.0025).clamp(1.0e-5, 0.01);
        let mut report = PhotographicSurfaceReportV1 {
            schema: PHOTOGRAPHIC_SURFACE_REPORT_SCHEMA_V1.to_owned(),
            source: "photographic_surface_solver".to_owned(),
            subject,
            mesh_count: meshes.len(),
            repaired_normal_meshes: 0,
            reversed_winding_meshes: 0,
            disconnected_meshes: 0,
            maximum_disconnected_components: 0,
            removed_degenerate_triangles: 0,
            generated_tangent_frames: 0,
            micro_beveled_meshes: 0,
            preserved_sharp_meshes: 0,
            micro_surface_materials: 0,
            neutral_fallback_materials: 0,
            max_bevel_m: 0.0,
            boundary_edges: 0,
            nonmanifold_edges: 0,
            folded_edges: 0,
            self_intersections: 0,
            duplicate_vertices_removed: 0,
            minimum_texture_dimension: None,
            inspection_scope: [
                "geometry",
                "transforms",
                "units",
                "materials",
                "textures",
                "animations",
                "cameras",
                "scene_hierarchy",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            coherent_visible_subject: !meshes.is_empty(),
            issues: Vec::new(),
            rejected_meshes: Vec::new(),
            substance_claims: Vec::new(),
            supported_promise: "automatic photorealistic rendering for coherent geometry with sufficient physical material information".to_owned(),
        };
        if meshes.is_empty() {
            report.issues.push(asset_issue(
                PhotographicAssetIssueClassV1::Unrecoverable,
                "visible_subject_geometry_missing",
                Some(subject),
                "selected photographic subject contains no renderable mesh",
                Some("supply coherent visible subject geometry"),
            ));
        }
        let mut component_signatures = Vec::<(crate::GeometryHandle, Transform)>::new();
        let mut component_bounds = Vec::<(u64, Aabb)>::new();
        for node in inspection.nodes() {
            if !subtree.contains(&node.node()) {
                continue;
            }
            let Some(geometry) = node.mesh_geometry() else {
                continue;
            };
            let handle = self.register_node(node.node());
            if !node.visible() {
                report.issues.push(asset_issue(
                    PhotographicAssetIssueClassV1::AppearanceChangeRequired,
                    "hidden_subject_component",
                    Some(handle),
                    "selected subject contains a hidden mesh component",
                    Some("show the component or exclude it from the photographic subject"),
                ));
            }
            let signature = (geometry, node.world_transform());
            if component_signatures.contains(&signature) {
                report.issues.push(asset_issue(
                    PhotographicAssetIssueClassV1::AppearanceChangeRequired,
                    "duplicate_subject_component",
                    Some(handle),
                    "selected subject contains an exact duplicate geometry placement",
                    Some("remove the duplicate or confirm that both coincident components are intended"),
                ));
            } else {
                component_signatures.push(signature);
            }
            if let Some(material) = node
                .mesh_material()
                .and_then(|material| self.assets.material(material))
                && !material_values_are_physical(&material)
            {
                report.issues.push(asset_issue(
                    PhotographicAssetIssueClassV1::Unrecoverable,
                    "non_finite_material_value",
                    Some(handle),
                    "material contains non-finite or physically impossible scalar/color values",
                    Some("supply finite PBR values in their documented physical ranges"),
                ));
                report.coherent_visible_subject = false;
            }
            if let Some(bounds) = node.bounds() {
                let bounds = transform_bounds(bounds, node.world_transform());
                let component_radius = bounds.bounding_sphere_radius();
                if component_radius <= subject_radius * 1.0e-5 {
                    report.issues.push(asset_issue(
                        PhotographicAssetIssueClassV1::AppearanceChangeRequired,
                        "microscopic_subject_component",
                        Some(handle),
                        "subject component is microscopic relative to the selected subject",
                        Some("confirm units and transforms or remove the microscopic component"),
                    ));
                }
                component_bounds.push((handle, bounds));
            }
        }
        append_component_layout_issues(&mut report, &component_bounds);

        for (node, geometry_handle, material_handle) in meshes {
            let Some(original_geometry) = self.assets.geometry(geometry_handle) else {
                report.coherent_visible_subject = false;
                report.issues.push(asset_issue(
                    PhotographicAssetIssueClassV1::Unrecoverable,
                    "missing_geometry_resource",
                    Some(self.register_node(node)),
                    "subject mesh references geometry that is unavailable",
                    Some("supply the missing geometry resource"),
                ));
                report.rejected_meshes.push(rejected(
                    self.register_node(node),
                    "missing_geometry_resource",
                ));
                continue;
            };
            let Some(original_material) = self.assets.material(material_handle) else {
                report.coherent_visible_subject = false;
                report.issues.push(asset_issue(
                    PhotographicAssetIssueClassV1::Unrecoverable,
                    "missing_material_resource",
                    Some(self.register_node(node)),
                    "subject mesh references material data that is unavailable",
                    Some("supply a physical material definition or remove the invalid assignment"),
                ));
                report.rejected_meshes.push(rejected(
                    self.register_node(node),
                    "missing_material_resource",
                ));
                continue;
            };
            let repair = original_geometry.repair_for_photography();
            if let Some(reason) = repair.rejected_reason {
                report.issues.push(asset_issue(
                    PhotographicAssetIssueClassV1::Unrecoverable,
                    reason,
                    Some(self.register_node(node)),
                    "mesh geometry cannot produce a coherent visible surface",
                    Some("supply finite, indexed, nondegenerate replacement geometry"),
                ));
                report
                    .rejected_meshes
                    .push(rejected(self.register_node(node), reason));
                report.coherent_visible_subject = false;
                continue;
            }
            report.removed_degenerate_triangles += repair.removed_degenerate_triangles;
            report.repaired_normal_meshes += usize::from(repair.repaired_normals);
            report.reversed_winding_meshes += usize::from(repair.reversed_winding);
            report.disconnected_meshes += usize::from(repair.disconnected_components > 1);
            report.maximum_disconnected_components = report
                .maximum_disconnected_components
                .max(repair.disconnected_components);
            report.boundary_edges += repair.boundary_edges;
            report.nonmanifold_edges += repair.nonmanifold_edges;
            report.folded_edges += repair.folded_edges;
            report.self_intersections += repair.self_intersections;
            report.duplicate_vertices_removed += repair.duplicate_vertices_removed;
            if repair.removed_degenerate_triangles > 0 {
                report.issues.push(asset_issue(
                    PhotographicAssetIssueClassV1::SafeRepair,
                    "degenerate_triangles_removed",
                    Some(self.register_node(node)),
                    "degenerate triangles were isolated from photographic shading",
                    None,
                ));
            }
            if repair.repaired_normals || repair.reversed_winding {
                report.issues.push(asset_issue(
                    PhotographicAssetIssueClassV1::SafeRepair,
                    "surface_orientation_reconstructed",
                    Some(self.register_node(node)),
                    "invalid normals or unambiguous inverted winding were reconstructed",
                    None,
                ));
            }
            if repair.duplicate_vertices_removed > 0 {
                report.issues.push(asset_issue(
                    PhotographicAssetIssueClassV1::SafeRepair,
                    "duplicate_vertices_removed",
                    Some(self.register_node(node)),
                    "exact duplicate vertices without authored per-vertex data were merged",
                    None,
                ));
            }
            if repair.nonmanifold_edges > 0 {
                report.issues.push(asset_issue(
                    PhotographicAssetIssueClassV1::AppearanceChangeRequired,
                    "nonmanifold_geometry",
                    Some(self.register_node(node)),
                    "mesh has edges shared by more than two faces",
                    Some("supply a manifold replacement mesh or explicitly accept the appearance change"),
                ));
            }
            if repair.folded_edges > 0 {
                report.issues.push(asset_issue(
                    PhotographicAssetIssueClassV1::AppearanceChangeRequired,
                    "folded_geometry",
                    Some(self.register_node(node)),
                    "mesh contains inconsistently oriented or fully folded adjacent faces",
                    Some("supply corrected topology or explicitly accept an appearance-changing repair"),
                ));
            }
            if repair.self_intersections > 0 {
                report.issues.push(asset_issue(
                    PhotographicAssetIssueClassV1::AppearanceChangeRequired,
                    "self_intersecting_geometry",
                    Some(self.register_node(node)),
                    "mesh contains non-adjacent faces that cross",
                    Some("supply non-self-intersecting replacement geometry"),
                ));
            }
            if repair.disconnected_components > 32 {
                report.issues.push(asset_issue(
                    PhotographicAssetIssueClassV1::AppearanceChangeRequired,
                    "severely_disconnected_geometry",
                    Some(self.register_node(node)),
                    "mesh contains an unusually large number of disconnected components",
                    Some("confirm that every disconnected component belongs to the product"),
                ));
            }
            let mut geometry = repair.geometry.unwrap_or_else(|| original_geometry.clone());
            let mut geometry_changed = geometry != original_geometry;

            let preserve_sharp = self.scene.has_tag(node, PRESERVE_SHARP_EDGES_TAG);
            if preserve_sharp {
                report.preserved_sharp_meshes += 1;
            }
            let material_textured = material_has_authored_surface_texture(&original_material);
            let deforming = geometry.skin().is_some() || !geometry.morph_targets().is_empty();
            if !preserve_sharp && !material_textured && !deforming {
                let size = geometry.bounds().half_extent() * 2.0;
                let bevel = size.min_element().max(0.0) * 0.006;
                if bevel > 1.0e-7
                    && let Some(beveled) = geometry.micro_beveled_box(bevel)
                {
                    geometry = beveled;
                    geometry_changed = true;
                    report.micro_beveled_meshes += 1;
                    report.max_bevel_m = report.max_bevel_m.max(bevel);
                }
            }
            if original_material.normal_texture().is_some() && geometry.tangents().is_none() {
                report.generated_tangent_frames += 1;
            }
            if geometry_changed {
                let geometry = self.assets.create_geometry(geometry);
                self.scene.set_mesh_geometry(node, geometry)?;
            }

            let mut material = original_material.clone();
            for texture in material_texture_handles(&material) {
                let Some(texture) = self.assets.texture(texture) else {
                    report.issues.push(asset_issue(
                        PhotographicAssetIssueClassV1::Unrecoverable,
                        "missing_texture_resource",
                        Some(self.register_node(node)),
                        "material references texture data that is unavailable",
                        Some("supply the referenced texture or remove the invalid texture assignment"),
                    ));
                    report.coherent_visible_subject = false;
                    continue;
                };
                if let Some((width, height)) = texture.decoded_dimensions() {
                    let minimum = width.min(height);
                    report.minimum_texture_dimension = Some(
                        report
                            .minimum_texture_dimension
                            .map_or(minimum, |current| current.min(minimum)),
                    );
                }
            }
            let uniform_surface = material.kind() == MaterialKind::PbrMetallicRoughness
                && !material_has_authored_surface_texture(&material);
            if uniform_surface && material.photographic_micro_surface().is_none() {
                let strength = (0.018 + material.roughness_factor() * 0.025).clamp(0.018, 0.045);
                material = material.with_photographic_micro_surface(strength, micro_scale_m);
                let material = self.assets.create_material(material);
                self.scene.set_mesh_material(node, material)?;
                report.micro_surface_materials += 1;
                report.issues.push(asset_issue(
                    PhotographicAssetIssueClassV1::SafeRepair,
                    "neutral_micro_surface_added",
                    Some(self.register_node(node)),
                    "scale-aware neutral micro-roughness was added without changing material identity",
                    None,
                ));
            }
        }
        Ok(report)
    }
}

fn material_texture_handles(material: &MaterialDesc) -> Vec<crate::TextureHandle> {
    [
        material.base_color_texture(),
        material.normal_texture(),
        material.metallic_roughness_texture(),
        material.occlusion_texture(),
        material.emissive_texture(),
        material.clearcoat_texture(),
        material.clearcoat_roughness_texture(),
        material.clearcoat_normal_texture(),
        material.sheen_color_texture(),
        material.sheen_roughness_texture(),
        material.anisotropy_texture(),
        material.iridescence_texture(),
        material.iridescence_thickness_texture(),
        material.transmission_texture(),
        material.thickness_texture(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn asset_issue(
    class: PhotographicAssetIssueClassV1,
    code: &str,
    node: Option<u64>,
    message: &str,
    required_input: Option<&str>,
) -> PhotographicAssetIssueV1 {
    PhotographicAssetIssueV1 {
        class,
        code: code.to_owned(),
        node,
        message: message.to_owned(),
        required_input: required_input.map(str::to_owned),
    }
}

fn material_has_authored_surface_texture(material: &MaterialDesc) -> bool {
    material.base_color_texture().is_some()
        || material.normal_texture().is_some()
        || material.metallic_roughness_texture().is_some()
        || material.occlusion_texture().is_some()
        || material.emissive_texture().is_some()
        || material.clearcoat_texture().is_some()
        || material.clearcoat_roughness_texture().is_some()
        || material.clearcoat_normal_texture().is_some()
        || material.sheen_color_texture().is_some()
        || material.sheen_roughness_texture().is_some()
        || material.anisotropy_texture().is_some()
        || material.iridescence_texture().is_some()
        || material.iridescence_thickness_texture().is_some()
        || material.transmission_texture().is_some()
        || material.thickness_texture().is_some()
}

fn rejected(node: u64, reason: &str) -> PhotographicSurfaceRejectedMeshV1 {
    PhotographicSurfaceRejectedMeshV1 {
        node,
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, GeometryDesc, SceneHostCore};

    #[test]
    fn photographic_asset_health_reports_safe_repairs_and_supported_promise() {
        let mut host = SceneHostCore::headless(64, 64).expect("host builds");
        let geometry = host.assets.create_geometry(GeometryDesc::plane(1.0, 1.0));
        let material = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(Color::GRAY, 0.0, 0.7));
        let node = host
            .scene
            .mesh(geometry, material)
            .add()
            .expect("mesh inserts");
        let handle = host.register_node(node);

        let report = host
            .apply_photographic_surface(handle)
            .expect("coherent open surface can be assessed and safely improved");
        assert!(report.coherent_visible_subject);
        assert!(report.boundary_edges > 0);
        assert!(report.micro_surface_materials > 0);
        assert!(report.issues.iter().any(|issue| {
            issue.class == PhotographicAssetIssueClassV1::SafeRepair
                && issue.code == "neutral_micro_surface_added"
        }));
        assert!(report.supported_promise.contains("coherent geometry"));
        assert!(
            report
                .inspection_scope
                .contains(&"scene_hierarchy".to_owned())
        );
    }

    #[test]
    fn photographic_asset_health_reports_hidden_and_duplicate_components() {
        let mut host = SceneHostCore::headless(64, 64).expect("host builds");
        let geometry = host
            .assets
            .create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
        let material = host.assets.create_material(
            MaterialDesc::pbr_metallic_roughness(Color::GRAY, 0.0, 0.7)
                .with_roughness_factor(f32::NAN),
        );
        let root = host
            .scene
            .add_empty(host.scene.root(), crate::Transform::IDENTITY)
            .expect("subject root inserts");
        host.scene
            .mesh(geometry, material)
            .parent(root)
            .add()
            .expect("visible component inserts");
        let hidden = host
            .scene
            .mesh(geometry, material)
            .parent(root)
            .add()
            .expect("hidden duplicate inserts");
        host.scene
            .set_visible(hidden, false)
            .expect("component hides");

        let handle = host.register_node(root);
        let report = host
            .apply_photographic_surface(handle)
            .expect("asset health report builds");
        for code in ["hidden_subject_component", "duplicate_subject_component"] {
            assert!(
                report.issues.iter().any(|issue| issue.code == code),
                "missing issue {code}: {:?}",
                report.issues
            );
        }
        assert!(report.coherent_visible_subject);
    }
}
