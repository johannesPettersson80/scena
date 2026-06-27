use crate::render::{Background, Profile, RenderMode};

pub const VIEWER_PROFILE_NAMES: &[&str] = &[
    "model_viewer",
    "cad_inspection",
    "product",
    "industrial",
    "documentation",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerProfileLighting {
    None,
    Directional,
    Studio,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewerProfile {
    name: &'static str,
    renderer_profile: Profile,
    render_mode: RenderMode,
    background: Option<Background>,
    default_environment: bool,
    lighting: ViewerProfileLighting,
    grid: bool,
    default_picking: bool,
    section_controls: bool,
    orbit_controls: bool,
}

impl ViewerProfile {
    pub const fn model_viewer() -> Self {
        Self {
            name: "model_viewer",
            renderer_profile: Profile::Balanced,
            render_mode: RenderMode::OnChange,
            background: Some(Background::Studio),
            default_environment: true,
            lighting: ViewerProfileLighting::Directional,
            grid: false,
            default_picking: false,
            section_controls: false,
            orbit_controls: true,
        }
    }

    pub const fn cad_inspection() -> Self {
        Self {
            name: "cad_inspection",
            renderer_profile: Profile::Industrial,
            render_mode: RenderMode::OnChange,
            background: Some(Background::NeutralGray),
            default_environment: false,
            lighting: ViewerProfileLighting::Studio,
            grid: true,
            default_picking: true,
            section_controls: true,
            orbit_controls: true,
        }
    }

    pub const fn product() -> Self {
        Self {
            name: "product",
            renderer_profile: Profile::Quality,
            render_mode: RenderMode::OnChange,
            background: Some(Background::Studio),
            default_environment: true,
            lighting: ViewerProfileLighting::Studio,
            grid: false,
            default_picking: true,
            section_controls: false,
            orbit_controls: true,
        }
    }

    pub const fn industrial() -> Self {
        Self {
            name: "industrial",
            renderer_profile: Profile::Industrial,
            render_mode: RenderMode::OnChange,
            background: Some(Background::DarkStudio),
            default_environment: true,
            lighting: ViewerProfileLighting::Studio,
            grid: true,
            default_picking: true,
            section_controls: true,
            orbit_controls: true,
        }
    }

    pub const fn documentation() -> Self {
        Self {
            name: "documentation",
            renderer_profile: Profile::Quality,
            render_mode: RenderMode::Manual,
            background: Some(Background::White),
            default_environment: true,
            lighting: ViewerProfileLighting::Studio,
            grid: false,
            default_picking: false,
            section_controls: false,
            orbit_controls: false,
        }
    }

    pub fn named(name: &str) -> Option<Self> {
        match name {
            "model_viewer" => Some(Self::model_viewer()),
            "cad_inspection" => Some(Self::cad_inspection()),
            "product" => Some(Self::product()),
            "industrial" => Some(Self::industrial()),
            "documentation" => Some(Self::documentation()),
            _ => None,
        }
    }

    pub const fn names() -> &'static [&'static str] {
        VIEWER_PROFILE_NAMES
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn renderer_profile(&self) -> Profile {
        self.renderer_profile
    }

    pub const fn render_mode(&self) -> RenderMode {
        self.render_mode
    }

    pub const fn background(&self) -> Option<Background> {
        self.background
    }

    pub const fn default_environment(&self) -> bool {
        self.default_environment
    }

    pub const fn lighting(&self) -> ViewerProfileLighting {
        self.lighting
    }

    pub const fn grid(&self) -> bool {
        self.grid
    }

    pub const fn default_picking(&self) -> bool {
        self.default_picking
    }

    pub const fn section_controls(&self) -> bool {
        self.section_controls
    }

    pub const fn orbit_controls(&self) -> bool {
        self.orbit_controls
    }

    pub const fn with_grid(mut self, enabled: bool) -> Self {
        self.grid = enabled;
        self
    }

    pub const fn with_default_picking(mut self, enabled: bool) -> Self {
        self.default_picking = enabled;
        self
    }

    pub const fn with_section_controls(mut self, enabled: bool) -> Self {
        self.section_controls = enabled;
        self
    }

    pub const fn with_orbit_controls(mut self, enabled: bool) -> Self {
        self.orbit_controls = enabled;
        self
    }

    pub const fn with_background(mut self, background: Option<Background>) -> Self {
        self.background = background;
        self
    }
}
