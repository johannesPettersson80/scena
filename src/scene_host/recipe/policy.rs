use super::error_diagnostic;
use crate::assets::{AssetLoadReport, DefaultAssetFetcher, SceneAsset, TextureHandle};
use crate::material::MaterialDesc;
use crate::scene::recipe::{RecipeBuildPolicy, SceneRecipeDiagnosticV1};
use crate::scene_host::SceneHostCore;
use crate::{GeometryHandle, MaterialHandle};

pub(super) fn asset_policy_diagnostics(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    report: &AssetLoadReport<SceneAsset>,
    import_path: &str,
) -> Vec<SceneRecipeDiagnosticV1> {
    let mut diagnostics = Vec::new();
    let fetched_bytes = report.fetched_bytes()
        + report
            .external_resources()
            .iter()
            .filter_map(|resource| resource.bytes)
            .sum::<usize>();
    if fetched_bytes > policy.fetch_byte_limit() {
        diagnostics.push(error_diagnostic(
            import_path,
            "policy_violation",
            format!(
                "import fetched {fetched_bytes} bytes, exceeding RecipeBuildPolicy fetch_byte_limit {}",
                policy.fetch_byte_limit()
            ),
            "use a smaller asset or raise the operator-owned fetch_byte_limit policy",
        ));
    }

    let asset = report.asset();
    let node_count = asset.node_count();
    if node_count > policy.max_nodes() {
        diagnostics.push(error_diagnostic(
            import_path,
            "policy_violation",
            format!(
                "import contains {node_count} nodes, exceeding RecipeBuildPolicy max_nodes {}",
                policy.max_nodes()
            ),
            "use a smaller asset or raise the operator-owned max_nodes policy",
        ));
    }

    let instance_count = asset
        .nodes()
        .iter()
        .map(|node| node.instance_transforms().len())
        .sum::<usize>();
    if instance_count > policy.max_instances() {
        diagnostics.push(error_diagnostic(
            import_path,
            "policy_violation",
            format!(
                "import contains {instance_count} authored instances, exceeding RecipeBuildPolicy max_instances {}",
                policy.max_instances()
            ),
            "use fewer instances or raise the operator-owned max_instances policy",
        ));
    }

    let mut geometries = Vec::<GeometryHandle>::new();
    let mut materials = Vec::<MaterialHandle>::new();
    for node in asset.nodes() {
        for mesh in node.meshes() {
            push_unique(&mut geometries, mesh.geometry());
            push_unique(&mut materials, mesh.material());
            for binding in mesh.material_variant_bindings() {
                push_unique(&mut materials, binding.material());
            }
        }
    }

    check_material_count(policy, import_path, materials.len(), &mut diagnostics);
    check_geometry_budget(policy, host, import_path, geometries, &mut diagnostics);
    check_texture_budget(policy, host, import_path, materials, &mut diagnostics);
    diagnostics
}

fn check_material_count(
    policy: &RecipeBuildPolicy,
    import_path: &str,
    count: usize,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if count > policy.max_materials() {
        diagnostics.push(error_diagnostic(
            import_path,
            "policy_violation",
            format!(
                "import references {count} materials, exceeding RecipeBuildPolicy max_materials {}",
                policy.max_materials()
            ),
            "use fewer materials or raise the operator-owned max_materials policy",
        ));
    }
}

fn check_geometry_budget(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    import_path: &str,
    geometries: Vec<GeometryHandle>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let mut vertex_count = 0usize;
    let mut index_count = 0usize;
    for geometry in geometries {
        if let Some(desc) = host.assets.geometry(geometry) {
            vertex_count = vertex_count.saturating_add(desc.vertices().len());
            index_count = index_count.saturating_add(desc.indices().len());
        }
    }
    if vertex_count > policy.max_vertices() {
        diagnostics.push(error_diagnostic(
            import_path,
            "policy_violation",
            format!(
                "import references {vertex_count} vertices, exceeding RecipeBuildPolicy max_vertices {}",
                policy.max_vertices()
            ),
            "use lower-detail geometry or raise the operator-owned max_vertices policy",
        ));
    }
    if index_count > policy.max_indices() {
        diagnostics.push(error_diagnostic(
            import_path,
            "policy_violation",
            format!(
                "import references {index_count} indices, exceeding RecipeBuildPolicy max_indices {}",
                policy.max_indices()
            ),
            "use lower-detail geometry or raise the operator-owned max_indices policy",
        ));
    }
}

fn check_texture_budget(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    import_path: &str,
    materials: Vec<MaterialHandle>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let mut textures = Vec::<TextureHandle>::new();
    for material in materials {
        if let Some(desc) = host.assets.material(material) {
            collect_material_textures(&desc, &mut textures);
        }
    }
    if textures.len() > policy.max_textures() {
        diagnostics.push(error_diagnostic(
            import_path,
            "policy_violation",
            format!(
                "import references {} textures, exceeding RecipeBuildPolicy max_textures {}",
                textures.len(),
                policy.max_textures()
            ),
            "use fewer textures or raise the operator-owned max_textures policy",
        ));
    }
    let decoded_texture_bytes =
        check_texture_dimensions(policy, host, import_path, &textures, diagnostics);
    if decoded_texture_bytes > policy.max_texture_bytes() {
        diagnostics.push(error_diagnostic(
            import_path,
            "policy_violation",
            format!(
                "decoded textures use {decoded_texture_bytes} bytes, exceeding RecipeBuildPolicy max_texture_bytes {}",
                policy.max_texture_bytes()
            ),
            "use smaller textures or raise the operator-owned max_texture_bytes policy",
        ));
    }
}

fn check_texture_dimensions(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    import_path: &str,
    textures: &[TextureHandle],
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> usize {
    let mut decoded_texture_bytes = 0usize;
    for texture in textures {
        let Some(desc) = host.assets.texture(*texture) else {
            continue;
        };
        if let Some((width, height, rgba8)) = desc.decoded_rgba8() {
            decoded_texture_bytes = decoded_texture_bytes.saturating_add(rgba8.len());
            check_texture_dimension(policy, import_path, width, height, diagnostics);
        } else if let Some((width, height)) = desc.decoded_dimensions() {
            check_texture_dimension(policy, import_path, width, height, diagnostics);
        }
    }
    decoded_texture_bytes
}

fn check_texture_dimension(
    policy: &RecipeBuildPolicy,
    import_path: &str,
    width: u32,
    height: u32,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let max_dimension = width.max(height);
    if max_dimension > policy.max_image_dimension() {
        diagnostics.push(error_diagnostic(
            import_path,
            "policy_violation",
            format!(
                "texture dimensions {width}x{height} exceed RecipeBuildPolicy max_image_dimension {}",
                policy.max_image_dimension()
            ),
            "use smaller textures or raise the operator-owned max_image_dimension policy",
        ));
    }
}

fn collect_material_textures(material: &MaterialDesc, textures: &mut Vec<TextureHandle>) {
    for texture in [
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
    {
        push_unique(textures, texture);
    }
}

fn push_unique<T: Copy + PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}
