use crate::assets::EnvironmentHandle;
use crate::picking::InteractionStyle;

use super::{Renderer, Tonemapper};

impl Renderer {
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
