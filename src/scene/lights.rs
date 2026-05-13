use crate::diagnostics::LookupError;
use crate::material::Color;

use super::{Angle, LightKey, NodeKey, NodeKind, Scene, Transform};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Light {
    Directional(DirectionalLight),
    Point(PointLight),
    Spot(SpotLight),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionalLight {
    color: Color,
    illuminance_lux: f32,
    casts_shadows: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointLight {
    color: Color,
    intensity_candela: f32,
    range: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpotLight {
    color: Color,
    intensity_candela: f32,
    range: Option<f32>,
    inner_cone_angle: Angle,
    outer_cone_angle: Angle,
}

/// Builder returned by [`Scene::directional_light`], [`Scene::point_light`], and
/// [`Scene::spot_light`].
#[must_use = "light builders do nothing until add() is called"]
pub struct LightBuilder<'scene> {
    scene: &'scene mut Scene,
    parent: NodeKey,
    transform: Transform,
    light: Light,
}

impl Scene {
    pub fn light(&self, light: LightKey) -> Option<&Light> {
        self.lights.get(light)
    }

    pub fn directional_light(&mut self, light: DirectionalLight) -> LightBuilder<'_> {
        self.light_builder(Light::Directional(light))
    }

    /// Phase 5.3: insert a "studio" 3-point directional rig (key +
    /// cool fill + warm rim). Returns the three node keys in
    /// (key, fill, rim) order. Intensities are tuned to match the
    /// look of the Khronos sample-thumbnail renders without
    /// over-exposing PBR metallic body materials.
    ///
    /// Tests and examples previously hand-rolled this rig with
    /// 80,000-lux suns that drowned the asset's authored materials in
    /// specular reflections. This preset uses moderate intensities
    /// (key 12,000 lux + fill 4,000 lux + rim 3,000 lux) that read
    /// closer to what canonical PBR viewers produce.
    pub fn add_studio_lighting(&mut self) -> Result<StudioLightingHandles, LookupError> {
        let key = self
            .directional_light(
                DirectionalLight::default()
                    .with_color(Color::WHITE)
                    .with_illuminance_lux(12_000.0),
            )
            .transform(Transform::default().rotate_x_deg(-30.0).rotate_y_deg(20.0))
            .add()?;
        let fill = self
            .directional_light(
                DirectionalLight::default()
                    .with_color(Color::from_srgb_u8(200, 215, 235))
                    .with_illuminance_lux(4_000.0),
            )
            .transform(
                Transform::default()
                    .rotate_x_deg(-10.0)
                    .rotate_y_deg(-120.0),
            )
            .add()?;
        let rim = self
            .directional_light(
                DirectionalLight::default()
                    .with_color(Color::from_srgb_u8(255, 235, 210))
                    .with_illuminance_lux(3_000.0),
            )
            .transform(Transform::default().rotate_x_deg(15.0).rotate_y_deg(170.0))
            .add()?;
        Ok(StudioLightingHandles { key, fill, rim })
    }

    pub fn point_light(&mut self, light: PointLight) -> LightBuilder<'_> {
        self.light_builder(Light::Point(light))
    }

    pub fn spot_light(&mut self, light: SpotLight) -> LightBuilder<'_> {
        self.light_builder(Light::Spot(light))
    }

    fn light_builder(&mut self, light: Light) -> LightBuilder<'_> {
        let parent = self.root;
        LightBuilder {
            scene: self,
            parent,
            transform: Transform::default(),
            light,
        }
    }

    fn insert_light(
        &mut self,
        parent: NodeKey,
        light: Light,
        transform: Transform,
    ) -> Result<NodeKey, LookupError> {
        let light = self.lights.insert(light);
        match self.insert_node(parent, NodeKind::Light(light), transform) {
            Ok(node) => Ok(node),
            Err(error) => {
                self.lights.remove(light);
                Err(error)
            }
        }
    }
}

/// Phase 5.3: handles for the three lights inserted by
/// [`Scene::add_studio_lighting`]. Returned so callers can later
/// adjust an individual light (e.g. raise the key, tint the rim) or
/// remove the rig.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StudioLightingHandles {
    pub key: NodeKey,
    pub fill: NodeKey,
    pub rim: NodeKey,
}

impl LightBuilder<'_> {
    /// Overrides the parent node. The parent is validated when [`Self::add`] is called.
    pub fn parent(mut self, parent: NodeKey) -> Self {
        self.parent = parent;
        self
    }

    /// Overrides the local transform. Light direction and position are derived from this
    /// node transform during render preparation.
    pub fn transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    /// Inserts the light node and returns its typed scene node key.
    pub fn add(self) -> Result<NodeKey, LookupError> {
        self.scene
            .insert_light(self.parent, self.light, self.transform)
    }
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            illuminance_lux: 10_000.0,
            casts_shadows: false,
        }
    }
}

impl DirectionalLight {
    pub const fn color(self) -> Color {
        self.color
    }

    pub const fn illuminance_lux(self) -> f32 {
        self.illuminance_lux
    }

    pub const fn casts_shadows(self) -> bool {
        self.casts_shadows
    }

    pub const fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub const fn with_illuminance_lux(mut self, illuminance_lux: f32) -> Self {
        self.illuminance_lux = non_negative_or(illuminance_lux, 10_000.0);
        self
    }

    pub const fn with_shadows(mut self, enabled: bool) -> Self {
        self.casts_shadows = enabled;
        self
    }
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            intensity_candela: 100.0,
            range: None,
        }
    }
}

impl PointLight {
    pub const fn color(self) -> Color {
        self.color
    }

    pub const fn intensity_candela(self) -> f32 {
        self.intensity_candela
    }

    pub const fn range(self) -> Option<f32> {
        self.range
    }

    pub const fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub const fn with_intensity_candela(mut self, intensity_candela: f32) -> Self {
        self.intensity_candela = non_negative_or(intensity_candela, 100.0);
        self
    }

    pub const fn with_range(mut self, range: f32) -> Self {
        self.range = positive_range(range);
        self
    }
}

impl Default for SpotLight {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            intensity_candela: 100.0,
            range: None,
            inner_cone_angle: Angle::from_radians(0.0),
            outer_cone_angle: Angle::from_radians(std::f32::consts::FRAC_PI_4),
        }
    }
}

impl SpotLight {
    pub const fn color(self) -> Color {
        self.color
    }

    pub const fn intensity_candela(self) -> f32 {
        self.intensity_candela
    }

    pub const fn range(self) -> Option<f32> {
        self.range
    }

    pub const fn inner_cone_angle(self) -> Angle {
        self.inner_cone_angle
    }

    pub const fn outer_cone_angle(self) -> Angle {
        self.outer_cone_angle
    }

    pub const fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub const fn with_intensity_candela(mut self, intensity_candela: f32) -> Self {
        self.intensity_candela = non_negative_or(intensity_candela, 100.0);
        self
    }

    pub const fn with_range(mut self, range: f32) -> Self {
        self.range = positive_range(range);
        self
    }

    pub const fn with_inner_cone_angle(mut self, angle: Angle) -> Self {
        self.inner_cone_angle = clamp_angle(angle, 0.0, self.outer_cone_angle.radians());
        self
    }

    pub const fn with_outer_cone_angle(mut self, angle: Angle) -> Self {
        self.outer_cone_angle =
            clamp_angle(angle, self.inner_cone_angle.radians(), std::f32::consts::PI);
        self
    }
}

const fn non_negative_or(value: f32, fallback: f32) -> f32 {
    if value.is_nan() {
        fallback
    } else if value < 0.0 {
        0.0
    } else {
        value
    }
}

const fn positive_range(value: f32) -> Option<f32> {
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        None
    }
}

const fn clamp_angle(angle: Angle, min: f32, max: f32) -> Angle {
    let radians = angle.radians();
    if !radians.is_finite() || radians < min {
        Angle::from_radians(min)
    } else if radians > max {
        Angle::from_radians(max)
    } else {
        angle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_studio_lighting_inserts_three_directional_nodes_with_distinct_keys() {
        let mut scene = Scene::new();
        let handles = scene
            .add_studio_lighting()
            .expect("studio lighting inserts");
        assert_ne!(handles.key, handles.fill);
        assert_ne!(handles.fill, handles.rim);
        assert_ne!(handles.key, handles.rim);
        // Each handle resolves to a Light::Directional in the scene.
        for node in [handles.key, handles.fill, handles.rim] {
            let node_data = scene.node(node).expect("node exists");
            match node_data.kind {
                NodeKind::Light(light_key) => {
                    let light = scene.light(light_key).expect("light exists");
                    assert!(matches!(light, Light::Directional(_)));
                }
                _ => panic!("studio lighting handle must point at a Light node"),
            }
        }
    }

    #[test]
    fn add_studio_lighting_uses_moderate_intensities_not_overdriven_3point() {
        // Phase 5.3 motivation: the previous test rig used 80,000 lux
        // suns that overwhelmed PBR materials. The preset must stay at
        // moderate intensities so a metallic-1 surface doesn't render
        // as polished gold under a flood of specular highlights.
        let mut scene = Scene::new();
        let handles = scene.add_studio_lighting().expect("inserts");
        let mut illuminances = Vec::new();
        for node in [handles.key, handles.fill, handles.rim] {
            let node_data = scene.node(node).expect("node");
            let NodeKind::Light(light_key) = node_data.kind else {
                panic!("light node");
            };
            let Light::Directional(light) = scene.light(light_key).expect("light") else {
                panic!("directional");
            };
            illuminances.push(light.illuminance_lux());
        }
        for lux in &illuminances {
            assert!(
                *lux < 20_000.0,
                "studio preset must stay under 20k lux per light (got {lux})"
            );
        }
        let total: f32 = illuminances.iter().sum();
        assert!(
            total < 30_000.0,
            "combined studio preset under 30k lux total (got {total})"
        );
    }
}
