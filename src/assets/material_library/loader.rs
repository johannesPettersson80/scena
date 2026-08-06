use std::collections::BTreeMap;
use std::path::{Component, Path};

use sha2::{Digest, Sha256};

#[cfg(feature = "scene-host")]
use super::PhotographicMaterialPackBinding;
use super::{
    PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V1, PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V2,
    PhotographicMaterialPackAssets, PhotographicMaterialPackMapRoleV1,
    PhotographicMaterialPackMapV1, PhotographicMaterialPackV1, PhotographicMaterialPackV2,
    PhotographicMaterialResolutionV1,
};
use crate::assets::{
    AssetFetcher, AssetPath, Assets, TextureFilter, TextureMemoryDesc, TextureMemoryId,
    TextureMipPolicy, TextureSamplerDesc, TextureSlot, TextureWrap,
};
use crate::diagnostics::AssetError;
use crate::material::{Color, MaterialDesc};

const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
const MAX_PACK_MAP_BYTES: usize = 64 * 1024 * 1024;

impl<F> Assets<F>
where
    F: AssetFetcher,
{
    /// Loads a compiled photographic material pack into normal scena
    /// texture and material handles.
    ///
    /// Every map is hash-checked against the pack manifest before decode.
    /// Map paths must remain below the manifest directory; renders never
    /// contact the catalog provider.
    pub async fn load_photographic_material_pack(
        &self,
        manifest_path: impl Into<AssetPath>,
    ) -> Result<PhotographicMaterialPackAssets, AssetError> {
        let manifest_path = manifest_path.into();
        let manifest_bytes = self.tracked_fetcher().fetch(&manifest_path).await?;
        if manifest_bytes.len() > MAX_MANIFEST_BYTES {
            return Err(pack_error(
                &manifest_path,
                format!(
                    "material pack manifest is {} bytes; maximum is {MAX_MANIFEST_BYTES}",
                    manifest_bytes.len()
                ),
            ));
        }
        let manifest: serde_json::Value =
            serde_json::from_slice(&manifest_bytes).map_err(|error| {
                pack_error(
                    &manifest_path,
                    format!("material pack manifest is not valid JSON: {error}"),
                )
            })?;
        let schema = manifest
            .get("schema")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let (pack, manifest_schema, resolution) = match schema.as_str() {
            PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V1 => {
                let pack: PhotographicMaterialPackV1 =
                    serde_json::from_value(manifest).map_err(|error| {
                        pack_error(
                            &manifest_path,
                            format!("v1 material pack manifest is invalid: {error}"),
                        )
                    })?;
                (
                    pack,
                    PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V1.to_owned(),
                    PhotographicMaterialResolutionV1::OneK,
                )
            }
            PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V2 => {
                let pack: PhotographicMaterialPackV2 =
                    serde_json::from_value(manifest).map_err(|error| {
                        pack_error(
                            &manifest_path,
                            format!("v2 material pack manifest is invalid: {error}"),
                        )
                    })?;
                validate_pack_v2(&manifest_path, &pack)?;
                let resolution = pack.resolution;
                (
                    pack.as_v1_compatibility_pack(),
                    PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V2.to_owned(),
                    resolution,
                )
            }
            _ => {
                return Err(pack_error(
                    &manifest_path,
                    format!(
                        "unsupported material pack schema '{schema}'; expected {PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V1} or {PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V2}"
                    ),
                ));
            }
        };
        validate_pack(&manifest_path, &pack)?;

        let mut textures = BTreeMap::new();
        for map in &pack.maps {
            let path = resolve_pack_map_path(&manifest_path, &map.path)?;
            let bytes = self.tracked_fetcher().fetch(&path).await?;
            if bytes.len() > MAX_PACK_MAP_BYTES {
                return Err(pack_error(
                    &path,
                    format!(
                        "{} map is {} bytes; maximum is {MAX_PACK_MAP_BYTES}",
                        map.role.as_str(),
                        bytes.len()
                    ),
                ));
            }
            let observed_sha256 = sha256_hex(&bytes);
            if observed_sha256 != map.sha256 {
                return Err(pack_error(
                    &path,
                    format!(
                        "{} map SHA-256 mismatch: expected {}, observed {observed_sha256}",
                        map.role.as_str(),
                        map.sha256
                    ),
                ));
            }
            let image = image::load_from_memory(&bytes).map_err(|error| {
                pack_error(
                    &path,
                    format!("failed to decode {} map: {error}", map.role.as_str()),
                )
            })?;
            let rgba = image.to_rgba8();
            if rgba.dimensions() != (map.width, map.height) {
                return Err(pack_error(
                    &path,
                    format!(
                        "{} map dimensions {}x{} do not match manifest {}x{}",
                        map.role.as_str(),
                        rgba.width(),
                        rgba.height(),
                        map.width,
                        map.height
                    ),
                ));
            }
            let slot = texture_slot(map.role);
            validate_map_color_space(&manifest_path, map, slot)?;
            let identity =
                TextureMemoryId::new(if manifest_schema == PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V1 {
                    format!(
                        "scena/material-pack/v1/{}/{}",
                        map.role.as_str(),
                        map.sha256
                    )
                } else {
                    format!(
                        "scena/material-pack/v2/{}/{}/{}/{}",
                        pack.id,
                        resolution.as_str(),
                        map.role.as_str(),
                        map.sha256
                    )
                })?;
            let sampler = TextureSamplerDesc::new(
                Some(TextureFilter::Linear),
                Some(TextureFilter::LinearMipmapLinear),
                TextureWrap::Repeat,
                TextureWrap::Repeat,
            );
            let handle = self.create_texture_for_slot(
                TextureMemoryDesc::rgba8_for_slot(
                    identity,
                    map.width,
                    map.height,
                    rgba.into_raw(),
                    slot,
                )
                .with_sampler(sampler)
                .with_mip_policy(TextureMipPolicy::Generate),
                slot,
            )?;
            textures.insert(map.role, handle);
        }

        let base_color_texture = required_texture(
            &manifest_path,
            &textures,
            PhotographicMaterialPackMapRoleV1::BaseColor,
        )?;
        let normal_texture = required_texture(
            &manifest_path,
            &textures,
            PhotographicMaterialPackMapRoleV1::NormalGl,
        )?;
        let metallic_roughness_texture = required_texture(
            &manifest_path,
            &textures,
            PhotographicMaterialPackMapRoleV1::OcclusionRoughnessMetallic,
        )?;
        let material = self.create_material(
            MaterialDesc::pbr_metallic_roughness(Color::WHITE, 1.0, 1.0)
                .with_base_color_texture(base_color_texture)
                .with_normal_texture(normal_texture)
                .with_metallic_roughness_texture(metallic_roughness_texture)
                .with_occlusion_texture(metallic_roughness_texture)
                .with_photographic_surface_tile_size_m(pack.recommended_tile_size_m),
        );
        #[cfg(feature = "scene-host")]
        self.storage().photographic_material_pack_bindings.insert(
            material,
            PhotographicMaterialPackBinding {
                manifest_path: manifest_path.clone(),
                pack_id: pack.id.clone(),
                resolution,
            },
        );
        Ok(PhotographicMaterialPackAssets {
            pack,
            manifest_schema,
            manifest_path,
            resolution,
            material,
            base_color_texture,
            normal_texture,
            metallic_roughness_texture,
        })
    }

    #[cfg(feature = "scene-host")]
    pub(crate) fn photographic_material_pack_binding(
        &self,
        material: crate::MaterialHandle,
    ) -> Option<PhotographicMaterialPackBinding> {
        self.storage()
            .photographic_material_pack_bindings
            .get(&material)
            .cloned()
    }

    #[cfg(feature = "scene-host")]
    pub(crate) fn create_photographic_material_pack_derivative(
        &self,
        source: crate::MaterialHandle,
        material: MaterialDesc,
    ) -> Result<crate::MaterialHandle, AssetError> {
        let binding = self
            .photographic_material_pack_binding(source)
            .ok_or(AssetError::MaterialHandleNotFound { material: source })?;
        let handle = self.create_material(material);
        self.storage()
            .photographic_material_pack_bindings
            .insert(handle, binding);
        Ok(handle)
    }
}

fn validate_pack_v2(path: &AssetPath, pack: &PhotographicMaterialPackV2) -> Result<(), AssetError> {
    if pack.schema != PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V2 {
        return Err(pack_error(
            path,
            format!(
                "unsupported material pack schema '{}'; expected {PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V2}",
                pack.schema
            ),
        ));
    }
    let expected = pack.resolution.dimension_px();
    if let Some(map) = pack
        .maps
        .iter()
        .find(|map| map.width != expected || map.height != expected)
    {
        return Err(pack_error(
            path,
            format!(
                "{} pack contains a {} map at {}x{}; expected {expected}x{expected}",
                pack.resolution.as_str(),
                map.role.as_str(),
                map.width,
                map.height
            ),
        ));
    }
    Ok(())
}

fn validate_pack(path: &AssetPath, pack: &PhotographicMaterialPackV1) -> Result<(), AssetError> {
    if pack.schema != PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V1 {
        return Err(pack_error(
            path,
            format!(
                "unsupported material pack schema '{}'; expected {PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V1}",
                pack.schema
            ),
        ));
    }
    if pack.id.trim().is_empty() {
        return Err(pack_error(path, "material pack id must not be empty"));
    }
    if !pack.recommended_tile_size_m.is_finite() || pack.recommended_tile_size_m <= 0.0 {
        return Err(pack_error(
            path,
            "material pack recommended_tile_size_m must be finite and positive",
        ));
    }
    if !valid_sha256(&pack.source.archive_sha256) {
        return Err(pack_error(
            path,
            "material pack source archive_sha256 must contain 64 hexadecimal digits",
        ));
    }
    if pack.maps.len() != 3 {
        return Err(pack_error(
            path,
            format!(
                "material pack must contain exactly three canonical maps, found {}",
                pack.maps.len()
            ),
        ));
    }
    let mut roles = BTreeMap::new();
    for map in &pack.maps {
        if !valid_sha256(&map.sha256) {
            return Err(pack_error(
                path,
                format!(
                    "{} map sha256 must contain 64 hexadecimal digits",
                    map.role.as_str()
                ),
            ));
        }
        if map.width == 0 || map.height == 0 {
            return Err(pack_error(
                path,
                format!("{} map dimensions must be non-zero", map.role.as_str()),
            ));
        }
        if roles.insert(map.role, ()).is_some() {
            return Err(pack_error(
                path,
                format!("material pack repeats {} map", map.role.as_str()),
            ));
        }
    }
    for role in [
        PhotographicMaterialPackMapRoleV1::BaseColor,
        PhotographicMaterialPackMapRoleV1::NormalGl,
        PhotographicMaterialPackMapRoleV1::OcclusionRoughnessMetallic,
    ] {
        if !roles.contains_key(&role) {
            return Err(pack_error(
                path,
                format!("material pack is missing {} map", role.as_str()),
            ));
        }
    }
    Ok(())
}

fn validate_map_color_space(
    path: &AssetPath,
    map: &PhotographicMaterialPackMapV1,
    slot: TextureSlot,
) -> Result<(), AssetError> {
    let expected = match slot {
        TextureSlot::BaseColor | TextureSlot::Emissive | TextureSlot::SheenColor => "srgb",
        _ => "linear",
    };
    if map.color_space == expected {
        Ok(())
    } else {
        Err(pack_error(
            path,
            format!(
                "{} map color_space '{}' does not match required '{expected}'",
                map.role.as_str(),
                map.color_space
            ),
        ))
    }
}

const fn texture_slot(role: PhotographicMaterialPackMapRoleV1) -> TextureSlot {
    match role {
        PhotographicMaterialPackMapRoleV1::BaseColor => TextureSlot::BaseColor,
        PhotographicMaterialPackMapRoleV1::NormalGl => TextureSlot::Normal,
        PhotographicMaterialPackMapRoleV1::OcclusionRoughnessMetallic => {
            TextureSlot::MetallicRoughness
        }
    }
}

fn required_texture(
    path: &AssetPath,
    textures: &BTreeMap<PhotographicMaterialPackMapRoleV1, crate::TextureHandle>,
    role: PhotographicMaterialPackMapRoleV1,
) -> Result<crate::TextureHandle, AssetError> {
    textures.get(&role).copied().ok_or_else(|| {
        pack_error(
            path,
            format!("material pack is missing {} map", role.as_str()),
        )
    })
}

fn resolve_pack_map_path(
    manifest_path: &AssetPath,
    relative: &str,
) -> Result<AssetPath, AssetError> {
    let relative_path = Path::new(relative);
    if relative.is_empty()
        || relative.contains('\\')
        || relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(pack_error(
            manifest_path,
            format!("material pack map path '{relative}' must stay below the manifest directory"),
        ));
    }
    let manifest = manifest_path.as_str();
    if manifest.contains("://") {
        let Some((base, _)) = manifest.rsplit_once('/') else {
            return Err(pack_error(
                manifest_path,
                "URL material pack manifest has no parent path",
            ));
        };
        Ok(AssetPath::from(format!("{base}/{relative}")))
    } else {
        let resolved = Path::new(manifest)
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(relative);
        Ok(AssetPath::from(resolved.display().to_string()))
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    encoded
}

fn pack_error(path: &AssetPath, reason: impl Into<String>) -> AssetError {
    AssetError::Parse {
        path: path.as_str().to_string(),
        reason: reason.into(),
    }
}
