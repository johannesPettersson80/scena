//! Shared material-preset showcase fixture used by the public demo and proof lanes.

use crate::{Color, GeometryDesc, MaterialDesc, Transform, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialShowcasePreset {
    pub id: &'static str,
    pub label: &'static str,
    pub source_surface: &'static str,
    pub geometry: MaterialShowcaseGeometry,
    pub lighting_mode: MaterialShowcaseLighting,
    grid_column: u32,
    grid_row: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialShowcaseBackgroundBar {
    pub offset: Vec3,
    pub scale: Vec3,
    pub color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MaterialShowcaseGeometry {
    CurvedPanel,
    CurvedPart,
    BrushedPlate,
    FoldedSheet,
    StrapPanel,
    GlassBlockGrid,
    GlassScreenGrid,
    GasketFoot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialShowcaseLighting {
    Studio,
    IblOnly,
}

const MATERIAL_DESC_SURFACE: &str = "MaterialDesc";
const SOURCE_BACKED_SURFACE: &str = "Assets::material_presets()";

const GLASS_BACKGROUND_TARGET_BARS: [MaterialShowcaseBackgroundBar; 9] = [
    MaterialShowcaseBackgroundBar::new(
        Vec3::new(0.0, 0.0, -0.006),
        Vec3::new(0.56, 0.38, 0.006),
        Color::WHITE,
    ),
    MaterialShowcaseBackgroundBar::new(
        Vec3::new(0.0, -0.13, 0.006),
        Vec3::new(0.50, 0.040, 0.010),
        Color::BLACK,
    ),
    MaterialShowcaseBackgroundBar::new(
        Vec3::new(0.0, -0.06, 0.006),
        Vec3::new(0.50, 0.040, 0.010),
        Color::BLACK,
    ),
    MaterialShowcaseBackgroundBar::new(
        Vec3::new(0.0, 0.01, 0.006),
        Vec3::new(0.50, 0.040, 0.010),
        Color::BLACK,
    ),
    MaterialShowcaseBackgroundBar::new(
        Vec3::new(0.0, 0.08, 0.006),
        Vec3::new(0.50, 0.040, 0.010),
        Color::BLACK,
    ),
    MaterialShowcaseBackgroundBar::new(
        Vec3::new(0.0, 0.15, 0.006),
        Vec3::new(0.50, 0.040, 0.010),
        Color::BLACK,
    ),
    MaterialShowcaseBackgroundBar::new(
        Vec3::new(-0.19, 0.0, 0.006),
        Vec3::new(0.040, 0.34, 0.010),
        Color::BLACK,
    ),
    MaterialShowcaseBackgroundBar::new(
        Vec3::new(0.0, 0.0, 0.006),
        Vec3::new(0.040, 0.34, 0.010),
        Color::BLACK,
    ),
    MaterialShowcaseBackgroundBar::new(
        Vec3::new(0.19, 0.0, 0.006),
        Vec3::new(0.040, 0.34, 0.010),
        Color::BLACK,
    ),
];

const MATERIAL_PRESET_SHOWCASE: [MaterialShowcasePreset; 12] = [
    MaterialShowcasePreset::new(
        "matte",
        "Matte",
        MATERIAL_DESC_SURFACE,
        MaterialShowcaseGeometry::CurvedPanel,
        MaterialShowcaseLighting::Studio,
        0,
        0,
    ),
    MaterialShowcasePreset::new(
        "plastic",
        "Plastic",
        MATERIAL_DESC_SURFACE,
        MaterialShowcaseGeometry::CurvedPanel,
        MaterialShowcaseLighting::Studio,
        1,
        0,
    ),
    MaterialShowcasePreset::new(
        "metal",
        "Metal",
        MATERIAL_DESC_SURFACE,
        MaterialShowcaseGeometry::CurvedPart,
        MaterialShowcaseLighting::IblOnly,
        2,
        0,
    ),
    MaterialShowcasePreset::new(
        "rough_metal",
        "Rough metal",
        MATERIAL_DESC_SURFACE,
        MaterialShowcaseGeometry::CurvedPart,
        MaterialShowcaseLighting::IblOnly,
        3,
        0,
    ),
    MaterialShowcasePreset::new(
        "chrome",
        "Chrome",
        MATERIAL_DESC_SURFACE,
        MaterialShowcaseGeometry::CurvedPart,
        MaterialShowcaseLighting::IblOnly,
        0,
        1,
    ),
    MaterialShowcasePreset::new(
        "brushed_steel",
        "Brushed steel",
        MATERIAL_DESC_SURFACE,
        MaterialShowcaseGeometry::BrushedPlate,
        MaterialShowcaseLighting::IblOnly,
        1,
        1,
    ),
    MaterialShowcasePreset::new(
        "clearcoat_plastic",
        "Clearcoat plastic",
        MATERIAL_DESC_SURFACE,
        MaterialShowcaseGeometry::CurvedPanel,
        MaterialShowcaseLighting::IblOnly,
        2,
        1,
    ),
    MaterialShowcasePreset::new(
        "satin",
        "Satin",
        SOURCE_BACKED_SURFACE,
        MaterialShowcaseGeometry::FoldedSheet,
        MaterialShowcaseLighting::Studio,
        3,
        1,
    ),
    MaterialShowcasePreset::new(
        "leather",
        "Leather",
        SOURCE_BACKED_SURFACE,
        MaterialShowcaseGeometry::StrapPanel,
        MaterialShowcaseLighting::Studio,
        0,
        2,
    ),
    MaterialShowcasePreset::new(
        "clear_glass",
        "Clear glass",
        MATERIAL_DESC_SURFACE,
        MaterialShowcaseGeometry::GlassBlockGrid,
        MaterialShowcaseLighting::IblOnly,
        1,
        2,
    ),
    MaterialShowcasePreset::new(
        "frosted_glass",
        "Frosted glass",
        MATERIAL_DESC_SURFACE,
        MaterialShowcaseGeometry::GlassScreenGrid,
        MaterialShowcaseLighting::IblOnly,
        2,
        2,
    ),
    MaterialShowcasePreset::new(
        "rubber",
        "Rubber",
        SOURCE_BACKED_SURFACE,
        MaterialShowcaseGeometry::GasketFoot,
        MaterialShowcaseLighting::Studio,
        3,
        2,
    ),
];

pub fn material_preset_showcase() -> &'static [MaterialShowcasePreset] {
    &MATERIAL_PRESET_SHOWCASE
}

pub const fn glass_background_target_bars() -> &'static [MaterialShowcaseBackgroundBar] {
    &GLASS_BACKGROUND_TARGET_BARS
}

impl MaterialShowcaseBackgroundBar {
    const fn new(offset: Vec3, scale: Vec3, color: Color) -> Self {
        Self {
            offset,
            scale,
            color,
        }
    }
}

impl MaterialShowcasePreset {
    const fn new(
        id: &'static str,
        label: &'static str,
        source_surface: &'static str,
        geometry: MaterialShowcaseGeometry,
        lighting_mode: MaterialShowcaseLighting,
        grid_column: u32,
        grid_row: u32,
    ) -> Self {
        Self {
            id,
            label,
            source_surface,
            geometry,
            lighting_mode,
            grid_column,
            grid_row,
        }
    }

    pub fn material_desc(self) -> MaterialDesc {
        match self.id {
            "matte" => MaterialDesc::matte(Color::BLUE),
            "plastic" => MaterialDesc::plastic(Color::BLUE),
            "metal" => MaterialDesc::metal(Color::LIGHT_GRAY),
            "rough_metal" => MaterialDesc::rough_metal(Color::GRAY),
            "chrome" => MaterialDesc::chrome(),
            "brushed_steel" => MaterialDesc::brushed_steel(),
            "clearcoat_plastic" => MaterialDesc::clearcoat_plastic(Color::BLUE),
            "satin" => MaterialDesc::satin(Color::MAGENTA),
            "leather" => MaterialDesc::leather(Color::ORANGE),
            "clear_glass" => MaterialDesc::clear_glass(Color::COOL_WHITE),
            "frosted_glass" => MaterialDesc::frosted_glass(Color::WHITE),
            "rubber" => MaterialDesc::rubber(),
            _ => unreachable!("material showcase ids are fixed"),
        }
    }

    pub fn geometry_desc(self) -> GeometryDesc {
        match self.geometry {
            MaterialShowcaseGeometry::CurvedPanel | MaterialShowcaseGeometry::CurvedPart => {
                GeometryDesc::sphere(1.0, 64, 32)
            }
            MaterialShowcaseGeometry::BrushedPlate
            | MaterialShowcaseGeometry::FoldedSheet
            | MaterialShowcaseGeometry::StrapPanel
            | MaterialShowcaseGeometry::GlassBlockGrid
            | MaterialShowcaseGeometry::GlassScreenGrid => GeometryDesc::box_xyz(1.0, 1.0, 1.0),
            MaterialShowcaseGeometry::GasketFoot => GeometryDesc::cylinder(1.0, 1.0, 48),
        }
    }

    pub fn transform(self) -> Transform {
        Transform {
            translation: self.position(),
            rotation: self.geometry.rotation(),
            scale: self.geometry.scale(),
        }
    }

    pub fn label_position(self) -> Vec3 {
        let position = self.position();
        Vec3::new(position.x, position.y - 0.245, position.z)
    }

    pub fn background_target_position(self) -> Option<Vec3> {
        self.geometry
            .uses_background_target()
            .then(|| self.position() + Vec3::new(0.0, 0.0, -0.14))
    }

    pub fn position(self) -> Vec3 {
        let x = -0.9 + self.grid_column as f32 * 0.6;
        let y = 0.58 - self.grid_row as f32 * 0.56;
        Vec3::new(x, y, 0.0)
    }
}

impl MaterialShowcaseGeometry {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CurvedPanel => "curved-panel",
            Self::CurvedPart => "curved-part",
            Self::BrushedPlate => "brushed-plate",
            Self::FoldedSheet => "folded-sheet",
            Self::StrapPanel => "strap-panel",
            Self::GlassBlockGrid => "glass-block-grid",
            Self::GlassScreenGrid => "glass-screen-grid",
            Self::GasketFoot => "gasket-foot",
        }
    }

    pub fn scale(self) -> Vec3 {
        match self {
            Self::CurvedPanel => Vec3::new(0.20, 0.15, 0.12),
            Self::CurvedPart => Vec3::new(0.18, 0.18, 0.18),
            Self::BrushedPlate => Vec3::new(0.34, 0.08, 0.025),
            Self::FoldedSheet => Vec3::new(0.30, 0.13, 0.035),
            Self::StrapPanel => Vec3::new(0.32, 0.08, 0.035),
            Self::GlassBlockGrid => Vec3::new(0.22, 0.15, 0.09),
            Self::GlassScreenGrid => Vec3::new(0.28, 0.16, 0.025),
            Self::GasketFoot => Vec3::new(0.12, 0.06, 0.12),
        }
    }

    pub fn rotation(self) -> crate::Quat {
        match self {
            Self::BrushedPlate => crate::Quat::from_rotation_z(-0.12),
            Self::FoldedSheet => crate::Quat::from_rotation_z(0.16),
            Self::StrapPanel => crate::Quat::from_rotation_z(-0.08),
            _ => crate::Quat::IDENTITY,
        }
    }

    pub const fn uses_background_target(self) -> bool {
        matches!(self, Self::GlassBlockGrid | Self::GlassScreenGrid)
    }
}
