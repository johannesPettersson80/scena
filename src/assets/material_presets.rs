use crate::diagnostics::AssetError;
use crate::material::{Color, MaterialDesc, TextureColorSpace, TextureTransform};

use super::{AssetFetcher, Assets, MaterialHandle, TextureHandle};

#[cfg(target_arch = "wasm32")]
macro_rules! sample_path {
    ($path:literal) => {
        concat!("samples/", $path)
    };
}

#[cfg(not(target_arch = "wasm32"))]
macro_rules! sample_path {
    ($path:literal) => {
        concat!("demo/samples/", $path)
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialPresetProvenance {
    pub id: &'static str,
    pub source: &'static str,
    pub source_page: &'static str,
    pub license: &'static str,
    pub base_color_path: &'static str,
    pub normal_path: &'static str,
    pub metallic_roughness_path: &'static str,
    pub texture_bytes_budget: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct MaterialPresetAssets<'a, F> {
    assets: &'a Assets<F>,
}

const SATIN: MaterialPresetProvenance = MaterialPresetProvenance {
    id: "satin",
    source: "ambientCG Fabric001",
    source_page: "https://ambientcg.com/a/Fabric001",
    license: "CC0",
    base_color_path: sample_path!("materials/ambientcg/Fabric001/demo-512/Fabric001_512_Color.jpg"),
    normal_path: sample_path!("materials/ambientcg/Fabric001/demo-512/Fabric001_512_NormalGL.jpg"),
    metallic_roughness_path: sample_path!(
        "materials/ambientcg/Fabric001/demo-512/Fabric001_512_OcclusionRoughnessMetallic.png"
    ),
    texture_bytes_budget: 800_000,
};

const LEATHER: MaterialPresetProvenance = MaterialPresetProvenance {
    id: "leather",
    source: "ambientCG Leather001",
    source_page: "https://ambientcg.com/a/Leather001",
    license: "CC0",
    base_color_path: sample_path!(
        "materials/ambientcg/Leather001/demo-512/Leather001_512_Color.jpg"
    ),
    normal_path: sample_path!(
        "materials/ambientcg/Leather001/demo-512/Leather001_512_NormalGL.jpg"
    ),
    metallic_roughness_path: sample_path!(
        "materials/ambientcg/Leather001/demo-512/Leather001_512_OcclusionRoughnessMetallic.png"
    ),
    texture_bytes_budget: 800_000,
};

const RUBBER: MaterialPresetProvenance = MaterialPresetProvenance {
    id: "rubber",
    source: "ambientCG Rubber002",
    source_page: "https://ambientcg.com/a/Rubber002",
    license: "CC0",
    base_color_path: sample_path!("materials/ambientcg/Rubber002/demo-512/Rubber002_512_Color.jpg"),
    normal_path: sample_path!("materials/ambientcg/Rubber002/demo-512/Rubber002_512_NormalGL.jpg"),
    metallic_roughness_path: sample_path!(
        "materials/ambientcg/Rubber002/demo-512/Rubber002_512_OcclusionRoughnessMetallic.png"
    ),
    texture_bytes_budget: 800_000,
};

const SOURCE_BACKED_PRESETS: [MaterialPresetProvenance; 3] = [SATIN, LEATHER, RUBBER];

impl<F> Assets<F> {
    pub fn material_presets(&self) -> MaterialPresetAssets<'_, F> {
        MaterialPresetAssets { assets: self }
    }
}

pub const fn source_backed_material_preset_provenance() -> &'static [MaterialPresetProvenance] {
    &SOURCE_BACKED_PRESETS
}

impl<'a, F> MaterialPresetAssets<'a, F>
where
    F: AssetFetcher,
{
    pub async fn satin(&self) -> Result<MaterialHandle, AssetError> {
        self.load_textured_material(
            SATIN,
            MaterialDesc::satin(Color::WHITE)
                .with_double_sided(true)
                .with_metallic_roughness_texture_transform(tile_transform(3.0))
                .with_normal_texture_transform(tile_transform(3.0))
                .with_base_color_texture_transform(tile_transform(3.0)),
        )
        .await
    }

    pub async fn leather(&self) -> Result<MaterialHandle, AssetError> {
        self.load_textured_material(
            LEATHER,
            MaterialDesc::leather(Color::from_srgb_u8(142, 82, 48))
                .with_double_sided(true)
                .with_metallic_roughness_texture_transform(tile_transform(2.5))
                .with_normal_texture_transform(tile_transform(2.5))
                .with_base_color_texture_transform(tile_transform(2.5)),
        )
        .await
    }

    pub async fn rubber(&self) -> Result<MaterialHandle, AssetError> {
        self.load_textured_material(
            RUBBER,
            MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0)
                .with_double_sided(true)
                .with_metallic_roughness_texture_transform(tile_transform(3.5))
                .with_normal_texture_transform(tile_transform(3.5))
                .with_base_color_texture_transform(tile_transform(3.5)),
        )
        .await
    }

    async fn load_textured_material(
        &self,
        provenance: MaterialPresetProvenance,
        material: MaterialDesc,
    ) -> Result<MaterialHandle, AssetError> {
        let base_color = self
            .load_required_texture(provenance.base_color_path, TextureColorSpace::Srgb)
            .await?;
        let normal = self
            .load_required_texture(provenance.normal_path, TextureColorSpace::Linear)
            .await?;
        let metallic_roughness = self
            .load_required_texture(
                provenance.metallic_roughness_path,
                TextureColorSpace::Linear,
            )
            .await?;
        Ok(self.assets.create_material(
            material
                .with_base_color_texture(base_color)
                .with_normal_texture(normal)
                .with_metallic_roughness_texture(metallic_roughness)
                .with_occlusion_texture(metallic_roughness),
        ))
    }

    async fn load_required_texture(
        &self,
        path: &'static str,
        color_space: TextureColorSpace,
    ) -> Result<TextureHandle, AssetError> {
        let texture = self.assets.load_texture(path, color_space).await?;
        let decoded = self.assets.try_texture(texture)?.has_decoded_pixels();
        if decoded {
            Ok(texture)
        } else {
            Err(AssetError::Parse {
                path: path.to_string(),
                reason: "source-backed material preset texture did not decode".to_string(),
            })
        }
    }
}

const fn tile_transform(scale: f32) -> TextureTransform {
    TextureTransform::new([0.0, 0.0], 0.0, [scale, scale], None)
}
