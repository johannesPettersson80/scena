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
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.transmission_texture.as_ref(),
        &format!("{path}.transmission_texture"),
        TextureColorSpace::Linear,
    )
    .await?
    {
        material = material.with_transmission_texture(texture);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.thickness_texture.as_ref(),
        &format!("{path}.thickness_texture"),
        TextureColorSpace::Linear,
    )
    .await?
    {
        material = material.with_thickness_texture(texture);
    }
    Ok((kind.to_owned(), material))
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

    use serde_json::json;

    use super::*;
    use crate::{
        Assets, DirectionalLight, EnvironmentPreset, GeometryDesc, Renderer, Scene, Transform, Vec3,
    };

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
            "thickness_factor": 0.05,
            "attenuation_distance": 2.5,
            "attenuation_color": "blue",
            "clearcoat_texture": { "uri": texture, "color_space": "linear" },
            "clearcoat_roughness_texture": { "uri": texture, "color_space": "linear" },
            "clearcoat_normal_texture": { "uri": texture, "color_space": "linear" },
            "sheen_color_texture": { "uri": texture, "color_space": "srgb" },
            "sheen_roughness_texture": { "uri": texture, "color_space": "linear" },
            "anisotropy_texture": { "uri": texture, "color_space": "linear" },
            "iridescence_texture": { "uri": texture, "color_space": "linear" },
            "iridescence_thickness_texture": { "uri": texture, "color_space": "linear" },
            "transmission_texture": { "uri": texture, "color_space": "linear" },
            "thickness_texture": { "uri": texture, "color_space": "linear" }
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
        assert_close(material.thickness_factor(), 0.05);
        assert_close(material.attenuation_distance(), 2.5);
        assert_close(material.attenuation_color().b, 1.0);
        assert!(material.clearcoat_texture().is_some());
        assert!(material.clearcoat_roughness_texture().is_some());
        assert!(material.clearcoat_normal_texture().is_some());
        assert!(material.sheen_color_texture().is_some());
        assert!(material.sheen_roughness_texture().is_some());
        assert!(material.anisotropy_texture().is_some());
        assert!(material.iridescence_texture().is_some());
        assert!(material.iridescence_thickness_texture().is_some());
        assert!(material.transmission_texture().is_some());
        assert!(material.thickness_texture().is_some());
    }

    #[test]
    fn authored_advanced_pbr_recipe_factors_change_headless_gpu_pixels() {
        let baseline = recipe_material(json!({
            "id": "baseline",
            "kind": "pbr_metallic_roughness",
            "base_color": "base",
            "metallic": 0.0,
            "roughness": 0.42
        }));
        let advanced = recipe_material(json!({
            "id": "advanced",
            "kind": "pbr_metallic_roughness",
            "base_color": "base",
            "metallic": 0.0,
            "roughness": 0.42,
            "clearcoat_factor": 0.9,
            "clearcoat_roughness_factor": 0.12,
            "sheen_color_factor": "white",
            "sheen_roughness_factor": 0.25,
            "anisotropy_strength_factor": 0.8,
            "anisotropy_rotation_radians": 0.5,
            "iridescence_factor": 0.7,
            "iridescence_ior": 1.45,
            "iridescence_thickness_minimum_nm": 110.0,
            "iridescence_thickness_maximum_nm": 520.0,
            "dispersion_factor": 0.04,
            "transmission_factor": 0.25,
            "ior": 1.55,
            "thickness_factor": 0.08,
            "attenuation_distance": 2.0,
            "attenuation_color": "blue"
        }));

        let baseline_frame = render_material_gpu(baseline);
        let advanced_frame = render_material_gpu(advanced);
        assert_ne!(
            baseline_frame, advanced_frame,
            "advanced PBR recipe factors must affect HeadlessGpu pixels"
        );
    }

    fn recipe_material(value: serde_json::Value) -> MaterialDesc {
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
        .expect("recipe material builds")
        .1
    }

    fn render_material_gpu(material: MaterialDesc) -> Vec<u8> {
        let assets = Assets::new();
        let geometry = assets.create_geometry(GeometryDesc::sphere(0.45, 32, 16));
        let material = assets.create_material(material);
        let mut scene = Scene::new();
        scene
            .mesh(geometry, material)
            .transform(Transform::at(Vec3::ZERO))
            .add()
            .expect("mesh inserts");
        scene
            .directional_light(DirectionalLight::key_light().with_illuminance_lux(12_000.0))
            .add()
            .expect("light inserts");
        let camera = scene.add_default_camera().expect("camera inserts");
        let mut renderer = Renderer::headless_gpu(64, 64).expect("HeadlessGpu renderer builds");
        let environment =
            pollster::block_on(assets.load_environment_preset(EnvironmentPreset::NeutralStudio))
                .expect("neutral studio environment loads");
        renderer.set_environment(environment);
        renderer
            .prepare_with_assets(&mut scene, &assets)
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
