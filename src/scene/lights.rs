use crate::diagnostics::LookupError;
use crate::material::Color;

use super::{Angle, LightKey, NodeKey, NodeKind, Scene, Transform};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Light {
    Directional(DirectionalLight),
    Point(PointLight),
    Spot(SpotLight),
    Area(AreaLight),
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AreaLightShape {
    Rect { width: f32, height: f32 },
    Disc { radius: f32 },
    Sphere { radius: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaLight {
    color: Color,
    luminous_flux_lumens: f32,
    range: Option<f32>,
    shape: AreaLightShape,
}

/// Builder returned by [`Scene::directional_light`], [`Scene::point_light`],
/// [`Scene::spot_light`], and [`Scene::area_light`].
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

    /// Inserts a studio-style three-point directional rig.
    ///
    /// The rig contains a key light, cool fill, and warm rim light. The
    /// returned handles are ordered as key, fill, and rim so callers can
    /// adjust or remove individual lights after insertion. Intensities are
    /// tuned for neutral glTF product/model-viewer scenes without
    /// over-exposing PBR metallic body materials.
    ///
    /// This preset uses moderate intensities (key 13,500 lux, fill 4,500 lux,
    /// rim 3,500 lux). Only the key casts shadows because the renderer
    /// supports one shadowed directional light per scene.
    ///
    /// # Errors
    ///
    /// Returns a [`LookupError`] if the scene cannot insert one of the light
    /// nodes under the root.
    pub fn add_studio_lighting(&mut self) -> Result<StudioLightingHandles, LookupError> {
        let key = self
            .directional_light(DirectionalLight::key_light())
            .transform(Transform::default().rotate_x_deg(-30.0).rotate_y_deg(20.0))
            .add()?;
        let fill = self
            .directional_light(DirectionalLight::fill_light())
            .transform(
                Transform::default()
                    .rotate_x_deg(-10.0)
                    .rotate_y_deg(-120.0),
            )
            .add()?;
        let rim = self
            .directional_light(DirectionalLight::rim_light())
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

    pub fn area_light(&mut self, light: AreaLight) -> LightBuilder<'_> {
        self.light_builder(Light::Area(light))
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

/// Handles for the three lights inserted by [`Scene::add_studio_lighting`].
///
/// Returned so callers can later adjust an individual light, for example to
/// raise the key, tint the rim, or remove the rig.
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
    /// Direct sunlight preset: neutral daylight, high illuminance, shadowed.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::DirectionalLight;
    ///
    /// let sun = DirectionalLight::sun();
    /// assert!(sun.casts_shadows());
    /// assert!(sun.illuminance_lux() > DirectionalLight::key_light().illuminance_lux());
    /// ```
    pub fn sun() -> Self {
        Self::default()
            .with_color(Color::from_kelvin(5600.0))
            .with_illuminance_lux(110_000.0)
            .with_shadows(true)
    }

    /// Product-viewer key light preset used by [`Scene::add_studio_lighting`].
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::DirectionalLight;
    ///
    /// let key = DirectionalLight::key_light();
    /// assert!(key.casts_shadows());
    /// assert_eq!(key.illuminance_lux(), 13_500.0);
    /// ```
    pub fn key_light() -> Self {
        Self::default()
            .with_color(Color::WHITE)
            .with_illuminance_lux(13_500.0)
            .with_shadows(true)
    }

    /// Cool fill light preset used by [`Scene::add_studio_lighting`].
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{Color, DirectionalLight};
    ///
    /// let fill = DirectionalLight::fill_light();
    /// assert_eq!(fill.color(), Color::COOL_WHITE);
    /// assert!(!fill.casts_shadows());
    /// ```
    pub fn fill_light() -> Self {
        Self::default()
            .with_color(Color::COOL_WHITE)
            .with_illuminance_lux(4_500.0)
    }

    /// Warm rim light preset used by [`Scene::add_studio_lighting`].
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{Color, DirectionalLight};
    ///
    /// let rim = DirectionalLight::rim_light();
    /// assert_eq!(rim.color(), Color::WARM_WHITE);
    /// assert!(!rim.casts_shadows());
    /// ```
    pub fn rim_light() -> Self {
        Self::default()
            .with_color(Color::WARM_WHITE)
            .with_illuminance_lux(3_500.0)
    }

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
    /// Neutral softbox-like point-light approximation for simple product shots.
    ///
    /// This is still a point light, not an area light; it is named for the
    /// workflow role rather than a physically large emitter shape.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::PointLight;
    ///
    /// let light = PointLight::softbox();
    /// assert_eq!(light.range(), Some(4.0));
    /// ```
    pub fn softbox() -> Self {
        Self::default()
            .with_color(Color::from_kelvin(5600.0))
            .with_intensity_candela(900.0)
            .with_range(4.0)
    }

    /// Warm practical bulb preset, approximately 2700K.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::PointLight;
    ///
    /// let light = PointLight::bulb_warm();
    /// assert_eq!(light.range(), Some(6.0));
    /// assert!(light.color().r > light.color().b);
    /// ```
    pub fn bulb_warm() -> Self {
        Self::default()
            .with_color(Color::from_kelvin(2700.0))
            .with_intensity_candela(450.0)
            .with_range(6.0)
    }

    /// Cool practical bulb preset, approximately 5600K.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::PointLight;
    ///
    /// let warm = PointLight::bulb_warm();
    /// let cool = PointLight::bulb_cool();
    /// assert!(cool.color().b > warm.color().b);
    /// ```
    pub fn bulb_cool() -> Self {
        Self::default()
            .with_color(Color::from_kelvin(5600.0))
            .with_intensity_candela(450.0)
            .with_range(6.0)
    }

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

impl Default for AreaLightShape {
    fn default() -> Self {
        Self::Rect {
            width: 1.0,
            height: 1.0,
        }
    }
}

impl AreaLightShape {
    pub const fn rect(width: f32, height: f32) -> Self {
        Self::Rect {
            width: positive_or(width, 1.0),
            height: positive_or(height, 1.0),
        }
    }

    pub const fn disc(radius: f32) -> Self {
        Self::Disc {
            radius: positive_or(radius, 0.5),
        }
    }

    pub const fn sphere(radius: f32) -> Self {
        Self::Sphere {
            radius: positive_or(radius, 0.5),
        }
    }
}

impl Default for AreaLight {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            luminous_flux_lumens: 1_000.0,
            range: None,
            shape: AreaLightShape::default(),
        }
    }
}

impl AreaLight {
    /// Rectangular studio softbox preset for product-style lighting.
    ///
    /// The current renderer evaluates area lights by deterministic emitter
    /// samples in the prepare step so the same authored shape affects the CPU
    /// and GPU paths. Dedicated soft-shadow maps remain a later rendering
    /// slice and are not exposed as a shadow knob here.
    pub fn softbox() -> Self {
        Self::default()
            .with_color(Color::from_kelvin(5600.0))
            .with_luminous_flux_lumens(3_600.0)
            .with_range(5.0)
            .with_shape(AreaLightShape::rect(1.2, 0.6))
    }

    pub const fn color(self) -> Color {
        self.color
    }

    pub const fn luminous_flux_lumens(self) -> f32 {
        self.luminous_flux_lumens
    }

    pub const fn range(self) -> Option<f32> {
        self.range
    }

    pub const fn shape(self) -> AreaLightShape {
        self.shape
    }

    pub const fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub const fn with_luminous_flux_lumens(mut self, luminous_flux_lumens: f32) -> Self {
        self.luminous_flux_lumens = non_negative_or(luminous_flux_lumens, 1_000.0);
        self
    }

    pub const fn with_range(mut self, range: f32) -> Self {
        self.range = positive_range(range);
        self
    }

    pub const fn with_shape(mut self, shape: AreaLightShape) -> Self {
        self.shape = shape;
        self
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

const fn positive_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
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
        // Keep the preset moderate so PBR material differences stay visible.
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
        assert_eq!(illuminances, [13_500.0, 4_500.0, 3_500.0]);
        assert!(
            total < 30_000.0,
            "combined studio preset under 30k lux total (got {total})"
        );
    }

    #[test]
    fn add_studio_lighting_shadows_only_the_key_light() {
        let mut scene = Scene::new();
        let handles = scene.add_studio_lighting().expect("inserts");

        let casts_shadows = |scene: &Scene, node| -> bool {
            let node_data = scene.node(node).expect("node");
            let NodeKind::Light(light_key) = node_data.kind else {
                panic!("light node");
            };
            let Light::Directional(light) = scene.light(light_key).expect("light") else {
                panic!("directional");
            };
            light.casts_shadows()
        };

        assert!(
            casts_shadows(&scene, handles.key),
            "studio key light should cast the single supported directional shadow"
        );
        assert!(
            !casts_shadows(&scene, handles.fill),
            "studio fill light must not cast a second directional shadow"
        );
        assert!(
            !casts_shadows(&scene, handles.rim),
            "studio rim light must not cast a second directional shadow"
        );
    }

    #[test]
    fn area_light_softbox_uses_real_area_shape() {
        let light = AreaLight::softbox();
        assert_eq!(
            light.shape(),
            AreaLightShape::Rect {
                width: 1.2,
                height: 0.6
            }
        );
        assert_eq!(light.luminous_flux_lumens(), 3_600.0);
        assert_eq!(light.range(), Some(5.0));
    }
}
