use crate::assets::EnvironmentHandle;
use crate::diagnostics::DebugOverlay;
use crate::picking::InteractionStyle;

use super::{Renderer, Tonemapper};

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

    pub const fn profile(self) -> Profile {
        self.profile
    }

    pub const fn explicit_quality(self) -> Option<Quality> {
        self.quality
    }

    pub const fn explicit_render_mode(self) -> Option<RenderMode> {
        self.render_mode
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

    pub fn exposure_ev(&self) -> f32 {
        self.output.exposure_ev()
    }

    pub fn set_exposure_ev(&mut self, exposure_ev: f32) {
        self.output.set_exposure_ev(exposure_ev);
    }

    pub fn tonemapper(&self) -> Tonemapper {
        self.output.tonemapper()
    }

    pub fn set_tonemapper(&mut self, tonemapper: Tonemapper) {
        self.output.set_tonemapper(tonemapper);
    }

    pub fn debug_overlay(&self) -> DebugOverlay {
        self.debug_overlay
    }

    pub fn set_debug(&mut self, overlay: DebugOverlay) {
        if self.debug_overlay != overlay {
            self.debug_overlay = overlay;
            self.debug_revision = self.debug_revision.saturating_add(1);
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
            self.environment_revision = self.environment_revision.saturating_add(1);
        }
    }

    pub fn clear_environment(&mut self) {
        if self.environment.is_some() {
            self.environment = None;
            self.environment_revision = self.environment_revision.saturating_add(1);
        }
    }
}
