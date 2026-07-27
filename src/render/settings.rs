use crate::assets::EnvironmentHandle;
use crate::diagnostics::OutputColorSpace;
use crate::material::Color;

use super::{
    AntiAliasing, AutoExposureSubjectMetering, AutoExposureSubjectRect, Background,
    DepthOfFieldConfig, OrderIndependentTransparencyConfig, PostBloomConfig, ReconstructionFilter,
    Renderer, ScreenSpaceAmbientOcclusionConfig, ScreenSpaceReflectionConfig, Tonemapper,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Profile {
    #[default]
    Auto,
    Quality,
    Balanced,
    Compatibility,
    Industrial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Quality {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum RenderMode {
    #[default]
    Manual,
    OnChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RendererOptions {
    profile: Profile,
    quality: Option<Quality>,
    render_mode: Option<RenderMode>,
    output_color_space: OutputColorSpace,
    semantic_aov_capture: bool,
}

impl RendererOptions {
    pub const fn with_profile(mut self, profile: Profile) -> Self {
        self.profile = profile;
        self
    }

    pub const fn with_quality(mut self, quality: Quality) -> Self {
        self.quality = Some(quality);
        self
    }

    pub const fn with_render_mode(mut self, render_mode: RenderMode) -> Self {
        self.render_mode = Some(render_mode);
        self
    }

    pub const fn with_output_color_space(mut self, output_color_space: OutputColorSpace) -> Self {
        self.output_color_space = output_color_space;
        self
    }

    /// Retains lifecycle-owned GPU targets and readback buffers for semantic
    /// ID, linear-depth, and world-normal capture. CPU semantic capture does
    /// not require this opt-in.
    pub const fn with_semantic_aov_capture(mut self, enabled: bool) -> Self {
        self.semantic_aov_capture = enabled;
        self
    }

    pub const fn profile(self) -> Profile {
        self.profile
    }

    pub const fn explicit_quality(self) -> Option<Quality> {
        self.quality
    }

    pub const fn explicit_render_mode(self) -> Option<RenderMode> {
        self.render_mode
    }

    pub const fn output_color_space(self) -> OutputColorSpace {
        self.output_color_space
    }

    pub const fn semantic_aov_capture(self) -> bool {
        self.semantic_aov_capture
    }
}

impl Renderer {
    pub fn profile(&self) -> Profile {
        self.profile
    }

    pub fn quality(&self) -> Quality {
        self.quality
    }

    pub fn render_mode(&self) -> RenderMode {
        self.render_mode
    }

    pub fn output_color_space(&self) -> OutputColorSpace {
        self.output_color_space
    }

    pub const fn semantic_aov_capture_enabled(&self) -> bool {
        self.semantic_aov_capture_enabled
    }

    /// Changes whether the next GPU `prepare()` builds semantic AOV targets.
    /// Resource creation remains explicit in prepare and never occurs inside
    /// render or capture.
    pub fn set_semantic_aov_capture_enabled(&mut self, enabled: bool) {
        if self.semantic_aov_capture_enabled != enabled {
            self.semantic_aov_capture_enabled = enabled;
            self.mark_output_resources_changed();
        }
    }

    pub fn exposure_ev(&self) -> f32 {
        self.output.exposure_ev()
    }

    /// Sets a fixed exposure and records that the caller chose it explicitly.
    ///
    /// Scene setup presets treat an explicit choice as authoritative and will
    /// not install their own auto exposure over it. Metering must not use this
    /// method for its own result -- see `set_metered_exposure_ev`.
    pub fn set_exposure_ev(&mut self, exposure_ev: f32) {
        self.explicit_exposure_ev = true;
        self.set_metered_exposure_ev(exposure_ev);
    }

    /// Writes an exposure without claiming it was an explicit caller choice.
    pub(super) fn set_metered_exposure_ev(&mut self, exposure_ev: f32) {
        let before = self.output.exposure_ev();
        self.output.set_exposure_ev(exposure_ev);
        if self.output.exposure_ev() != before {
            self.mark_output_changed();
        }
    }

    /// Returns whether a caller set a fixed exposure through
    /// [`Self::set_exposure_ev`].
    pub const fn has_explicit_exposure_ev(&self) -> bool {
        self.explicit_exposure_ev
    }

    pub fn set_auto_exposure_subject_metering(
        &mut self,
        subject_rect: AutoExposureSubjectRect,
        surround_weight: f32,
    ) {
        let metering = AutoExposureSubjectMetering::new(subject_rect, surround_weight);
        if self.auto_exposure_subject_metering != Some(metering) {
            self.auto_exposure_subject_metering = Some(metering);
            self.last_auto_exposure = None;
            self.auto_exposure_status = super::AutoExposureStatus::Pending;
            self.mark_output_changed();
        }
    }

    pub fn clear_auto_exposure_subject_metering(&mut self) {
        if self.auto_exposure_subject_metering.take().is_some() {
            self.last_auto_exposure = None;
            self.auto_exposure_status = super::AutoExposureStatus::Pending;
            self.mark_output_changed();
        }
    }

    pub const fn auto_exposure_subject_metering(&self) -> Option<AutoExposureSubjectMetering> {
        self.auto_exposure_subject_metering
    }

    pub fn tonemapper(&self) -> Tonemapper {
        self.output.tonemapper()
    }

    pub const fn white_balance(&self) -> super::WhiteBalance {
        self.output.white_balance()
    }

    pub fn set_white_balance(&mut self, white_balance: super::WhiteBalance) {
        if self.output.white_balance() != white_balance {
            self.output.set_white_balance(white_balance);
            self.mark_output_changed();
        }
    }

    pub fn anti_aliasing(&self) -> AntiAliasing {
        self.anti_aliasing
    }

    /// Returns whether the CPU prepare path may run its coarse occlusion
    /// prepass. GPU prepares never use the CPU prepass.
    pub const fn cpu_occlusion_culling(&self) -> bool {
        self.cpu_occlusion_culling
    }

    /// Enables or disables the CPU occlusion prepass.
    ///
    /// This is a performance policy only; disabling it must not change
    /// rendered pixels. The setting invalidates prepared state because it can
    /// change the retained primitive list.
    pub fn set_cpu_occlusion_culling(&mut self, enabled: bool) {
        if self.cpu_occlusion_culling != enabled {
            self.cpu_occlusion_culling = enabled;
            self.target_revision = self.target_revision.saturating_add(1);
            self.clear_rendered_frame();
        }
    }

    pub fn supersample_factor(&self) -> u32 {
        self.supersample_factor
    }

    pub fn reconstruction_filter(&self) -> ReconstructionFilter {
        self.reconstruction_filter
    }

    pub fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing) {
        self.configuration_diagnostics
            .retain(|diagnostic| diagnostic.code != crate::DiagnosticCode::MultisampleFallback);
        self.diagnostics
            .retain(|diagnostic| diagnostic.code != crate::DiagnosticCode::MultisampleFallback);
        if self.anti_aliasing != anti_aliasing {
            self.anti_aliasing = anti_aliasing;
            self.mark_output_resources_changed();
        }
    }

    pub fn set_supersample_factor(&mut self, factor: u32) -> Result<(), crate::RenderError> {
        super::target::validate_supersample_target(self.target, factor)?;
        if self.supersample_factor != factor {
            self.supersample_factor = factor;
            self.target_revision = self.target_revision.saturating_add(1);
            self.clear_rendered_frame();
            self.mark_output_changed();
        }
        Ok(())
    }

    pub fn set_reconstruction_filter(&mut self, filter: ReconstructionFilter) {
        if self.reconstruction_filter != filter {
            self.reconstruction_filter = filter;
            self.mark_output_changed();
        }
    }

    pub fn set_tonemapper(&mut self, tonemapper: Tonemapper) {
        if self.output.tonemapper() != tonemapper {
            self.output.set_tonemapper(tonemapper);
            self.mark_output_changed();
        }
    }

    pub fn bloom(&self) -> Option<PostBloomConfig> {
        self.bloom
    }

    pub fn order_independent_transparency(&self) -> Option<OrderIndependentTransparencyConfig> {
        self.order_independent_transparency
    }

    pub fn set_order_independent_transparency(
        &mut self,
        config: Option<OrderIndependentTransparencyConfig>,
    ) {
        if self.order_independent_transparency != config {
            self.order_independent_transparency = config;
            self.mark_output_changed();
        }
    }

    pub fn clear_order_independent_transparency(&mut self) {
        self.set_order_independent_transparency(None);
    }

    pub fn screen_space_ambient_occlusion(&self) -> Option<ScreenSpaceAmbientOcclusionConfig> {
        self.screen_space_ambient_occlusion
    }

    pub fn set_screen_space_ambient_occlusion(
        &mut self,
        config: Option<ScreenSpaceAmbientOcclusionConfig>,
    ) {
        if self.screen_space_ambient_occlusion != config {
            self.screen_space_ambient_occlusion = config;
            self.mark_output_resources_changed();
        }
    }

    pub fn clear_screen_space_ambient_occlusion(&mut self) {
        self.set_screen_space_ambient_occlusion(None);
    }

    pub fn screen_space_reflections(&self) -> Option<ScreenSpaceReflectionConfig> {
        self.screen_space_reflections
    }

    pub fn set_screen_space_reflections(&mut self, config: Option<ScreenSpaceReflectionConfig>) {
        if self.screen_space_reflections != config {
            self.screen_space_reflections = config;
            self.mark_output_resources_changed();
        }
    }

    pub fn clear_screen_space_reflections(&mut self) {
        self.set_screen_space_reflections(None);
    }

    pub fn depth_of_field(&self) -> Option<DepthOfFieldConfig> {
        self.depth_of_field
    }

    pub fn set_depth_of_field(&mut self, config: Option<DepthOfFieldConfig>) {
        if self.depth_of_field != config {
            self.depth_of_field = config;
            self.mark_output_resources_changed();
        }
    }

    pub fn clear_depth_of_field(&mut self) {
        self.set_depth_of_field(None);
    }

    pub fn set_bloom(&mut self, bloom: Option<PostBloomConfig>) {
        if self.bloom != bloom {
            self.bloom = bloom;
            self.mark_output_resources_changed();
        }
    }

    pub fn clear_bloom(&mut self) {
        self.set_bloom(None);
    }

    pub fn environment(&self) -> Option<EnvironmentHandle> {
        self.environment
    }

    pub const fn environment_intensity(&self) -> f32 {
        self.environment_intensity
    }

    pub fn set_environment_intensity(&mut self, intensity: f32) {
        let intensity = if intensity.is_finite() {
            intensity.clamp(0.0, 16.0)
        } else {
            1.0
        };
        if self.environment_intensity != intensity {
            self.environment_intensity = intensity;
            self.environment_revision = self.environment_revision.saturating_add(1);
            self.clear_rendered_frame();
        }
    }

    pub fn environment_rotation_y_degrees(&self) -> f32 {
        self.environment_rotation_y_radians.to_degrees()
    }

    pub fn set_environment_rotation_y_degrees(&mut self, degrees: f32) {
        let radians = if degrees.is_finite() {
            degrees.to_radians().rem_euclid(std::f32::consts::TAU)
        } else {
            0.0
        };
        if self.environment_rotation_y_radians != radians {
            self.environment_rotation_y_radians = radians;
            self.environment_revision = self.environment_revision.saturating_add(1);
            self.clear_rendered_frame();
        }
    }

    pub fn set_environment(&mut self, environment: EnvironmentHandle) {
        if self.environment != Some(environment) {
            self.environment = Some(environment);
            self.environment_lighting_cache.clear_active();
            self.environment_revision = self.environment_revision.saturating_add(1);
            self.clear_rendered_frame();
        }
    }

    pub fn clear_environment(&mut self) {
        if self.environment.is_some() {
            self.environment = None;
            self.environment_lighting_cache.clear_active();
            self.environment_revision = self.environment_revision.saturating_add(1);
            self.clear_rendered_frame();
        }
    }

    /// Current background clear color.
    pub fn background_color(&self) -> Color {
        self.background_color
    }

    /// Sets the background to an explicit linear RGBA [`Color`].
    ///
    /// Prefer [`Renderer::set_background`] with a named [`Background`]
    /// scheme; reach for this raw setter when a specific brand color is needed.
    pub fn set_background_color(&mut self, color: Color) {
        if self.background_color != color {
            self.background_color = color;
            self.mark_output_changed();
        }
    }

    /// Sets the background from a named [`Background`] scheme.
    ///
    /// Equivalent to [`Renderer::set_background_color`] with the scheme's
    /// resolved color. First-path code should reach for this method instead
    /// of constructing raw colors.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use scena::{Background, Renderer};
    /// # fn example() -> scena::Result<()> {
    /// let mut renderer = Renderer::headless(1280, 720)?;
    /// renderer.set_background(Background::Studio);
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_background(&mut self, background: Background) {
        self.set_background_color(background.color());
    }

    pub(super) fn mark_output_changed(&mut self) {
        self.render_generation = self.render_generation.saturating_add(1);
        self.clear_rendered_frame();
    }

    pub(super) fn mark_output_resources_changed(&mut self) {
        if self.gpu.is_some() {
            self.output_resources_revision = self.output_resources_revision.saturating_add(1);
        }
        self.mark_output_changed();
    }

    /// Revision of output settings that own prepared GPU resources.
    pub const fn output_resources_revision(&self) -> u64 {
        self.output_resources_revision
    }
}
