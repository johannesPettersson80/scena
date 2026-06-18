use std::collections::BTreeMap;

use super::common::{DiagnosticPathExt, authored_color};
use crate::assets::DefaultAssetFetcher;
use crate::material::{AlphaMode, MaterialDesc, TextureColorSpace};
use crate::scene::recipe::{
    RecipeBuildPolicy, SceneRecipeAlphaModeV1, SceneRecipeBuildResourceV1, SceneRecipeColorV1,
    SceneRecipeDiagnosticV1, SceneRecipeMaterialV1, SceneRecipeTextureColorSpaceV1,
    SceneRecipeTextureSlotV1,
};
use crate::scene_host::SceneHostCore;
use crate::{AssetPath, Color, MaterialHandle};

use super::super::error_diagnostic;

pub(in crate::scene_host::recipe) async fn build_authored_materials(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    recipes: &[SceneRecipeMaterialV1],
    manifest: &mut Vec<SceneRecipeBuildResourceV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> BTreeMap<String, MaterialHandle> {
    let mut handles = BTreeMap::new();
    if recipes.len() > policy.max_materials() {
        diagnostics.push(error_diagnostic(
            "$.materials",
            "policy_violation",
            format!(
                "recipe declares {} authored materials, exceeding RecipeBuildPolicy max_materials {}",
                recipes.len(),
                policy.max_materials()
            ),
            "reduce material count or raise the operator-owned max_materials policy",
        ));
        return handles;
    }
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.materials[{index}]");
        let base_color = match authored_color(colors, &recipe.base_color) {
            Ok(color) => color,
            Err(diagnostic) => {
                diagnostics.push((*diagnostic).with_path(format!("{path}.base_color")));
                continue;
            }
        };
        let (kind, material) =
            match authored_material(policy, host, recipe_path, colors, recipe, base_color, &path)
                .await
            {
                Ok(value) => value,
                Err(diagnostic) => {
                    diagnostics.push(*diagnostic);
                    continue;
                }
            };
        let handle = host.assets.create_material(material);
        handles.insert(recipe.id.clone(), handle);
        manifest.push(SceneRecipeBuildResourceV1 {
            id: recipe.id.clone(),
            kind,
            vertex_count: None,
            index_count: None,
        });
    }
    handles
}

async fn authored_material(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    recipe: &SceneRecipeMaterialV1,
    base_color: Color,
    path: &str,
) -> Result<(String, MaterialDesc), Box<SceneRecipeDiagnosticV1>> {
    let (kind, mut material) = match recipe.kind.as_str() {
        "unlit" => ("unlit", MaterialDesc::unlit(base_color)),
        "pbr_metallic_roughness" => (
            "pbr_metallic_roughness",
            MaterialDesc::pbr_metallic_roughness(
                base_color,
                recipe.metallic.unwrap_or(0.0) as f32,
                recipe.roughness.unwrap_or(1.0) as f32,
            ),
        ),
        "line" => (
            "line",
            MaterialDesc::line(base_color, recipe.stroke_width_px.unwrap_or(1.0) as f32),
        ),
        "wireframe" => (
            "wireframe",
            MaterialDesc::wireframe(base_color, recipe.stroke_width_px.unwrap_or(1.0) as f32),
        ),
        "edge" => {
            let mut material =
                MaterialDesc::edge(base_color, recipe.stroke_width_px.unwrap_or(1.0) as f32);
            if let Some(threshold) = recipe.edge_angle_threshold_degrees {
                material = material.with_edge_angle_threshold_degrees(threshold as f32);
            }
            ("edge", material)
        }
        kind => {
            return Err(Box::new(error_diagnostic(
                path,
                "unsupported_feature",
                format!("material kind '{kind}' is not implemented in this slice"),
                "use kind:\"unlit\", \"pbr_metallic_roughness\", \"line\", \"wireframe\", or \"edge\"",
            )));
        }
    };
    material = material.with_double_sided(recipe.double_sided);
    if let Some(emissive) = &recipe.emissive {
        material =
            material.with_emissive(authored_color(colors, emissive).map_err(|diagnostic| {
                Box::new((*diagnostic).with_path(format!("{path}.emissive")))
            })?);
    }
    if let Some(strength) = recipe.emissive_strength {
        material = material.with_emissive_strength(strength as f32);
    }
    if let Some(alpha_mode) = &recipe.alpha_mode {
        material = material.with_alpha_mode(match alpha_mode {
            SceneRecipeAlphaModeV1::Opaque => AlphaMode::Opaque,
            SceneRecipeAlphaModeV1::Mask { cutoff } => AlphaMode::Mask {
                cutoff: *cutoff as f32,
            },
            SceneRecipeAlphaModeV1::Blend => AlphaMode::Blend,
        });
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.base_color_texture.as_ref(),
        &format!("{path}.base_color_texture"),
        TextureColorSpace::Srgb,
    )
    .await?
    {
        material = material.with_base_color_texture(texture);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.normal_texture.as_ref(),
        &format!("{path}.normal_texture"),
        TextureColorSpace::Linear,
    )
    .await?
    {
        material = material.with_normal_texture(texture);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.metallic_roughness_texture.as_ref(),
        &format!("{path}.metallic_roughness_texture"),
        TextureColorSpace::Linear,
    )
    .await?
    {
        material = material.with_metallic_roughness_texture(texture);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.occlusion_texture.as_ref(),
        &format!("{path}.occlusion_texture"),
        TextureColorSpace::Linear,
    )
    .await?
    {
        material = material.with_occlusion_texture(texture);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.emissive_texture.as_ref(),
        &format!("{path}.emissive_texture"),
        TextureColorSpace::Srgb,
    )
    .await?
    {
        material = material.with_emissive_texture(texture);
    }
    if let Some(value) = recipe.clearcoat_factor {
        material = material.with_clearcoat_factor(value as f32);
    }
    if let Some(value) = recipe.clearcoat_roughness_factor {
        material = material.with_clearcoat_roughness_factor(value as f32);
    }
    if let Some(value) = recipe.clearcoat_normal_scale {
        material = material.with_clearcoat_normal_scale(value as f32);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.clearcoat_texture.as_ref(),
        &format!("{path}.clearcoat_texture"),
        TextureColorSpace::Linear,
    )
    .await?
    {
        material = material.with_clearcoat_texture(texture);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.clearcoat_roughness_texture.as_ref(),
        &format!("{path}.clearcoat_roughness_texture"),
        TextureColorSpace::Linear,
    )
    .await?
    {
        material = material.with_clearcoat_roughness_texture(texture);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.clearcoat_normal_texture.as_ref(),
        &format!("{path}.clearcoat_normal_texture"),
        TextureColorSpace::Linear,
    )
    .await?
    {
        material = material.with_clearcoat_normal_texture(texture);
    }
    if let Some(color) = &recipe.sheen_color_factor {
        material = material.with_sheen_color_factor(authored_color(colors, color).map_err(
            |diagnostic| Box::new((*diagnostic).with_path(format!("{path}.sheen_color_factor"))),
        )?);
    }
    if let Some(value) = recipe.sheen_roughness_factor {
        material = material.with_sheen_roughness_factor(value as f32);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.sheen_color_texture.as_ref(),
        &format!("{path}.sheen_color_texture"),
        TextureColorSpace::Srgb,
    )
    .await?
    {
        material = material.with_sheen_color_texture(texture);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.sheen_roughness_texture.as_ref(),
        &format!("{path}.sheen_roughness_texture"),
        TextureColorSpace::Linear,
    )
    .await?
    {
        material = material.with_sheen_roughness_texture(texture);
    }
    if let Some(value) = recipe.anisotropy_strength_factor {
        material = material.with_anisotropy_strength_factor(value as f32);
    }
    if let Some(value) = recipe.anisotropy_rotation_radians {
        material = material.with_anisotropy_rotation_radians(value as f32);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.anisotropy_texture.as_ref(),
        &format!("{path}.anisotropy_texture"),
        TextureColorSpace::Linear,
    )
    .await?
    {
        material = material.with_anisotropy_texture(texture);
    }
    if let Some(value) = recipe.iridescence_factor {
        material = material.with_iridescence_factor(value as f32);
    }
    if let Some(value) = recipe.iridescence_ior {
        material = material.with_iridescence_ior(value as f32);
    }
    if recipe.iridescence_thickness_minimum_nm.is_some()
        || recipe.iridescence_thickness_maximum_nm.is_some()
    {
        let minimum_nm = recipe
            .iridescence_thickness_minimum_nm
            .unwrap_or_else(|| f64::from(material.iridescence_thickness_minimum_nm()));
        let maximum_nm = recipe
            .iridescence_thickness_maximum_nm
            .unwrap_or_else(|| f64::from(material.iridescence_thickness_maximum_nm()));
        material =
            material.with_iridescence_thickness_range_nm(minimum_nm as f32, maximum_nm as f32);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.iridescence_texture.as_ref(),
        &format!("{path}.iridescence_texture"),
        TextureColorSpace::Linear,
    )
    .await?
    {
        material = material.with_iridescence_texture(texture);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.iridescence_thickness_texture.as_ref(),
        &format!("{path}.iridescence_thickness_texture"),
        TextureColorSpace::Linear,
    )
    .await?
    {
        material = material.with_iridescence_thickness_texture(texture);
    }
    if let Some(value) = recipe.dispersion_factor {
        material = material.with_dispersion_factor(value as f32);
    }
    if let Some(value) = recipe.transmission_factor {
        material = material.with_transmission_factor(value as f32);
    }
    if let Some(value) = recipe.ior {
        material = material.with_ior(value as f32);
    }
    if let Some(value) = recipe.thickness_factor {
        material = material.with_thickness_factor(value as f32);
    }
    if let Some(value) = recipe.attenuation_distance {
        material = material.with_attenuation_distance(value as f32);
    }
    if let Some(color) = &recipe.attenuation_color {
        material = material.with_attenuation_color(authored_color(colors, color).map_err(
            |diagnostic| Box::new((*diagnostic).with_path(format!("{path}.attenuation_color"))),
        )?);
    }
    reject_gpu_unsupported_volume_texture(
        recipe.transmission_texture.as_ref(),
        path,
        "transmission_texture",
    )?;
    reject_gpu_unsupported_volume_texture(
        recipe.thickness_texture.as_ref(),
        path,
        "thickness_texture",
    )?;
    Ok((kind.to_owned(), material))
}

fn reject_gpu_unsupported_volume_texture(
    slot: Option<&SceneRecipeTextureSlotV1>,
    path: &str,
    field: &str,
) -> Result<(), Box<SceneRecipeDiagnosticV1>> {
    if slot.is_none() {
        return Ok(());
    }
    Err(Box::new(error_diagnostic(
        format!("{path}.{field}"),
        "unsupported_feature",
        format!(
            "{field} is not exposed by scene_recipe.v1 until the GPU path supports it without exceeding the WebGL2 texture-unit floor"
        ),
        "remove this texture slot; transmission_factor remains supported for recipe-authored glass",
    )))
}

async fn load_texture_slot(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    slot: Option<&SceneRecipeTextureSlotV1>,
    path: &str,
    default_color_space: TextureColorSpace,
) -> Result<Option<crate::TextureHandle>, Box<SceneRecipeDiagnosticV1>> {
    let Some(slot) = slot else {
        return Ok(None);
    };
    let color_space = match slot.color_space {
        Some(SceneRecipeTextureColorSpaceV1::Srgb) => TextureColorSpace::Srgb,
        Some(SceneRecipeTextureColorSpaceV1::Linear) => TextureColorSpace::Linear,
        None => default_color_space,
    };
    let resolved = policy.resolve_import_uri(recipe_path, &slot.uri, format!("{path}.uri"))?;
    match host
        .assets
        .load_texture(AssetPath::from(resolved.as_str()), color_space)
        .await
    {
        Ok(texture) => Ok(Some(texture)),
        Err(error) if slot.optional => Ok(None),
        Err(error) => Err(Box::new(error_diagnostic(
            path,
            "texture_load_failed",
            format!(
                "required texture '{}' could not be loaded: {error}",
                slot.uri
            ),
            "fix the texture uri or mark the texture slot optional only if the fallback is acceptable",
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::{Value, json};

    use super::*;
    use crate::{Assets, Color, DirectionalLight, GeometryDesc, Renderer, Scene, Transform, Vec3};

    #[test]
    fn authored_advanced_pbr_recipe_fields_map_to_material_descriptor() {
        let texture = "gltf/khronos/WaterBottle/WaterBottle_baseColor.png";
        let material = recipe_material(json!({
            "id": "advanced",
            "kind": "pbr_metallic_roughness",
            "base_color": "base",
            "metallic": 0.0,
            "roughness": 0.42,
            "clearcoat_factor": 0.8,
            "clearcoat_roughness_factor": 0.16,
            "clearcoat_normal_scale": 1.25,
            "sheen_color_factor": "white",
            "sheen_roughness_factor": 0.35,
            "anisotropy_strength_factor": 0.65,
            "anisotropy_rotation_radians": 0.3,
            "iridescence_factor": 0.45,
            "iridescence_ior": 1.45,
            "iridescence_thickness_minimum_nm": 120.0,
            "iridescence_thickness_maximum_nm": 480.0,
            "dispersion_factor": 0.02,
            "transmission_factor": 0.18,
            "ior": 1.52,
            "thickness_factor": 0.35,
            "attenuation_distance": 2.0,
            "attenuation_color": "blue",
            "clearcoat_texture": { "uri": texture, "color_space": "linear" },
            "clearcoat_roughness_texture": { "uri": texture, "color_space": "linear" },
            "clearcoat_normal_texture": { "uri": texture, "color_space": "linear" },
            "sheen_color_texture": { "uri": texture, "color_space": "srgb" },
            "sheen_roughness_texture": { "uri": texture, "color_space": "linear" },
            "anisotropy_texture": { "uri": texture, "color_space": "linear" },
            "iridescence_texture": { "uri": texture, "color_space": "linear" },
            "iridescence_thickness_texture": { "uri": texture, "color_space": "linear" }
        }));

        assert_close(material.clearcoat_factor(), 0.8);
        assert_close(material.clearcoat_roughness_factor(), 0.16);
        assert_close(material.clearcoat_normal_scale(), 1.25);
        assert_close(material.sheen_color_factor().r, 1.0);
        assert_close(material.sheen_roughness_factor(), 0.35);
        assert_close(material.anisotropy_strength_factor(), 0.65);
        assert_close(material.anisotropy_rotation_radians(), 0.3);
        assert_close(material.iridescence_factor(), 0.45);
        assert_close(material.iridescence_ior(), 1.45);
        assert_close(material.iridescence_thickness_minimum_nm(), 120.0);
        assert_close(material.iridescence_thickness_maximum_nm(), 480.0);
        assert_close(material.dispersion_factor(), 0.02);
        assert_close(material.transmission_factor(), 0.18);
        assert_close(material.ior(), 1.52);
        assert_close(material.thickness_factor(), 0.35);
        assert_close(material.attenuation_distance(), 2.0);
        assert!(material.attenuation_color().b > material.attenuation_color().r);
        assert!(material.clearcoat_texture().is_some());
        assert!(material.clearcoat_roughness_texture().is_some());
        assert!(material.clearcoat_normal_texture().is_some());
        assert!(material.sheen_color_texture().is_some());
        assert!(material.sheen_roughness_texture().is_some());
        assert!(material.anisotropy_texture().is_some());
        assert!(material.iridescence_texture().is_some());
        assert!(material.iridescence_thickness_texture().is_some());
        assert!(material.transmission_texture().is_none());
        assert!(material.thickness_texture().is_none());
    }

    #[test]
    fn authored_advanced_pbr_recipe_ior_matches_public_setter_domain() {
        let zero = recipe_material(json!({
            "id": "advanced",
            "kind": "pbr_metallic_roughness",
            "base_color": "base",
            "ior": 0.0
        }));
        assert_close(zero.ior(), 0.0);

        let boundary = recipe_material(json!({
            "id": "advanced",
            "kind": "pbr_metallic_roughness",
            "base_color": "base",
            "ior": 1.0
        }));
        assert_close(boundary.ior(), 1.0);
    }

    #[test]
    fn authored_advanced_pbr_recipe_rejects_gpu_unsupported_volume_texture_fields() {
        for (field, value) in [
            (
                "transmission_texture",
                json!({
                    "uri": "gltf/khronos/WaterBottle/WaterBottle_baseColor.png",
                    "color_space": "linear"
                }),
            ),
            (
                "thickness_texture",
                json!({
                    "uri": "gltf/khronos/WaterBottle/WaterBottle_baseColor.png",
                    "color_space": "linear"
                }),
            ),
        ] {
            let mut material = json!({
                "id": "advanced",
                "kind": "pbr_metallic_roughness",
                "base_color": "base"
            });
            material
                .as_object_mut()
                .expect("material recipe is an object")
                .insert(field.to_owned(), value);
            let result = try_recipe_material(material);
            let error = result.expect_err("GPU-unsupported recipe volume field should fail closed");
            assert_eq!(error.code, "unsupported_feature");
            assert_eq!(error.path, format!("$.materials[0].{field}"));
        }
    }

    #[test]
    fn authored_advanced_pbr_recipe_scalars_each_change_headless_gpu_pixels() {
        struct ScalarGpuCase {
            name: &'static str,
            baseline: Vec<(&'static str, Value)>,
            changed: Vec<(&'static str, Value)>,
            min_delta: u64,
        }

        let linear_texture = json!({
            "uri": "gltf/khronos/WaterBottle/WaterBottle_baseColor.png",
            "color_space": "linear"
        });
        let cases = vec![
            ScalarGpuCase {
                name: "clearcoat_factor",
                baseline: vec![
                    ("roughness", json!(0.24)),
                    ("clearcoat_factor", json!(0.0)),
                    ("clearcoat_roughness_factor", json!(0.12)),
                ],
                changed: vec![
                    ("roughness", json!(0.24)),
                    ("clearcoat_factor", json!(0.9)),
                    ("clearcoat_roughness_factor", json!(0.12)),
                ],
                min_delta: 16,
            },
            ScalarGpuCase {
                name: "clearcoat_roughness_factor",
                baseline: vec![
                    ("roughness", json!(0.18)),
                    ("clearcoat_factor", json!(0.9)),
                    ("clearcoat_roughness_factor", json!(0.02)),
                ],
                changed: vec![
                    ("roughness", json!(0.18)),
                    ("clearcoat_factor", json!(0.9)),
                    ("clearcoat_roughness_factor", json!(0.72)),
                ],
                min_delta: 16,
            },
            ScalarGpuCase {
                name: "clearcoat_normal_scale",
                baseline: vec![
                    ("roughness", json!(0.28)),
                    ("clearcoat_factor", json!(0.85)),
                    ("clearcoat_roughness_factor", json!(0.08)),
                    ("clearcoat_normal_scale", json!(0.0)),
                    ("clearcoat_normal_texture", linear_texture.clone()),
                ],
                changed: vec![
                    ("roughness", json!(0.28)),
                    ("clearcoat_factor", json!(0.85)),
                    ("clearcoat_roughness_factor", json!(0.08)),
                    ("clearcoat_normal_scale", json!(2.0)),
                    ("clearcoat_normal_texture", linear_texture.clone()),
                ],
                min_delta: 16,
            },
            ScalarGpuCase {
                name: "sheen_color_factor",
                baseline: vec![
                    ("roughness", json!(0.5)),
                    ("sheen_color_factor", json!("base")),
                    ("sheen_roughness_factor", json!(0.35)),
                ],
                changed: vec![
                    ("roughness", json!(0.5)),
                    ("sheen_color_factor", json!("blue")),
                    ("sheen_roughness_factor", json!(0.35)),
                ],
                min_delta: 16,
            },
            ScalarGpuCase {
                name: "sheen_roughness_factor",
                baseline: vec![
                    ("roughness", json!(0.46)),
                    ("sheen_color_factor", json!("white")),
                    ("sheen_roughness_factor", json!(0.02)),
                ],
                changed: vec![
                    ("roughness", json!(0.46)),
                    ("sheen_color_factor", json!("white")),
                    ("sheen_roughness_factor", json!(0.9)),
                ],
                min_delta: 16,
            },
            ScalarGpuCase {
                name: "anisotropy_strength_factor",
                baseline: vec![
                    ("roughness", json!(0.28)),
                    ("anisotropy_strength_factor", json!(0.0)),
                    ("anisotropy_rotation_radians", json!(0.65)),
                ],
                changed: vec![
                    ("roughness", json!(0.28)),
                    ("anisotropy_strength_factor", json!(0.9)),
                    ("anisotropy_rotation_radians", json!(0.65)),
                ],
                min_delta: 16,
            },
            ScalarGpuCase {
                name: "anisotropy_rotation_radians",
                baseline: vec![
                    ("roughness", json!(0.28)),
                    ("anisotropy_strength_factor", json!(0.85)),
                    ("anisotropy_rotation_radians", json!(0.0)),
                ],
                changed: vec![
                    ("roughness", json!(0.28)),
                    ("anisotropy_strength_factor", json!(0.85)),
                    ("anisotropy_rotation_radians", json!(1.25)),
                ],
                min_delta: 16,
            },
            ScalarGpuCase {
                name: "iridescence_factor",
                baseline: vec![
                    ("roughness", json!(0.34)),
                    ("iridescence_factor", json!(0.0)),
                    ("iridescence_ior", json!(1.45)),
                    ("iridescence_thickness_minimum_nm", json!(140.0)),
                    ("iridescence_thickness_maximum_nm", json!(560.0)),
                ],
                changed: vec![
                    ("roughness", json!(0.34)),
                    ("iridescence_factor", json!(0.85)),
                    ("iridescence_ior", json!(1.45)),
                    ("iridescence_thickness_minimum_nm", json!(140.0)),
                    ("iridescence_thickness_maximum_nm", json!(560.0)),
                ],
                min_delta: 16,
            },
            ScalarGpuCase {
                name: "iridescence_ior",
                baseline: vec![
                    ("roughness", json!(0.34)),
                    ("iridescence_factor", json!(0.8)),
                    ("iridescence_ior", json!(1.1)),
                    ("iridescence_thickness_minimum_nm", json!(140.0)),
                    ("iridescence_thickness_maximum_nm", json!(560.0)),
                ],
                changed: vec![
                    ("roughness", json!(0.34)),
                    ("iridescence_factor", json!(0.8)),
                    ("iridescence_ior", json!(2.0)),
                    ("iridescence_thickness_minimum_nm", json!(140.0)),
                    ("iridescence_thickness_maximum_nm", json!(560.0)),
                ],
                min_delta: 16,
            },
            ScalarGpuCase {
                name: "iridescence_thickness_minimum_nm",
                baseline: vec![
                    ("roughness", json!(0.34)),
                    ("iridescence_factor", json!(0.8)),
                    ("iridescence_ior", json!(1.45)),
                    ("iridescence_thickness_minimum_nm", json!(100.0)),
                    ("iridescence_thickness_maximum_nm", json!(650.0)),
                    ("iridescence_thickness_texture", linear_texture.clone()),
                ],
                changed: vec![
                    ("roughness", json!(0.34)),
                    ("iridescence_factor", json!(0.8)),
                    ("iridescence_ior", json!(1.45)),
                    ("iridescence_thickness_minimum_nm", json!(500.0)),
                    ("iridescence_thickness_maximum_nm", json!(650.0)),
                    ("iridescence_thickness_texture", linear_texture.clone()),
                ],
                min_delta: 16,
            },
            ScalarGpuCase {
                name: "iridescence_thickness_maximum_nm",
                baseline: vec![
                    ("roughness", json!(0.34)),
                    ("iridescence_factor", json!(0.8)),
                    ("iridescence_ior", json!(1.45)),
                    ("iridescence_thickness_minimum_nm", json!(120.0)),
                    ("iridescence_thickness_maximum_nm", json!(180.0)),
                ],
                changed: vec![
                    ("roughness", json!(0.34)),
                    ("iridescence_factor", json!(0.8)),
                    ("iridescence_ior", json!(1.45)),
                    ("iridescence_thickness_minimum_nm", json!(120.0)),
                    ("iridescence_thickness_maximum_nm", json!(700.0)),
                ],
                min_delta: 16,
            },
            ScalarGpuCase {
                name: "dispersion_factor",
                baseline: vec![
                    ("roughness", json!(0.22)),
                    ("transmission_factor", json!(0.65)),
                    ("ior", json!(1.55)),
                    ("dispersion_factor", json!(0.0)),
                ],
                changed: vec![
                    ("roughness", json!(0.22)),
                    ("transmission_factor", json!(0.65)),
                    ("ior", json!(1.55)),
                    ("dispersion_factor", json!(0.08)),
                ],
                min_delta: 16,
            },
            ScalarGpuCase {
                name: "transmission_factor",
                baseline: vec![
                    ("roughness", json!(0.18)),
                    ("alpha_mode", json!({ "kind": "blend" })),
                    ("transmission_factor", json!(0.0)),
                    ("ior", json!(1.55)),
                ],
                changed: vec![
                    ("roughness", json!(0.18)),
                    ("alpha_mode", json!({ "kind": "blend" })),
                    ("transmission_factor", json!(0.65)),
                    ("ior", json!(1.55)),
                ],
                min_delta: 16,
            },
            ScalarGpuCase {
                name: "ior",
                baseline: vec![
                    ("roughness", json!(0.22)),
                    ("dispersion_factor", json!(0.08)),
                    ("ior", json!(1.01)),
                ],
                changed: vec![
                    ("roughness", json!(0.22)),
                    ("dispersion_factor", json!(0.08)),
                    ("ior", json!(4.0)),
                ],
                min_delta: 16,
            },
        ];

        let mut failures = Vec::new();
        for case in cases {
            let baseline_frame =
                render_recipe_material_gpu(advanced_material_recipe("baseline", case.baseline));
            let changed_frame =
                render_recipe_material_gpu(advanced_material_recipe("changed", case.changed));
            let delta = rgba8_absolute_delta(&baseline_frame, &changed_frame);
            if delta < case.min_delta {
                failures.push(format!(
                    "{} delta {delta}, expected >= {}",
                    case.name, case.min_delta
                ));
            }
        }
        assert!(
            failures.is_empty(),
            "advanced PBR recipe scalar GPU attribution failures: {}",
            failures.join("; ")
        );
    }

    #[test]
    fn authored_advanced_pbr_recipe_volume_scalars_change_headless_gpu_pixels_in_coupled_scene() {
        let red_frame = render_recipe_material_gpu(advanced_material_recipe(
            "red_volume",
            coupled_volume_fields("red", 0.22),
        ));
        let blue_frame = render_recipe_material_gpu(advanced_material_recipe(
            "blue_volume",
            coupled_volume_fields("blue", 0.22),
        ));
        let (red_absorption, blue_absorption, color_changed_pixels, color_delta) =
            changed_region_rgb_averages(&red_frame, &blue_frame);
        assert!(
            color_delta >= 24 && color_changed_pixels >= 16,
            "attenuation_color must visibly change the transmitted region: delta={color_delta}, changed_pixels={color_changed_pixels}, red={red_absorption:?}, blue={blue_absorption:?}"
        );
        assert!(
            red_absorption[0] > blue_absorption[0] && blue_absorption[2] > red_absorption[2],
            "red vs blue KHR_materials_volume attenuation should bias different channels: red={red_absorption:?}, blue={blue_absorption:?}"
        );

        let strong_frame = render_recipe_material_gpu(advanced_material_recipe(
            "short_distance",
            coupled_volume_fields("red", 0.18),
        ));
        let weak_frame = render_recipe_material_gpu(advanced_material_recipe(
            "long_distance",
            coupled_volume_fields("red", 3.0),
        ));
        let (strong_absorption, weak_absorption, distance_changed_pixels, distance_delta) =
            changed_region_rgb_averages(&strong_frame, &weak_frame);
        assert!(
            distance_delta >= 18 && distance_changed_pixels >= 16,
            "attenuation_distance must change volume absorption when transmission and thickness are active: delta={distance_delta}, changed_pixels={distance_changed_pixels}, strong={strong_absorption:?}, weak={weak_absorption:?}"
        );
    }

    fn coupled_volume_fields(color: &str, attenuation_distance: f64) -> Vec<(&str, Value)> {
        vec![
            ("base_color", json!("white")),
            ("alpha_mode", json!({ "kind": "blend" })),
            ("roughness", json!(0.08)),
            ("transmission_factor", json!(1.0)),
            ("ior", json!(1.5)),
            ("thickness_factor", json!(1.0)),
            ("attenuation_distance", json!(attenuation_distance)),
            ("attenuation_color", json!(color)),
        ]
    }

    fn advanced_material_recipe(id: &str, fields: Vec<(&str, Value)>) -> Value {
        let mut material = json!({
            "id": id,
            "kind": "pbr_metallic_roughness",
            "base_color": "base",
            "metallic": 0.0,
            "roughness": 0.42,
            "double_sided": true
        });
        let object = material
            .as_object_mut()
            .expect("material recipe is an object");
        for (field, value) in fields {
            object.insert(field.to_owned(), value);
        }
        material
    }

    fn rgba8_absolute_delta(a: &[u8], b: &[u8]) -> u64 {
        assert_eq!(a.len(), b.len(), "frames must have matching dimensions");
        a.iter()
            .zip(b)
            .map(|(left, right)| u64::from(left.abs_diff(*right)))
            .sum()
    }

    fn changed_region_rgb_averages(left: &[u8], right: &[u8]) -> ([u8; 3], [u8; 3], usize, u64) {
        const WIDTH: usize = 64;
        const HEIGHT: usize = 64;
        assert_eq!(left.len(), WIDTH * HEIGHT * 4);
        assert_eq!(right.len(), left.len());
        let mut left_sum = [0u64; 3];
        let mut right_sum = [0u64; 3];
        let mut count = 0u64;
        let mut delta = 0u64;
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let offset = (y * WIDTH + x) * 4;
                let pixel_delta = (0..3)
                    .map(|channel| {
                        u64::from(left[offset + channel].abs_diff(right[offset + channel]))
                    })
                    .sum::<u64>();
                delta += pixel_delta;
                if pixel_delta >= 8 {
                    for channel in 0..3 {
                        left_sum[channel] += u64::from(left[offset + channel]);
                        right_sum[channel] += u64::from(right[offset + channel]);
                    }
                    count += 1;
                }
            }
        }
        if count == 0 {
            return ([0; 3], [0; 3], 0, delta);
        }
        (
            [
                (left_sum[0] / count) as u8,
                (left_sum[1] / count) as u8,
                (left_sum[2] / count) as u8,
            ],
            [
                (right_sum[0] / count) as u8,
                (right_sum[1] / count) as u8,
                (right_sum[2] / count) as u8,
            ],
            count as usize,
            delta,
        )
    }

    fn recipe_material(value: serde_json::Value) -> MaterialDesc {
        try_recipe_material(value).expect("recipe material builds")
    }

    fn render_recipe_material_gpu(value: Value) -> Vec<u8> {
        let recipe: SceneRecipeMaterialV1 =
            serde_json::from_value(value).expect("recipe material decodes");
        let colors = test_colors();
        let base_color = authored_color(&colors, &recipe.base_color).expect("base color resolves");
        let host = SceneHostCore::headless(64, 64).expect("host builds");
        let (_, material) = pollster::block_on(authored_material(
            &RecipeBuildPolicy::testing(),
            &host,
            "tests/assets/slice9.recipe.json",
            &colors,
            &recipe,
            base_color,
            "$.materials[0]",
        ))
        .expect("recipe material builds");
        render_material_with_assets_gpu(&host.assets, material)
    }

    fn try_recipe_material(
        value: serde_json::Value,
    ) -> Result<MaterialDesc, Box<SceneRecipeDiagnosticV1>> {
        let recipe: SceneRecipeMaterialV1 =
            serde_json::from_value(value).expect("recipe material decodes");
        let colors = test_colors();
        let base_color = authored_color(&colors, &recipe.base_color).expect("base color resolves");
        let host = SceneHostCore::headless(64, 64).expect("host builds");
        pollster::block_on(authored_material(
            &RecipeBuildPolicy::testing(),
            &host,
            "tests/assets/slice9.recipe.json",
            &colors,
            &recipe,
            base_color,
            "$.materials[0]",
        ))
        .map(|(_, material)| material)
    }

    fn render_material_with_assets_gpu(assets: &Assets, material: MaterialDesc) -> Vec<u8> {
        let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.65, 0.65, 0.05));
        let material = assets.create_material(material);
        let backdrop_geometry = assets.create_geometry(GeometryDesc::box_xyz(3.0, 3.0, 0.02));
        let backdrop = assets.create_material(MaterialDesc::unlit(Color::WHITE));
        let mut scene = Scene::new();
        scene
            .mesh(backdrop_geometry, backdrop)
            .transform(Transform::at(Vec3::new(0.0, 0.0, -0.28)))
            .add()
            .expect("backdrop mesh inserts");
        scene
            .mesh(geometry, material)
            .transform(Transform::at(Vec3::ZERO))
            .add()
            .expect("mesh inserts");
        scene
            .directional_light(DirectionalLight::default().with_illuminance_lux(18_000.0))
            .add()
            .expect("light inserts");
        let camera = scene.add_default_camera().expect("camera inserts");
        let mut renderer = Renderer::headless_gpu(64, 64).expect("HeadlessGpu renderer builds");
        renderer.set_background_color(Color::from_srgb_u8(18, 24, 32));
        renderer
            .prepare_with_assets(&mut scene, assets)
            .expect("scene prepares on HeadlessGpu");
        renderer
            .render(&scene, camera)
            .expect("scene renders on HeadlessGpu");
        renderer.frame_rgba8().to_vec()
    }

    fn test_colors() -> BTreeMap<String, SceneRecipeColorV1> {
        [
            (
                "base".to_owned(),
                SceneRecipeColorV1::Hex("#7C8798".to_owned()),
            ),
            (
                "white".to_owned(),
                SceneRecipeColorV1::Hex("#FFFFFF".to_owned()),
            ),
            (
                "blue".to_owned(),
                SceneRecipeColorV1::Hex("#BFD7FF".to_owned()),
            ),
            (
                "red".to_owned(),
                SceneRecipeColorV1::Hex("#FF4040".to_owned()),
            ),
            (
                "black".to_owned(),
                SceneRecipeColorV1::Hex("#000000".to_owned()),
            ),
        ]
        .into_iter()
        .collect()
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 1.0e-5,
            "expected {actual} to equal {expected}"
        );
    }
}
