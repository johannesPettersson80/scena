use serde::{Deserialize, Serialize};

use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{
    AntiAliasing, AssetFetcher, AutoExposureConfig, Background, Color, DirectionalLight,
    EnvironmentPreset, GridFloorOptions, LookupError, MaterialDesc, PostBloomConfig,
    ScreenSpaceAmbientOcclusionConfig, Transform, Vec3,
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
    /// Adds Scena's stable three-directional-light studio rig and registers its
    /// nodes for host-side reporting and cleanup.
    pub fn add_studio_lighting(&mut self) -> Result<Vec<u64>, SceneHostError> {
        let lights = self.scene.add_studio_lighting()?;
        Ok([lights.key, lights.fill, lights.rim]
            .into_iter()
            .map(|node| self.register_node(node))
            .collect())
    }

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

    /// Applies the lightweight shaded-with-edges presentation used by interactive CAD viewers.
    ///
    /// This changes only renderer-owned presentation state and the host scene's material
    /// bindings. It does not mutate imported geometry or source assets.
    pub fn apply_cad_viewport_visuals(
        &mut self,
        roots: &[u64],
        background: &str,
    ) -> Result<Vec<u64>, SceneHostError> {
        let roots = roots
            .iter()
            .map(|handle| self.resolve_node(*handle))
            .collect::<Result<Vec<_>, _>>()?;
        let background = scene_host_background(background)?;

        self.renderer.set_background(background);
        self.renderer
            .set_anti_aliasing(SceneSetupPreset::CadStudio.anti_aliasing());

        if self.scene.light_nodes().next().is_none() {
            let key = self
                .scene
                .directional_light(DirectionalLight::default().with_illuminance_lux(18_000.0))
                .transform(Transform::default().rotate_x_deg(-35.0).rotate_y_deg(25.0))
                .add()?;
            let fill = self
                .scene
                .directional_light(
                    DirectionalLight::default()
                        .with_color(Color::COOL_WHITE)
                        .with_illuminance_lux(8_000.0),
                )
                .transform(Transform::default().rotate_x_deg(20.0).rotate_y_deg(-130.0))
                .add()?;
            self.register_node(key);
            self.register_node(fill);
        }

        let surface = self
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(
                Color::from_srgb_u8(196, 206, 211),
                0.0,
                0.82,
            ));
        let edges = self.assets.create_material(
            MaterialDesc::edge(Color::from_srgb_u8(53, 65, 71), 1.25)
                .with_edge_angle_threshold_degrees(18.0),
        );

        let mut overlay_handles = Vec::new();
        for root in roots {
            self.scene.set_subtree_mesh_material(root, surface)?;
            for overlay in self.scene.add_subtree_edge_overlays(root, edges)? {
                self.scene.set_helper_on_top(overlay, false)?;
                overlay_handles.push(self.register_node(overlay));
            }
        }
        Ok(overlay_handles)
    }

    /// Updates only the renderer-owned background of an interactive CAD viewport.
    ///
    /// Hosts use this when a UI theme changes after the scene has already been
    /// styled. It deliberately leaves geometry, materials, lights, camera state,
    /// anti-aliasing, and edge overlays untouched.
    pub fn set_cad_viewport_background(&mut self, background: &str) -> Result<(), SceneHostError> {
        self.renderer
            .set_background(scene_host_background(background)?);
        Ok(())
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
mod cad_viewport_tests {
    use super::*;
    use crate::{
        AntiAliasing, Color, GeometryDesc, MaterialDesc, MaterialKind, NodeKind, Transform,
    };

    #[test]
    fn cad_viewport_visuals_add_lightweight_shading_and_feature_edges() {
        let mut host = SceneHostCore::headless(96, 64).expect("headless host builds");
        let root = host
            .scene
            .add_empty(host.scene.root(), Transform::IDENTITY)
            .expect("CAD import root inserts");
        let geometry = host
            .assets
            .create_geometry(GeometryDesc::box_xyz(2.0, 1.0, 0.5));
        let source_material = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(Color::RED, 0.7, 0.1));
        let mesh = host
            .scene
            .mesh(geometry, source_material)
            .parent(root)
            .add()
            .expect("CAD mesh inserts");
        let root_handle = host.register_node(root);

        let overlays = host
            .apply_cad_viewport_visuals(&[root_handle], "studio")
            .expect("CAD viewport presentation applies");

        assert_eq!(host.renderer.background_color(), Color::STUDIO_BACKDROP);
        assert_eq!(host.renderer.anti_aliasing(), AntiAliasing::Fxaa);
        assert_eq!(
            host.scene.light_nodes().count(),
            2,
            "the interactive CAD path uses a cheap key/fill rig, not product-studio effects"
        );
        assert_eq!(overlays.len(), 1, "one source mesh gets one edge overlay");

        let NodeKind::Mesh(surface) = host.scene.node(mesh).expect("surface remains").kind() else {
            panic!("CAD surface remains a mesh");
        };
        let surface_material = host
            .assets
            .material(surface.material())
            .expect("CAD surface material exists");
        assert_eq!(surface_material.kind(), MaterialKind::PbrMetallicRoughness);
        assert_ne!(
            surface_material.base_color(),
            Color::RED,
            "CAD inspection uses a neutral presentation material without mutating the source asset"
        );

        let overlay = host
            .resolve_node(overlays[0])
            .expect("edge overlay handle resolves");
        assert_eq!(host.scene.helper_on_top(overlay), Some(false));
        let NodeKind::Mesh(edge_mesh) = host.scene.node(overlay).expect("overlay remains").kind()
        else {
            panic!("CAD edge overlay is a mesh drawable");
        };
        let edge_material = host
            .assets
            .material(edge_mesh.material())
            .expect("edge material exists");
        assert_eq!(edge_material.kind(), MaterialKind::Edge);
        assert_eq!(edge_material.stroke_width_px(), Some(1.25));
        assert_eq!(edge_material.edge_angle_threshold_degrees(), Some(18.0));
    }

    #[test]
    fn cad_viewport_background_changes_only_renderer_background() {
        let mut host = SceneHostCore::headless(96, 64).expect("headless host builds");
        let material = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(Color::RED, 0.7, 0.1));
        let material_before = host.assets.material(material).expect("material exists");
        let light_count = host.scene.light_nodes().count();

        host.set_cad_viewport_background("dark_studio")
            .expect("CAD viewport background applies");

        assert_eq!(host.renderer.background_color(), Color::CHARCOAL);
        assert_eq!(host.assets.material(material), Some(material_before));
        assert_eq!(host.scene.light_nodes().count(), light_count);
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
        let small_lateral_span = small.subject_extent_m.x.max(small.subject_extent_m.z);
        assert!(
            small.lights[0].emitter_width_m >= small_lateral_span * 1.5
                && small.lights[0].emitter_height_m >= small.subject_extent_m.y * 1.25,
            "the key must be a subject-sized softbox that produces broad photographic \
             highlights, not a compact emitter: {small:#?}"
        );
        assert!(
            small.lights[1].emitter_width_m >= small_lateral_span * 1.5,
            "the fill must be broad enough to lift material detail without a point-like \
             highlight: {small:#?}"
        );
    }

    #[test]
    fn final_photographic_lighting_uses_full_resolution_environment() {
        let material = MaterialDesc::pbr_metallic_roughness(Color::LIGHT_GRAY, 1.0, 0.2);
        let (mut preview_host, preview_subject) =
            subject_host(Vec3::new(1.0, 0.6, 0.8), material.clone());
        let (mut final_host, final_subject) = subject_host(Vec3::new(1.0, 0.6, 0.8), material);

        let preview = preview_host
            .apply_photographic_lighting(preview_subject)
            .expect("preview lighting solves");
        let final_report = final_host
            .apply_final_photographic_lighting(final_subject)
            .expect("final lighting solves");

        assert_eq!(preview.environment.source_dimensions, Some([128, 64]));
        assert_eq!(preview.environment.cubemap_resolution, Some(64));
        assert_eq!(
            final_report.environment.source_dimensions,
            Some([2048, 1024])
        );
        assert_eq!(final_report.environment.cubemap_resolution, Some(512));
        assert_eq!(
            final_report.environment.name.as_deref(),
            Some("studio_small_08_2048x1024.hdr")
        );
        assert!(preview.environment.equirectangular_hdr);
        assert!(final_report.environment.equirectangular_hdr);
    }

    #[test]
    fn photographic_white_balance_includes_environment_irradiance_color() {
        let (mut host, subject) = subject_host(
            Vec3::ONE,
            MaterialDesc::pbr_metallic_roughness(Color::GRAY, 0.0, 0.8),
        );
        let warm_irradiance = [2.0, 1.0, 0.5];
        let environment = host.assets.create_environment(
            crate::EnvironmentDesc::from_equirectangular_radiance(
                "warm-control_2x1.hdr",
                2,
                1,
                vec![warm_irradiance; 2],
            )
            .expect("warm gray-ball environment is valid")
            .with_cubemap_resolution(1),
        );
        host.renderer.set_environment(environment);
        let report = host
            .apply_photographic_lighting_adjusted(
                subject,
                crate::scene_host::PhotographicLightingAdjustmentV1 {
                    key_scale: 0.0,
                    fill_scale: 0.0,
                    rim_scale: 0.0,
                    overhead_scale: 0.0,
                    ..Default::default()
                },
            )
            .expect("environment-only lighting solves");

        let balanced = std::array::from_fn::<_, 3, _>(|channel| {
            warm_irradiance[channel] * report.white_balance.linear_multipliers[channel]
        });
        let srgb = balanced.map(linear_channel_to_srgb_u8);
        let minimum = *srgb.iter().min().expect("three gray-ball channels");
        let maximum = *srgb.iter().max().expect("three gray-ball channels");
        assert!(
            maximum - minimum <= 3,
            "automatic white balance must neutralize environment-lit gray within three sRGB \
             levels, got linear={balanced:?} srgb={srgb:?}: {report:#?}"
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
    fn photographic_lighting_solver_weights_dark_fraction_by_surface_coverage() {
        let mut host = SceneHostCore::headless(96, 64).expect("headless host builds");
        let root = host
            .scene
            .add_empty(host.scene.root(), Transform::IDENTITY)
            .expect("assembly root inserts");
        let dark = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(
                Color::from_srgb_u8(8, 10, 12),
                0.0,
                0.9,
            ));
        let light = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.9));
        let large = host.assets.create_geometry(GeometryDesc::plane(4.0, 4.0));
        let small = host.assets.create_geometry(GeometryDesc::plane(0.1, 0.1));
        host.scene
            .mesh(large, dark)
            .parent(root)
            .add()
            .expect("large dark surface inserts");
        host.scene
            .mesh(small, light)
            .parent(root)
            .transform(Transform::at(Vec3::new(0.0, 0.01, 0.0)))
            .add()
            .expect("small light surface inserts");
        host.renderer.set_background_color(Color::WHITE);
        let subject = host.register_node(root);

        let report = host
            .apply_photographic_lighting(subject)
            .expect("coverage-aware lighting solves");

        assert!(
            report.material.dark_fraction > 0.9,
            "dark fraction must follow surface coverage rather than material count: {report:#?}"
        );
        assert!(
            report.lights[2].luminous_flux_lumens > 0.0,
            "a predominantly dark surface needs minimum rim contribution even when its \
             average material/background separation is high: {report:#?}"
        );
    }

    #[test]
    fn photographic_lighting_solver_keeps_a_small_dark_surface_readable() {
        let mut host = SceneHostCore::headless(96, 64).expect("headless host builds");
        let root = host
            .scene
            .add_empty(host.scene.root(), Transform::IDENTITY)
            .expect("assembly root inserts");
        let light = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(
                Color::LIGHT_GRAY,
                0.8,
                0.24,
            ));
        let dark = host
            .assets
            .create_material(MaterialDesc::pbr_metallic_roughness(
                Color::from_srgb_u8(8, 10, 12),
                0.7,
                0.32,
            ));
        let large = host.assets.create_geometry(GeometryDesc::plane(4.0, 4.0));
        let small = host.assets.create_geometry(GeometryDesc::plane(1.3, 1.3));
        host.scene
            .mesh(large, light)
            .parent(root)
            .add()
            .expect("large light surface inserts");
        host.scene
            .mesh(small, dark)
            .parent(root)
            .transform(Transform::at(Vec3::new(0.0, 0.01, 0.0)))
            .add()
            .expect("small dark surface inserts");
        host.renderer.set_background_color(Color::WHITE);
        let subject = host.register_node(root);

        let report = host
            .apply_photographic_lighting(subject)
            .expect("mixed-coverage lighting solves");
        let key_flux = report.lights[0].luminous_flux_lumens;
        let fill_ratio = report.lights[1].luminous_flux_lumens / key_flux;
        let rim_ratio = report.lights[2].luminous_flux_lumens / key_flux;

        assert!(
            report.material.dark_fraction > 0.08 && report.material.dark_fraction < 0.15,
            "control must represent a small but meaningful dark-surface share: {report:#?}"
        );
        assert!(
            fill_ratio >= 0.34,
            "small dark-surface coverage needs enough broad fill for face readability, got \
             {fill_ratio}: {report:#?}"
        );
        assert!(
            rim_ratio >= 0.05,
            "small dark-surface coverage needs a bounded minimum rim even when average \
             background separation is high, got {rim_ratio}: {report:#?}"
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

    fn linear_channel_to_srgb_u8(value: f32) -> u8 {
        let encoded = if value <= 0.003_130_8 {
            value * 12.92
        } else {
            1.055 * value.powf(1.0 / 2.4) - 0.055
        };
        (encoded.clamp(0.0, 1.0) * 255.0).round() as u8
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
        assert_eq!(
            report.generated_nodes.len(),
            2,
            "the generated cyclorama owns its floor in one mesh, plus one contact-shadow node"
        );
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
    fn photographic_surface_repairs_geometry_and_adds_generated_physical_detail() {
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
        assert_eq!(improved.photographic_micro_surface(), None);
        assert!(improved.base_color_texture().is_some());
        assert!(improved.normal_texture().is_some());
        let packed = host
            .assets
            .sample_texture(
                improved
                    .metallic_roughness_texture()
                    .expect("generated packed material map"),
                [0.31, 0.73],
            )
            .expect("generated map samples");
        assert!((packed.b - 0.7).abs() <= 0.01);
        assert!((packed.g - 0.24).abs() <= 0.09);
        assert!(improved.occlusion_texture().is_some());
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
