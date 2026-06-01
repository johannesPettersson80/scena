use crate::assets::EnvironmentHandle;
use crate::diagnostics::{DebugOverlay, OutputColorSpace};
use crate::material::Color;
use crate::picking::InteractionStyle;

use super::{
    AntiAliasing, Background, OrderIndependentTransparencyConfig, PostBloomConfig, Renderer,
    ScreenSpaceAmbientOcclusionConfig, Tonemapper,
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

    pub fn exposure_ev(&self) -> f32 {
        self.output.exposure_ev()
    }

    pub fn set_exposure_ev(&mut self, exposure_ev: f32) {
        let before = self.output.exposure_ev();
        self.output.set_exposure_ev(exposure_ev);
        if self.output.exposure_ev() != before {
            self.mark_output_changed();
        }
    }

    pub fn tonemapper(&self) -> Tonemapper {
        self.output.tonemapper()
    }

    pub fn anti_aliasing(&self) -> AntiAliasing {
        self.anti_aliasing
    }

    pub fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing) {
        if self.anti_aliasing != anti_aliasing {
            self.anti_aliasing = anti_aliasing;
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
            self.mark_output_changed();
        }
    }

    pub fn clear_screen_space_ambient_occlusion(&mut self) {
        self.set_screen_space_ambient_occlusion(None);
    }

    pub fn set_bloom(&mut self, bloom: Option<PostBloomConfig>) {
        if self.bloom != bloom {
            self.bloom = bloom;
            self.mark_output_changed();
        }
    }

    pub fn clear_bloom(&mut self) {
        self.set_bloom(None);
    }

    pub fn debug_overlay(&self) -> DebugOverlay {
        self.debug_overlay
    }

    pub fn set_debug(&mut self, overlay: DebugOverlay) {
        self.set_debug_overlay(overlay);
    }

    pub fn set_debug_overlay(&mut self, overlay: DebugOverlay) {
        if self.debug_overlay != overlay {
            self.debug_overlay = overlay;
            self.debug_revision = self.debug_revision.saturating_add(1);
            self.clear_rendered_frame();
        }
    }

    pub fn hover_style(&self) -> InteractionStyle {
        self.hover_style
    }

    pub fn set_hover_style(&mut self, style: InteractionStyle) {
        self.hover_style = style;
    }

    pub fn selection_style(&self) -> InteractionStyle {
        self.selection_style
    }

    pub fn set_selection_style(&mut self, style: InteractionStyle) {
        self.selection_style = style;
    }

    pub fn environment(&self) -> Option<EnvironmentHandle> {
        self.environment
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
}
