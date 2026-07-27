use serde::{Deserialize, Serialize};

use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{
    AntiAliasing, AssetFetcher, AutoExposureConfig, Background, EnvironmentPreset,
    GridFloorOptions, LookupError, PostBloomConfig, ScreenSpaceAmbientOcclusionConfig, Vec3,
};

pub const SCENE_HOST_GROUNDING_SCHEMA_V1: &str = "scena.scene_host_grounding.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneSetupPreset {
    ProductStudio,
    CadStudio,
    IndustrialStudio,
}

impl SceneSetupPreset {
    pub const ALL: &'static [Self] =
        &[Self::ProductStudio, Self::CadStudio, Self::IndustrialStudio];

    pub const fn recipe_name(self) -> &'static str {
        match self {
            Self::ProductStudio => "product_studio",
            Self::CadStudio => "cad_studio",
            Self::IndustrialStudio => "industrial_studio",
        }
    }

    pub fn from_recipe_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|preset| preset.recipe_name() == name)
    }

    pub const fn background(self) -> Background {
        match self {
            Self::ProductStudio => Background::Studio,
            Self::CadStudio => Background::NeutralGray,
            Self::IndustrialStudio => Background::DarkStudio,
        }
    }

    pub const fn environment(self) -> EnvironmentPreset {
        match self {
            Self::ProductStudio | Self::IndustrialStudio => EnvironmentPreset::Studio,
            Self::CadStudio => EnvironmentPreset::NeutralStudio,
        }
    }

    pub const fn auto_exposure(self) -> AutoExposureConfig {
        match self {
            Self::ProductStudio => AutoExposureConfig::product_studio(),
            Self::CadStudio => AutoExposureConfig::mixed(),
            Self::IndustrialStudio => AutoExposureConfig::indoor(),
        }
    }

    pub fn grid_options(self) -> GridFloorOptions {
        match self {
            Self::ProductStudio => GridFloorOptions::new()
                .padding(0.18)
                .line_spacing(0.08)
                .line_width_px(4.0)
                .color(crate::Color::from_srgb_u8(58, 62, 70))
                .line_color(crate::Color::from_srgb_u8(83, 91, 104))
                .roughness(0.88),
            Self::CadStudio => GridFloorOptions::new()
                .padding(0.10)
                .line_spacing(0.05)
                .line_width_px(3.8)
                .color(crate::Color::from_srgb_u8(214, 218, 224))
                .line_color(crate::Color::from_srgb_u8(150, 158, 170))
                .roughness(0.92),
            Self::IndustrialStudio => GridFloorOptions::new()
                .padding(0.22)
                .line_spacing(0.12)
                .line_width_px(4.0)
                .color(crate::Color::from_srgb_u8(39, 44, 54))
                .line_color(crate::Color::from_srgb_u8(73, 84, 101))
                .roughness(0.94),
        }
    }

    pub const fn grid_reflection_strength(self) -> Option<f64> {
        match self {
            Self::ProductStudio => Some(0.32),
            Self::IndustrialStudio => Some(0.18),
            Self::CadStudio => None,
        }
    }

    pub fn ssao(self) -> ScreenSpaceAmbientOcclusionConfig {
        match self {
            Self::ProductStudio => ScreenSpaceAmbientOcclusionConfig::new(4, 0.42, 0.025),
            Self::CadStudio => ScreenSpaceAmbientOcclusionConfig::new(3, 0.28, 0.02),
            Self::IndustrialStudio => ScreenSpaceAmbientOcclusionConfig::new(4, 0.36, 0.03),
        }
    }

    pub const fn anti_aliasing(self) -> AntiAliasing {
        match self {
            Self::ProductStudio | Self::CadStudio | Self::IndustrialStudio => AntiAliasing::Fxaa,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneHostGroundingReportV1 {
    pub schema: String,
    pub target: u64,
    pub floor_handles: Vec<u64>,
    pub floor_receiver: bool,
    pub ssao_enabled: bool,
    pub active_paths: Vec<SceneHostGroundingPathV1>,
    pub fallbacks: Vec<SceneHostGroundingFallbackV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneHostGroundingPathV1 {
    FloorReceiver,
    ScreenSpaceAmbientOcclusion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneHostGroundingFallbackV1 {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub help: String,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn apply_scene_setup_preset_renderer(&mut self, preset: SceneSetupPreset) {
        self.renderer.set_background(preset.background());
        if self.renderer.auto_exposure().is_none() && !self.renderer.has_explicit_exposure_ev() {
            self.renderer.set_auto_exposure(preset.auto_exposure());
        }
        if self.renderer.screen_space_ambient_occlusion().is_none() {
            self.renderer
                .set_screen_space_ambient_occlusion(Some(preset.ssao()));
        }
    }

    pub fn apply_product_studio_visuals(&mut self, background: &str) -> Result<(), SceneHostError> {
        self.apply_product_studio_visuals_with_lighting(background, true)
    }

    pub fn apply_product_studio_visuals_with_lighting(
        &mut self,
        background: &str,
        add_lighting: bool,
    ) -> Result<(), SceneHostError> {
        let background = scene_host_background(background)?;
        if self.renderer.environment().is_none() {
            let environment = self.assets.default_environment();
            self.renderer.set_environment(environment);
        }
        self.apply_scene_setup_preset_renderer(SceneSetupPreset::ProductStudio);
        self.renderer.set_background(background);
        self.renderer
            .set_anti_aliasing(SceneSetupPreset::ProductStudio.anti_aliasing());
        self.renderer.set_bloom(Some(PostBloomConfig::subtle()));
        if add_lighting {
            let lights = self.scene.add_studio_lighting()?;
            self.register_node(lights.key);
            self.register_node(lights.fill);
            self.register_node(lights.rim);
        }
        Ok(())
    }

    pub fn apply_product_grounding_preset(
        &mut self,
        target: u64,
        background: &str,
    ) -> Result<SceneHostGroundingReportV1, SceneHostError> {
        self.ground_node_to_y_zero(target)?;
        self.apply_product_studio_visuals(background)?;
        let floor_handles = self.add_product_grid_floor_under_node(target)?;
        let floor_receiver = !floor_handles.is_empty();
        let ssao_enabled = self.renderer.screen_space_ambient_occlusion().is_some();
        let mut active_paths = Vec::new();
        if floor_receiver {
            active_paths.push(SceneHostGroundingPathV1::FloorReceiver);
        }
        if ssao_enabled {
            active_paths.push(SceneHostGroundingPathV1::ScreenSpaceAmbientOcclusion);
        }
        Ok(SceneHostGroundingReportV1 {
            schema: SCENE_HOST_GROUNDING_SCHEMA_V1.to_owned(),
            target,
            floor_handles,
            floor_receiver,
            ssao_enabled,
            active_paths,
            fallbacks: grounding_fallbacks(ssao_enabled),
        })
    }

    pub fn apply_product_grounding_preset_json(
        &mut self,
        target: u64,
        background: &str,
    ) -> Result<String, SceneHostError> {
        let report = self.apply_product_grounding_preset(target, background)?;
        serde_json::to_string(&report).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("grounding report serialization failed: {error}"),
            )
        })
    }

    pub fn add_product_grid_floor_under_node(
        &mut self,
        node: u64,
    ) -> Result<Vec<u64>, SceneHostError> {
        let node = self.resolve_node(node)?;
        let bounds = self
            .scene
            .node_world_bounds(node, &self.assets)?
            .ok_or(LookupError::ImportHasNoBounds)?;
        let floor = self.scene.add_grid_floor(
            &self.assets,
            GridFloorOptions::new()
                .under_bounds(bounds)
                .padding(0.24)
                .line_spacing(0.08),
        )?;
        Ok(vec![
            self.register_node(floor.slab),
            self.register_node(floor.grid),
        ])
    }

    fn ground_node_to_y_zero(&mut self, node: u64) -> Result<(), SceneHostError> {
        let node_key = self.resolve_node(node)?;
        let bounds = self
            .scene
            .node_world_bounds(node_key, &self.assets)?
            .ok_or(LookupError::ImportHasNoBounds)?;
        let drop_y = -bounds.min.y;
        if drop_y.abs() <= 1.0e-6 {
            return Ok(());
        }
        let mut world = self
            .scene
            .world_transform(node_key)
            .ok_or(LookupError::NodeNotFound(node_key))?;
        world.translation += Vec3::new(0.0, drop_y, 0.0);
        self.scene.align_to(node_key, world)?;
        Ok(())
    }
}

fn grounding_fallbacks(ssao_enabled: bool) -> Vec<SceneHostGroundingFallbackV1> {
    let mut fallbacks = Vec::new();
    if ssao_enabled {
        fallbacks.push(SceneHostGroundingFallbackV1 {
            code: "ssao_is_ambient_occlusion".to_owned(),
            severity: "info".to_owned(),
            message: "SSAO darkens depth contact edges but is not a drop-shadow substitute"
                .to_owned(),
            help: "use the report active_paths to distinguish floor receiver, SSAO, and shadow receiver claims"
                .to_owned(),
        });
    } else {
        fallbacks.push(SceneHostGroundingFallbackV1 {
            code: "ssao_unavailable".to_owned(),
            severity: "warning".to_owned(),
            message: "screen-space ambient occlusion is not active for this grounding preset"
                .to_owned(),
            help:
                "check backend capabilities or enable SSAO before relying on contact-edge darkening"
                    .to_owned(),
        });
    }
    fallbacks
}

fn scene_host_background(background: &str) -> Result<Background, SceneHostError> {
    match background {
        "studio_neutral" | "dark_studio" => Ok(Background::DarkStudio),
        "studio" => Ok(Background::Studio),
        "neutral_gray" => Ok(Background::NeutralGray),
        "black" => Ok(Background::Black),
        "white" => Ok(Background::White),
        other => Err(SceneHostError::new(
            SceneHostErrorCode::InvalidInput,
            format!("unsupported SceneHost product studio background {other}"),
        )),
    }
}

#[cfg(test)]
mod photographic_lighting_tests {
    use super::*;
    use crate::{Color, GeometryDesc, MaterialDesc, Transform};

    fn subject_host(size: Vec3, material: MaterialDesc) -> (SceneHostCore, u64) {
        let mut host = SceneHostCore::headless(96, 64).expect("headless host builds");
        let geometry = host
            .assets
            .create_geometry(GeometryDesc::box_xyz(size.x, size.y, size.z));
        let material = host.assets.create_material(material);
        let node = host
            .scene
            .mesh(geometry, material)
            .add()
            .expect("subject mesh inserts");
        let handle = host.register_node(node);
        (host, handle)
    }

    #[test]
    fn photographic_lighting_solver_scales_area_emitters_from_subject_geometry() {
        let material = MaterialDesc::pbr_metallic_roughness(Color::LIGHT_GRAY, 1.0, 0.32);
        let (mut small_host, small_subject) =
            subject_host(Vec3::new(0.4, 0.2, 0.3), material.clone());
        let (mut large_host, large_subject) = subject_host(Vec3::new(4.0, 2.0, 3.0), material);

        let small = small_host
            .apply_photographic_lighting(small_subject)
            .expect("small-subject lighting solves");
        let large = large_host
            .apply_photographic_lighting(large_subject)
            .expect("large-subject lighting solves");

        assert_eq!(small.source, "subject_lighting_solver");
        assert_eq!(small.lights.len(), 4);
        assert_eq!(
            small
                .lights
                .iter()
                .map(|light| light.role.as_str())
                .collect::<Vec<_>>(),
            ["key", "fill", "rim", "overhead"]
        );
        assert!(
            large.lights[0].emitter_width_m > small.lights[0].emitter_width_m * 5.0,
            "emitter size must follow subject scale: small={small:#?} large={large:#?}"
        );
        assert!(
            large.lights[0].luminous_flux_lumens > small.lights[0].luminous_flux_lumens * 25.0,
            "inverse-square placement requires flux to scale with subject area: small={small:#?} large={large:#?}"
        );
        assert!(
            small
                .lights
                .iter()
                .all(|light| light.position.is_finite() && light.target.is_finite()),
            "the solver must emit finite continuous light parameters: {small:#?}"
        );
    }

    #[test]
    fn photographic_lighting_solver_uses_material_response_instead_of_fixed_rig() {
        let (mut dark_host, dark_subject) = subject_host(
            Vec3::ONE,
            MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(10, 12, 14), 0.0, 0.9),
        );
        let (mut metal_host, metal_subject) = subject_host(
            Vec3::ONE,
            MaterialDesc::pbr_metallic_roughness(Color::LIGHT_GRAY, 1.0, 0.18),
        );

        let dark = dark_host
            .apply_photographic_lighting(dark_subject)
            .expect("dark-material lighting solves");
        let metal = metal_host
            .apply_photographic_lighting(metal_subject)
            .expect("metal lighting solves");

        assert!(dark.material.dark_fraction > metal.material.dark_fraction);
        assert!(metal.material.reflective_fraction > dark.material.reflective_fraction);
        assert!(
            dark.lights[1].luminous_flux_lumens / dark.lights[0].luminous_flux_lumens
                > metal.lights[1].luminous_flux_lumens / metal.lights[0].luminous_flux_lumens,
            "dark matte subjects need a stronger fill ratio"
        );
        assert_ne!(
            metal.environment.rotation_y_degrees, dark.environment.rotation_y_degrees,
            "reflective subjects need a differently oriented environment highlight"
        );
    }

    #[test]
    fn photographic_lighting_solver_places_emitters_relative_to_the_active_camera() {
        let material = MaterialDesc::pbr_metallic_roughness(Color::LIGHT_GRAY, 1.0, 0.32);
        let (mut front_host, front_subject) = subject_host(Vec3::ONE, material.clone());
        let (mut back_host, back_subject) = subject_host(Vec3::ONE, material);
        let front_camera = front_host
            .scene
            .camera_node(front_host.active_camera)
            .expect("front camera node exists");
        let back_camera = back_host
            .scene
            .camera_node(back_host.active_camera)
            .expect("back camera node exists");
        front_host
            .scene
            .set_transform(
                front_camera,
                Transform::at(Vec3::new(0.0, 0.0, 5.0)).looking_at(Vec3::ZERO, Vec3::Y),
            )
            .expect("front camera moves");
        back_host
            .scene
            .set_transform(
                back_camera,
                Transform::at(Vec3::new(0.0, 0.0, -5.0)).looking_at(Vec3::ZERO, Vec3::Y),
            )
            .expect("back camera moves");

        let front = front_host
            .apply_photographic_lighting(front_subject)
            .expect("front lighting solves");
        let back = back_host
            .apply_photographic_lighting(back_subject)
            .expect("back lighting solves");

        assert!(
            front.lights[0].position.z > 0.0 && back.lights[0].position.z < 0.0,
            "key light must remain camera-facing when the camera crosses the subject: front={front:#?} back={back:#?}"
        );
        assert!(
            front.lights[2].position.z < 0.0 && back.lights[2].position.z > 0.0,
            "rim light must remain behind the subject relative to the camera: front={front:#?} back={back:#?}"
        );
    }

    #[test]
    fn photographic_lighting_solver_profiles_subject_surface_orientations() {
        let mut host = SceneHostCore::headless(96, 64).expect("headless host builds");
        let geometry = host.assets.create_geometry(GeometryDesc::plane(2.0, 1.0));
        let material = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(
                Color::LIGHT_GRAY,
                0.0,
                0.72,
            ));
        let node = host
            .scene
            .mesh(geometry, material)
            .add()
            .expect("plane inserts");
        let subject = host.register_node(node);

        let report = host
            .apply_photographic_lighting(subject)
            .expect("plane lighting solves");

        assert!(report.geometry.vertex_normal_samples >= 4);
        assert!(
            report.geometry.normal_axis_weights[1] > 0.95,
            "an XZ plane should be classified as upward-facing geometry: {report:#?}"
        );
        assert!(
            report.geometry.normal_axis_weights[0] < 0.05
                && report.geometry.normal_axis_weights[2] < 0.05,
            "plane profile must not invent side-facing surface weight: {report:#?}"
        );
    }

    #[test]
    fn photographic_lighting_solver_profiles_and_preserves_authored_environment() {
        let (mut host, subject) = subject_host(
            Vec3::ONE,
            MaterialDesc::pbr_metallic_roughness(Color::LIGHT_GRAY, 1.0, 0.24),
        );
        let authored_environment =
            host.assets
                .create_environment(crate::EnvironmentDesc::from_equirectangular_hdr_path(
                    "authored-workshop.hdr",
                ));
        host.renderer.set_environment(authored_environment);

        host.apply_product_studio_visuals_with_lighting("dark_studio", false)
            .expect("photographic setup applies");
        let report = host
            .apply_photographic_lighting(subject)
            .expect("environment-aware lighting solves");

        assert_eq!(
            host.renderer.environment(),
            Some(authored_environment),
            "automatic photographic setup must preserve an authored environment"
        );
        assert!(report.environment.present);
        assert_eq!(
            report.environment.name.as_deref(),
            Some("authored-workshop.hdr")
        );
        assert!(report.environment.equirectangular_hdr);
    }

    #[test]
    fn photographic_lighting_solver_controls_environment_from_subject_response() {
        let (mut matte_host, matte_subject) = subject_host(
            Vec3::ONE,
            MaterialDesc::pbr_metallic_roughness(Color::LIGHT_GRAY, 0.0, 0.92),
        );
        let (mut metal_host, metal_subject) = subject_host(
            Vec3::ONE,
            MaterialDesc::pbr_metallic_roughness(Color::LIGHT_GRAY, 1.0, 0.12),
        );

        let matte = matte_host
            .apply_photographic_lighting(matte_subject)
            .expect("matte lighting solves");
        let metal = metal_host
            .apply_photographic_lighting(metal_subject)
            .expect("metal lighting solves");

        assert!(matte.environment.present && metal.environment.present);
        assert_ne!(
            matte.environment.rotation_y_degrees, metal.environment.rotation_y_degrees,
            "reflective response must alter environment orientation"
        );
        assert_eq!(
            metal_host.renderer.environment_rotation_y_degrees(),
            metal.environment.rotation_y_degrees
        );
        assert_eq!(
            metal_host.renderer.environment_intensity(),
            metal.environment.intensity
        );
        assert_eq!(
            metal_host.renderer.white_balance().linear_multipliers(),
            metal.white_balance.linear_multipliers,
            "the lighting estimate must drive a scene-linear camera white balance"
        );
    }

    #[test]
    fn photographic_lighting_solver_uses_fill_without_unconditional_rim() {
        let (mut separated_host, separated_subject) = subject_host(
            Vec3::ONE,
            MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.82),
        );
        separated_host.renderer.set_background_color(Color::BLACK);
        let (mut merging_host, merging_subject) = subject_host(
            Vec3::ONE,
            MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(10, 12, 14), 0.0, 0.88),
        );
        merging_host
            .renderer
            .set_background_color(Color::from_srgb_u8(8, 10, 12));

        let separated = separated_host
            .apply_photographic_lighting(separated_subject)
            .expect("separated subject lighting solves");
        let merging = merging_host
            .apply_photographic_lighting(merging_subject)
            .expect("merging subject lighting solves");
        let separated_key = separated.lights[0].luminous_flux_lumens;
        let separated_fill = separated.lights[1].luminous_flux_lumens;

        assert!(
            separated_fill > separated_key * 0.20 && separated_fill < separated_key * 0.70,
            "fill must retain detail without flattening the key: {separated:#?}"
        );
        assert_eq!(
            separated.lights[2].luminous_flux_lumens, 0.0,
            "a subject already separated from its background must not receive rim light"
        );
        assert!(
            merging.lights[2].luminous_flux_lumens > 0.0,
            "a low-contrast subject/background pair needs rim separation: {merging:#?}"
        );
    }

    #[test]
    fn photographic_lighting_solver_applies_measured_continuous_adjustments() {
        let (mut host, subject) = subject_host(
            Vec3::ONE,
            MaterialDesc::pbr_metallic_roughness(Color::LIGHT_GRAY, 1.0, 0.24),
        );
        let baseline = host
            .apply_photographic_lighting(subject)
            .expect("baseline lighting solves");
        for light in &baseline.lights {
            host.remove_node(light.node)
                .expect("baseline generated light removes");
        }
        let adjustment = crate::scene_host::PhotographicLightingAdjustmentV1 {
            key_scale: 0.8,
            fill_scale: 0.55,
            rim_scale: 1.3,
            overhead_scale: 0.7,
            environment_intensity_scale: 0.9,
            environment_rotation_offset_degrees: 37.0,
        };
        let adjusted = host
            .apply_photographic_lighting_adjusted(subject, adjustment)
            .expect("adjusted lighting solves");

        assert!(
            (adjusted.lights[0].luminous_flux_lumens
                - baseline.lights[0].luminous_flux_lumens * adjustment.key_scale)
                .abs()
                <= 1.0e-4
        );
        assert!(
            (adjusted.lights[1].luminous_flux_lumens
                - baseline.lights[1].luminous_flux_lumens * adjustment.fill_scale)
                .abs()
                <= 1.0e-4
        );
        assert!(
            (adjusted.environment.rotation_y_degrees
                - (baseline.environment.rotation_y_degrees
                    + adjustment.environment_rotation_offset_degrees)
                    .rem_euclid(360.0))
            .abs()
                <= 1.0e-4
        );
    }

    #[test]
    fn photographic_lighting_solver_handles_authored_multi_material_assembly() {
        let mut host = SceneHostCore::headless(96, 64).expect("headless host builds");
        let root = host
            .scene
            .add_empty(host.scene.root(), Transform::IDENTITY)
            .expect("assembly root inserts");
        let geometry = host
            .assets
            .create_geometry(GeometryDesc::box_xyz(0.8, 0.5, 0.4));
        let matte = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(
                Color::from_srgb_u8(18, 20, 24),
                0.0,
                0.9,
            ));
        let metal = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(
                Color::LIGHT_GRAY,
                1.0,
                0.16,
            ));
        host.scene
            .mesh(geometry, matte)
            .parent(root)
            .transform(Transform::at(Vec3::new(-0.5, 0.0, 0.0)))
            .add()
            .expect("matte part inserts");
        host.scene
            .mesh(geometry, metal)
            .parent(root)
            .transform(Transform::at(Vec3::new(0.5, 0.0, 0.0)))
            .add()
            .expect("metal part inserts");
        let subject = host.register_node(root);

        let report = host
            .apply_photographic_lighting(subject)
            .expect("assembly lighting solves");

        assert_eq!(report.material.material_count, 2);
        assert!(report.material.dark_fraction > 0.0);
        assert!(report.material.reflective_fraction > 0.0);
        assert!(report.geometry.vertex_normal_samples >= 48);
        assert!(report.subject_extent_m.x > 1.7);
    }
}

#[cfg(test)]
mod photographic_surroundings_tests {
    use super::*;
    use crate::{Color, GeometryDesc, MaterialDesc, Transform};

    fn subject_host(transform: Transform) -> (SceneHostCore, u64) {
        let mut host = SceneHostCore::headless(128, 96).expect("headless host builds");
        let geometry = host
            .assets
            .create_geometry(GeometryDesc::box_xyz(2.0, 1.0, 1.2));
        let material = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(
                Color::from_srgb_u8(88, 104, 122),
                0.72,
                0.28,
            ));
        let node = host
            .scene
            .mesh(geometry, material)
            .transform(transform)
            .add()
            .expect("subject mesh inserts");
        let handle = host.register_node(node);
        (host, handle)
    }

    #[test]
    fn photographic_surroundings_are_geometry_derived_and_transient() {
        let (mut host, subject) = subject_host(Transform::IDENTITY);
        let authored_nodes = host.scene.node_transforms().count();

        let report = host
            .apply_photographic_surroundings(subject)
            .expect("surroundings solve");

        assert_eq!(report.source, "subject_surroundings_solver");
        assert!(report.generated_floor);
        assert!(report.generated_cyclorama);
        assert!(report.support_height_m.is_some());
        assert!(report.extent_m > 2.0);
        assert!(
            report.background_luminance > 0.02 && report.background_luminance < 0.8,
            "automatic background must avoid pure black and white: {report:#?}"
        );
        assert_eq!(report.generated_nodes.len(), 3);
        assert_eq!(report.contact_shadow_nodes.len(), 1);
        assert!(report.grid_nodes.is_empty());
        assert!(host.scene.node_transforms().count() > authored_nodes);

        host.remove_photographic_surroundings(&report)
            .expect("transient surroundings remove");
        assert_eq!(
            host.scene.node_transforms().count(),
            authored_nodes,
            "automatic surroundings must not persist in the authored scene"
        );
    }

    #[test]
    fn photographic_surroundings_preserve_authored_support_and_environment() {
        let (mut host, subject) = subject_host(Transform::IDENTITY);
        let floor_geometry = host.assets.create_geometry(GeometryDesc::plane(8.0, 8.0));
        let floor_material = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(Color::GRAY, 0.0, 0.82));
        let floor = host
            .scene
            .mesh(floor_geometry, floor_material)
            .add()
            .expect("authored floor inserts");
        host.scene
            .add_tag(floor, "photographic_support")
            .expect("authored support tags");
        let environment = host.assets.default_environment();
        host.renderer.set_environment(environment);

        let report = host
            .apply_photographic_surroundings(subject)
            .expect("surroundings solve");

        assert!(report.preserved_authored_surroundings);
        assert!(report.preserved_authored_environment);
        assert!(!report.generated_floor);
        assert!(!report.generated_cyclorama);
        assert_eq!(report.generated_nodes, report.contact_shadow_nodes);
        assert_eq!(report.contact_shadow_nodes.len(), 1);
        assert!(host.scene.node(floor).is_some());
        assert_eq!(host.renderer.environment(), Some(environment));
    }

    #[test]
    fn photographic_surroundings_do_not_force_floor_under_suspended_subject() {
        let (mut host, subject) = subject_host(Transform::at(Vec3::new(0.0, 5.0, 0.0)));
        host.set_tag(subject, "photographic_suspended")
            .expect("subject tags");

        let report = host
            .apply_photographic_surroundings(subject)
            .expect("surroundings solve");

        assert_eq!(report.support_class, "suspended");
        assert!(report.support_height_m.is_none());
        assert!(!report.generated_floor);
        assert!(report.generated_cyclorama);
    }
}

#[cfg(test)]
mod photographic_surface_tests {
    use super::*;
    use crate::{
        Color, GeometryDesc, GeometryTopology, GeometryVertex, MaterialDesc, NodeKind, Vec3,
    };

    #[test]
    fn photographic_surface_repairs_geometry_and_adds_scale_aware_neutral_detail() {
        let mut host = SceneHostCore::headless(128, 96).expect("headless host builds");
        let geometry = host
            .assets
            .create_geometry(GeometryDesc::box_xyz(2.0, 1.0, 0.5));
        let material = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(
                Color::from_srgb_u8(110, 116, 124),
                0.7,
                0.24,
            ));
        let node = host
            .scene
            .mesh(geometry, material)
            .add()
            .expect("box inserts");
        let subject = host.register_node(node);

        let report = host
            .apply_photographic_surface(subject)
            .expect("surface improvement succeeds");

        assert_eq!(report.source, "photographic_surface_solver");
        assert_eq!(report.mesh_count, 1);
        assert_eq!(report.micro_beveled_meshes, 1);
        assert_eq!(report.micro_surface_materials, 1);
        assert!(report.max_bevel_m > 0.0 && report.max_bevel_m <= 0.01);
        assert!(report.substance_claims.is_empty());
        let NodeKind::Mesh(mesh) = host.scene.node(node).expect("node remains").kind() else {
            panic!("subject remains a mesh");
        };
        let improved = host
            .assets
            .material(mesh.material())
            .expect("improved material exists");
        let micro = improved
            .photographic_micro_surface()
            .expect("neutral micro surface is explicit");
        assert!(micro.strength() > 0.0);
        assert!(micro.scale_m() > 0.0);
    }

    #[test]
    fn photographic_surface_removes_degenerate_triangles_and_repairs_invalid_normals() {
        let mut host = SceneHostCore::headless(128, 96).expect("headless host builds");
        let vertices = vec![
            GeometryVertex {
                position: Vec3::new(-1.0, 0.0, 0.0),
                normal: Vec3::ZERO,
            },
            GeometryVertex {
                position: Vec3::new(1.0, 0.0, 0.0),
                normal: Vec3::ZERO,
            },
            GeometryVertex {
                position: Vec3::new(0.0, 1.0, 0.0),
                normal: Vec3::ZERO,
            },
        ];
        let geometry = host.assets.create_geometry(
            GeometryDesc::try_new(
                GeometryTopology::Triangles,
                vertices,
                vec![0, 1, 2, 0, 0, 0],
            )
            .expect("diagnostic geometry constructs"),
        );
        let material = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(Color::GRAY, 0.0, 0.7));
        let node = host
            .scene
            .mesh(geometry, material)
            .add()
            .expect("mesh inserts");
        let subject = host.register_node(node);

        let report = host
            .apply_photographic_surface(subject)
            .expect("safe geometry repair succeeds");

        assert_eq!(report.removed_degenerate_triangles, 1);
        assert_eq!(report.repaired_normal_meshes, 1);
        assert!(report.rejected_meshes.is_empty());
        let NodeKind::Mesh(mesh) = host.scene.node(node).expect("node remains").kind() else {
            panic!("subject remains a mesh");
        };
        let repaired = host
            .assets
            .geometry(mesh.geometry())
            .expect("repaired geometry exists");
        assert_eq!(repaired.indices().len(), 3);
        assert!(
            repaired
                .vertices()
                .iter()
                .all(|vertex| vertex.normal.is_finite() && vertex.normal.length_squared() > 0.99)
        );
    }

    #[test]
    fn photographic_surface_preserves_deliberately_sharp_or_textured_identity() {
        let mut host = SceneHostCore::headless(128, 96).expect("headless host builds");
        let geometry = host
            .assets
            .create_geometry(GeometryDesc::box_xyz(2.0, 1.0, 0.5));
        let material = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(Color::GRAY, 0.0, 0.5));
        let node = host
            .scene
            .mesh(geometry, material)
            .add()
            .expect("box inserts");
        host.scene
            .add_tag(node, "photographic_preserve_sharp_edges")
            .expect("preservation tag inserts");
        let subject = host.register_node(node);

        let report = host
            .apply_photographic_surface(subject)
            .expect("surface analysis succeeds");

        assert_eq!(report.micro_beveled_meshes, 0);
        assert_eq!(report.preserved_sharp_meshes, 1);
    }

    #[test]
    fn photographic_surface_reports_disconnected_faces_before_rendering() {
        let mut host = SceneHostCore::headless(128, 96).expect("headless host builds");
        let geometry = GeometryDesc::try_new(
            GeometryTopology::Triangles,
            vec![
                GeometryVertex {
                    position: Vec3::new(0.0, 0.0, 0.0),
                    normal: Vec3::Z,
                },
                GeometryVertex {
                    position: Vec3::new(1.0, 0.0, 0.0),
                    normal: Vec3::Z,
                },
                GeometryVertex {
                    position: Vec3::new(0.0, 1.0, 0.0),
                    normal: Vec3::Z,
                },
                GeometryVertex {
                    position: Vec3::new(3.0, 0.0, 0.0),
                    normal: Vec3::Z,
                },
                GeometryVertex {
                    position: Vec3::new(4.0, 0.0, 0.0),
                    normal: Vec3::Z,
                },
                GeometryVertex {
                    position: Vec3::new(3.0, 1.0, 0.0),
                    normal: Vec3::Z,
                },
            ],
            vec![0, 1, 2, 3, 4, 5],
        )
        .expect("disconnected geometry constructs");
        let geometry = host.assets.create_geometry(geometry);
        let material = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.5));
        let node = host
            .scene
            .mesh(geometry, material)
            .add()
            .expect("disconnected mesh inserts");
        let subject = host.register_node(node);

        let report = host
            .apply_photographic_surface(subject)
            .expect("surface analysis succeeds");

        assert_eq!(report.disconnected_meshes, 1);
        assert_eq!(report.maximum_disconnected_components, 2);
    }

    #[test]
    fn photographic_surface_reconstructs_valid_but_uninformative_normals() {
        let mut host = SceneHostCore::headless(128, 96).expect("headless host builds");
        let vertices = vec![
            GeometryVertex {
                position: Vec3::new(-1.0, -1.0, 0.0),
                normal: Vec3::X,
            },
            GeometryVertex {
                position: Vec3::new(1.0, -1.0, 0.0),
                normal: Vec3::X,
            },
            GeometryVertex {
                position: Vec3::new(1.0, 1.0, 0.0),
                normal: Vec3::X,
            },
            GeometryVertex {
                position: Vec3::new(-1.0, 1.0, 0.0),
                normal: Vec3::X,
            },
        ];
        let geometry = host.assets.create_geometry(
            GeometryDesc::try_new(
                GeometryTopology::Triangles,
                vertices,
                vec![0, 1, 2, 0, 2, 3],
            )
            .expect("smooth diagnostic geometry constructs"),
        );
        let material = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(Color::GRAY, 0.0, 0.5));
        let node = host
            .scene
            .mesh(geometry, material)
            .add()
            .expect("mesh inserts");
        let subject = host.register_node(node);

        let report = host
            .apply_photographic_surface(subject)
            .expect("weighted-normal repair succeeds");

        assert_eq!(report.repaired_normal_meshes, 1);
        let NodeKind::Mesh(mesh) = host.scene.node(node).expect("mesh remains").kind() else {
            panic!("subject remains a mesh");
        };
        let repaired = host
            .assets
            .geometry(mesh.geometry())
            .expect("repaired geometry exists");
        assert!(
            repaired
                .vertices()
                .iter()
                .all(|vertex| vertex.normal.dot(Vec3::Z) > 0.99)
        );
    }
}
