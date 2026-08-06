use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use image::{GrayImage, Rgba, RgbaImage};
use sha2::{Digest, Sha256};

use super::{
    PHOTOGRAPHIC_MATERIAL_ARCHIVE_MAX_BYTES, PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V1,
    PhotographicMaterialCatalogEntryV1, PhotographicMaterialCatalogEntryV2,
    PhotographicMaterialCategoryV1, PhotographicMaterialPackMapRoleV1,
    PhotographicMaterialPackMapV1, PhotographicMaterialPackSourceV1, PhotographicMaterialPackV1,
    PhotographicMaterialPackV2, PhotographicMaterialResolutionV1,
};

const MAX_ARCHIVE_ENTRIES: usize = 256;
const MAX_MAP_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_MAP_BYTES: u64 = 256 * 1024 * 1024;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub enum PhotographicMaterialPackError {
    ArchiveTooLarge {
        bytes: usize,
        maximum: usize,
    },
    InvalidArchive(String),
    TooManyArchiveEntries {
        count: usize,
        maximum: usize,
    },
    MapTooLarge {
        name: String,
        bytes: u64,
        maximum: u64,
    },
    TotalMapBytesTooLarge {
        bytes: u64,
        maximum: u64,
    },
    DuplicateMap {
        role: &'static str,
    },
    MissingMap {
        role: &'static str,
    },
    ImageDecode {
        role: &'static str,
        reason: String,
    },
    DimensionMismatch {
        role: &'static str,
        expected: [u32; 2],
        actual: [u32; 2],
    },
    ResolutionUnavailable {
        resolution: PhotographicMaterialResolutionV1,
    },
    ResolutionDimensionMismatch {
        resolution: PhotographicMaterialResolutionV1,
        expected: u32,
        actual: [u32; 2],
    },
    OutputExists(PathBuf),
    Io {
        path: PathBuf,
        reason: String,
    },
    Manifest(String),
}

impl fmt::Display for PhotographicMaterialPackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArchiveTooLarge { bytes, maximum } => {
                write!(
                    formatter,
                    "material archive is {bytes} bytes; maximum is {maximum}"
                )
            }
            Self::InvalidArchive(reason) => {
                write!(formatter, "invalid material ZIP archive: {reason}")
            }
            Self::TooManyArchiveEntries { count, maximum } => write!(
                formatter,
                "material archive contains {count} entries; maximum is {maximum}"
            ),
            Self::MapTooLarge {
                name,
                bytes,
                maximum,
            } => write!(
                formatter,
                "material map '{name}' is {bytes} bytes; maximum is {maximum}"
            ),
            Self::TotalMapBytesTooLarge { bytes, maximum } => write!(
                formatter,
                "selected material maps total {bytes} bytes; maximum is {maximum}"
            ),
            Self::DuplicateMap { role } => {
                write!(formatter, "material archive contains multiple {role} maps")
            }
            Self::MissingMap { role } => {
                write!(formatter, "material archive is missing required {role} map")
            }
            Self::ImageDecode { role, reason } => {
                write!(formatter, "failed to decode {role} map: {reason}")
            }
            Self::DimensionMismatch {
                role,
                expected,
                actual,
            } => write!(
                formatter,
                "{role} dimensions {}x{} do not match {}x{}",
                actual[0], actual[1], expected[0], expected[1]
            ),
            Self::ResolutionUnavailable { resolution } => write!(
                formatter,
                "material catalog does not expose a {} archive",
                resolution.as_str()
            ),
            Self::ResolutionDimensionMismatch {
                resolution,
                expected,
                actual,
            } => write!(
                formatter,
                "{} material archive decoded to {}x{}, expected {expected}x{expected}",
                resolution.as_str(),
                actual[0],
                actual[1]
            ),
            Self::OutputExists(path) => {
                write!(
                    formatter,
                    "material pack output already exists: {}",
                    path.display()
                )
            }
            Self::Io { path, reason } => {
                write!(formatter, "{}: {reason}", path.display())
            }
            Self::Manifest(reason) => {
                write!(
                    formatter,
                    "failed to serialize material pack manifest: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for PhotographicMaterialPackError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SourceMapRole {
    BaseColor,
    NormalGl,
    Roughness,
    Metalness,
    Occlusion,
}

impl SourceMapRole {
    const fn as_str(self) -> &'static str {
        match self {
            Self::BaseColor => "base_color",
            Self::NormalGl => "normal_gl",
            Self::Roughness => "roughness",
            Self::Metalness => "metalness",
            Self::Occlusion => "occlusion",
        }
    }
}

pub fn compile_photographic_material_archive(
    entry: &PhotographicMaterialCatalogEntryV1,
    archive_bytes: &[u8],
    output_dir: impl AsRef<Path>,
) -> Result<PhotographicMaterialPackV1, PhotographicMaterialPackError> {
    match compile_archive(entry, archive_bytes, output_dir.as_ref(), None)? {
        CompiledPack::V1(pack) => Ok(pack),
        CompiledPack::V2(_) => unreachable!("legacy compile requests a v1 pack"),
    }
}

pub fn compile_photographic_material_archive_at_resolution(
    entry: &PhotographicMaterialCatalogEntryV2,
    resolution: PhotographicMaterialResolutionV1,
    archive_bytes: &[u8],
    output_dir: impl AsRef<Path>,
) -> Result<PhotographicMaterialPackV2, PhotographicMaterialPackError> {
    let selected = entry
        .for_resolution(resolution)
        .ok_or(PhotographicMaterialPackError::ResolutionUnavailable { resolution })?;
    match compile_archive(
        &selected,
        archive_bytes,
        output_dir.as_ref(),
        Some(resolution),
    )? {
        CompiledPack::V2(pack) => Ok(pack),
        CompiledPack::V1(_) => unreachable!("resolution compile requests a v2 pack"),
    }
}

enum CompiledPack {
    V1(PhotographicMaterialPackV1),
    V2(PhotographicMaterialPackV2),
}

fn compile_archive(
    entry: &PhotographicMaterialCatalogEntryV1,
    archive_bytes: &[u8],
    output_dir: &Path,
    resolution: Option<PhotographicMaterialResolutionV1>,
) -> Result<CompiledPack, PhotographicMaterialPackError> {
    if archive_bytes.len() > PHOTOGRAPHIC_MATERIAL_ARCHIVE_MAX_BYTES {
        return Err(PhotographicMaterialPackError::ArchiveTooLarge {
            bytes: archive_bytes.len(),
            maximum: PHOTOGRAPHIC_MATERIAL_ARCHIVE_MAX_BYTES,
        });
    }
    if output_dir.exists() {
        return Err(PhotographicMaterialPackError::OutputExists(
            output_dir.to_path_buf(),
        ));
    }
    let parent = output_dir.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| io_error(parent, error))?;
    let output_name = output_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("material-pack");
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary_dir = parent.join(format!(
        ".{output_name}.tmp-{}-{sequence}",
        std::process::id()
    ));
    if temporary_dir.exists() {
        fs::remove_dir_all(&temporary_dir).map_err(|error| io_error(&temporary_dir, error))?;
    }
    fs::create_dir(&temporary_dir).map_err(|error| io_error(&temporary_dir, error))?;

    let result = compile_into(entry, archive_bytes, &temporary_dir, resolution);
    let pack = match result {
        Ok(pack) => pack,
        Err(error) => {
            let _ = fs::remove_dir_all(&temporary_dir);
            return Err(error);
        }
    };
    if let Err(error) = fs::rename(&temporary_dir, output_dir) {
        let _ = fs::remove_dir_all(&temporary_dir);
        return Err(io_error(output_dir, error));
    }
    Ok(pack)
}

fn compile_into(
    entry: &PhotographicMaterialCatalogEntryV1,
    archive_bytes: &[u8],
    output_dir: &Path,
    resolution: Option<PhotographicMaterialResolutionV1>,
) -> Result<CompiledPack, PhotographicMaterialPackError> {
    let maps = read_source_maps(archive_bytes)?;
    let base_color = decode_rgba(required_map(&maps, SourceMapRole::BaseColor)?, "base_color")?;
    let normal = decode_rgba(required_map(&maps, SourceMapRole::NormalGl)?, "normal_gl")?;
    let roughness = decode_luma(required_map(&maps, SourceMapRole::Roughness)?, "roughness")?;
    let dimensions = base_color.dimensions();
    if let Some(resolution) = resolution {
        let expected = resolution.dimension_px();
        if dimensions != (expected, expected) {
            return Err(PhotographicMaterialPackError::ResolutionDimensionMismatch {
                resolution,
                expected,
                actual: [dimensions.0, dimensions.1],
            });
        }
    }
    require_dimensions("normal_gl", dimensions, normal.dimensions())?;
    require_dimensions("roughness", dimensions, roughness.dimensions())?;

    let metalness = match maps.get(&SourceMapRole::Metalness) {
        Some(bytes) => {
            let image = decode_luma(bytes, "metalness")?;
            require_dimensions("metalness", dimensions, image.dimensions())?;
            Some(image)
        }
        None if entry.category == PhotographicMaterialCategoryV1::Metal => {
            return Err(PhotographicMaterialPackError::MissingMap { role: "metalness" });
        }
        None => None,
    };
    let occlusion = match maps.get(&SourceMapRole::Occlusion) {
        Some(bytes) => {
            let image = decode_luma(bytes, "occlusion")?;
            require_dimensions("occlusion", dimensions, image.dimensions())?;
            Some(image)
        }
        None => None,
    };

    let base_color_path = output_dir.join("base-color.png");
    base_color
        .save(&base_color_path)
        .map_err(|error| image_write_error(&base_color_path, error))?;
    let normal_path = output_dir.join("normal-gl.png");
    normal
        .save(&normal_path)
        .map_err(|error| image_write_error(&normal_path, error))?;

    let mut orm = RgbaImage::new(dimensions.0, dimensions.1);
    for y in 0..dimensions.1 {
        for x in 0..dimensions.0 {
            let ao = occlusion
                .as_ref()
                .map_or(255, |image| image.get_pixel(x, y).0[0]);
            let rough = roughness.get_pixel(x, y).0[0];
            let metal = metalness
                .as_ref()
                .map_or(0, |image| image.get_pixel(x, y).0[0]);
            orm.put_pixel(x, y, Rgba([ao, rough, metal, 255]));
        }
    }
    let orm_path = output_dir.join("occlusion-roughness-metallic.png");
    orm.save(&orm_path)
        .map_err(|error| image_write_error(&orm_path, error))?;

    let output_maps = vec![
        output_map(
            output_dir,
            PhotographicMaterialPackMapRoleV1::BaseColor,
            "base-color.png",
            "srgb",
            dimensions,
        )?,
        output_map(
            output_dir,
            PhotographicMaterialPackMapRoleV1::NormalGl,
            "normal-gl.png",
            "linear",
            dimensions,
        )?,
        output_map(
            output_dir,
            PhotographicMaterialPackMapRoleV1::OcclusionRoughnessMetallic,
            "occlusion-roughness-metallic.png",
            "linear",
            dimensions,
        )?,
    ];
    let pack = PhotographicMaterialPackV1 {
        schema: PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V1.to_string(),
        id: entry.id.clone(),
        label: entry.label.clone(),
        category: entry.category,
        surface_kind: entry.surface_kind,
        recommended_tile_size_m: entry.recommended_tile_size_m,
        source: PhotographicMaterialPackSourceV1 {
            provider: entry.provider.clone(),
            provider_asset_id: entry.provider_asset_id.clone(),
            source_page: entry.source_page.clone(),
            archive_uri: entry.archive_uri.clone(),
            archive_sha256: sha256_hex(archive_bytes),
            archive_bytes: archive_bytes.len() as u64,
            license: entry.license.clone(),
        },
        maps: output_maps,
    };
    let compiled = match resolution {
        Some(resolution) => CompiledPack::V2(PhotographicMaterialPackV2::from_v1(pack, resolution)),
        None => CompiledPack::V1(pack),
    };
    let manifest = match &compiled {
        CompiledPack::V1(pack) => serde_json::to_vec_pretty(pack),
        CompiledPack::V2(pack) => serde_json::to_vec_pretty(pack),
    }
    .map_err(|error| PhotographicMaterialPackError::Manifest(error.to_string()))?;
    let manifest_path = output_dir.join("scena-material-pack.json");
    fs::write(&manifest_path, manifest).map_err(|error| io_error(&manifest_path, error))?;
    Ok(compiled)
}

fn read_source_maps(
    archive_bytes: &[u8],
) -> Result<BTreeMap<SourceMapRole, Vec<u8>>, PhotographicMaterialPackError> {
    let cursor = Cursor::new(archive_bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|error| PhotographicMaterialPackError::InvalidArchive(error.to_string()))?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(PhotographicMaterialPackError::TooManyArchiveEntries {
            count: archive.len(),
            maximum: MAX_ARCHIVE_ENTRIES,
        });
    }
    let mut maps = BTreeMap::new();
    let mut total_bytes = 0_u64;
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|error| PhotographicMaterialPackError::InvalidArchive(error.to_string()))?;
        if file.is_dir() || file.enclosed_name().is_none() {
            continue;
        }
        let Some(role) = source_map_role(file.name()) else {
            continue;
        };
        if file.size() > MAX_MAP_BYTES {
            return Err(PhotographicMaterialPackError::MapTooLarge {
                name: file.name().to_string(),
                bytes: file.size(),
                maximum: MAX_MAP_BYTES,
            });
        }
        total_bytes = total_bytes.saturating_add(file.size());
        if total_bytes > MAX_TOTAL_MAP_BYTES {
            return Err(PhotographicMaterialPackError::TotalMapBytesTooLarge {
                bytes: total_bytes,
                maximum: MAX_TOTAL_MAP_BYTES,
            });
        }
        if maps.contains_key(&role) {
            return Err(PhotographicMaterialPackError::DuplicateMap {
                role: role.as_str(),
            });
        }
        let mut bytes = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut bytes)
            .map_err(|error| PhotographicMaterialPackError::InvalidArchive(error.to_string()))?;
        maps.insert(role, bytes);
    }
    Ok(maps)
}

fn source_map_role(name: &str) -> Option<SourceMapRole> {
    let normalized = name.replace('\\', "/").to_ascii_lowercase();
    if !matches!(
        Path::new(&normalized)
            .extension()
            .and_then(|extension| extension.to_str()),
        Some("jpg" | "jpeg" | "png" | "webp")
    ) {
        return None;
    }
    if normalized.contains("_normalgl.") {
        Some(SourceMapRole::NormalGl)
    } else if normalized.contains("_color.") {
        Some(SourceMapRole::BaseColor)
    } else if normalized.contains("_roughness.") {
        Some(SourceMapRole::Roughness)
    } else if normalized.contains("_metalness.") {
        Some(SourceMapRole::Metalness)
    } else if normalized.contains("_ambientocclusion.")
        || normalized.contains("_occlusion.")
        || normalized.contains("_ao.")
    {
        Some(SourceMapRole::Occlusion)
    } else {
        None
    }
}

fn required_map(
    maps: &BTreeMap<SourceMapRole, Vec<u8>>,
    role: SourceMapRole,
) -> Result<&[u8], PhotographicMaterialPackError> {
    maps.get(&role)
        .map(Vec::as_slice)
        .ok_or(PhotographicMaterialPackError::MissingMap {
            role: role.as_str(),
        })
}

fn decode_rgba(
    bytes: &[u8],
    role: &'static str,
) -> Result<RgbaImage, PhotographicMaterialPackError> {
    image::load_from_memory(bytes)
        .map(|image| image.to_rgba8())
        .map_err(|error| PhotographicMaterialPackError::ImageDecode {
            role,
            reason: error.to_string(),
        })
}

fn decode_luma(
    bytes: &[u8],
    role: &'static str,
) -> Result<GrayImage, PhotographicMaterialPackError> {
    image::load_from_memory(bytes)
        .map(|image| image.to_luma8())
        .map_err(|error| PhotographicMaterialPackError::ImageDecode {
            role,
            reason: error.to_string(),
        })
}

fn require_dimensions(
    role: &'static str,
    expected: (u32, u32),
    actual: (u32, u32),
) -> Result<(), PhotographicMaterialPackError> {
    if expected == actual {
        Ok(())
    } else {
        Err(PhotographicMaterialPackError::DimensionMismatch {
            role,
            expected: [expected.0, expected.1],
            actual: [actual.0, actual.1],
        })
    }
}

fn output_map(
    output_dir: &Path,
    role: PhotographicMaterialPackMapRoleV1,
    path: &str,
    color_space: &str,
    dimensions: (u32, u32),
) -> Result<PhotographicMaterialPackMapV1, PhotographicMaterialPackError> {
    let bytes =
        fs::read(output_dir.join(path)).map_err(|error| io_error(&output_dir.join(path), error))?;
    Ok(PhotographicMaterialPackMapV1 {
        role,
        path: path.to_string(),
        color_space: color_space.to_string(),
        sha256: sha256_hex(&bytes),
        width: dimensions.0,
        height: dimensions.1,
    })
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

fn io_error(path: &Path, error: std::io::Error) -> PhotographicMaterialPackError {
    PhotographicMaterialPackError::Io {
        path: path.to_path_buf(),
        reason: error.to_string(),
    }
}

fn image_write_error(path: &Path, error: image::ImageError) -> PhotographicMaterialPackError {
    PhotographicMaterialPackError::Io {
        path: path.to_path_buf(),
        reason: error.to_string(),
    }
}
