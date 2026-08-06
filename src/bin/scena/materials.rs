#[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
use std::path::{Path, PathBuf};

#[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
use sha2::{Digest, Sha256};

#[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
use super::scena_cli_error::CliErrorKind;
use super::scena_cli_error::CliFailure;
use super::scena_output::{CliOutcome, json_success};

pub(crate) fn run_materials_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let [subcommand, rest @ ..] = args else {
        return Err(CliFailure::invalid_arguments(
            "usage: scena materials list [--category metal|plastic|fabric|leather|rubber] [--query <text>]",
        ));
    };
    match subcommand.as_str() {
        "list" => list_materials(rest),
        "fetch" => fetch_material(rest),
        "import" => import_material(rest),
        other => Err(CliFailure::invalid_arguments(format!(
            "unknown materials command '{other}'; expected 'list', 'fetch', or 'import'"
        ))),
    }
}

#[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
fn fetch_material(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let Some(id) = args.first() else {
        return Err(CliFailure::invalid_arguments(
            "usage: scena materials fetch <id> [--resolution 1k|2k|4k] [--out <dir>] [--expect-sha256 <hex>]",
        ));
    };
    let entry = resolve_entry(id)?;
    let options = parse_pack_options(&args[1..], &entry)?;
    if let Some(pack) = validated_cached_pack(&entry, &options)? {
        return json_success(
            &pack,
            "failed to serialize cached photographic material pack",
        );
    }
    let resolution = options
        .resolution
        .unwrap_or(scena::PhotographicMaterialResolutionV1::OneK);
    let archive_uri = entry
        .archive_variant(resolution)
        .map(|variant| variant.archive_uri.as_str())
        .ok_or_else(|| {
            CliFailure::new(
                CliErrorKind::InvalidInput,
                format!(
                    "material '{}' has no {} archive",
                    entry.id,
                    resolution.as_str()
                ),
            )
        })?;
    let mut response = ureq::get(archive_uri)
        .header(
            "User-Agent",
            concat!("scena/", env!("CARGO_PKG_VERSION"), " material-library"),
        )
        .call()
        .map_err(|error| {
            CliFailure::new(
                CliErrorKind::Runtime,
                format!(
                    "failed to download material '{}' from '{}': {error}",
                    entry.id, archive_uri
                ),
            )
        })?;
    let archive = response
        .body_mut()
        .with_config()
        .limit(scena::PHOTOGRAPHIC_MATERIAL_ARCHIVE_MAX_BYTES as u64 + 1)
        .read_to_vec()
        .map_err(|error| {
            CliFailure::new(
                CliErrorKind::Runtime,
                format!("failed to read downloaded material '{}': {error}", entry.id),
            )
        })?;
    compile_pack(entry, archive, options)
}

#[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
fn validated_cached_pack(
    entry: &scena::PhotographicMaterialCatalogEntryV2,
    options: &PackOptions,
) -> Result<Option<serde_json::Value>, CliFailure> {
    if !options.output_dir.exists() {
        return Ok(None);
    }
    let manifest_path = options.output_dir.join("scena-material-pack.json");
    if !manifest_path.is_file() {
        return Err(CliFailure::new(
            CliErrorKind::InvalidInput,
            format!(
                "material output '{}' already exists without a scena material-pack manifest; choose another --out directory",
                options.output_dir.display()
            ),
        ));
    }
    let assets = scena::Assets::new();
    let loaded =
        pollster::block_on(assets.load_photographic_material_pack(manifest_path.as_path()))
            .map_err(|error| {
                CliFailure::new(
                    CliErrorKind::InvalidInput,
                    format!(
                        "cached material pack '{}' failed validation: {error}",
                        manifest_path.display()
                    ),
                )
            })?;
    let pack = loaded.pack().clone();
    if let Some(resolution) = options.resolution
        && loaded.resolution() != resolution
    {
        return Err(CliFailure::new(
            CliErrorKind::InvalidInput,
            format!(
                "cached material pack '{}' is {}, not requested {}",
                manifest_path.display(),
                loaded.resolution().as_str(),
                resolution.as_str()
            ),
        ));
    }
    if pack.id != entry.id
        || pack.category != entry.category
        || pack.surface_kind != entry.surface_kind
        || pack.source.provider != entry.provider
        || pack.source.provider_asset_id != entry.provider_asset_id
        || pack.source.license != entry.license
    {
        return Err(CliFailure::new(
            CliErrorKind::InvalidInput,
            format!(
                "cached material pack '{}' does not match catalog entry '{}'; choose another --out directory",
                manifest_path.display(),
                entry.id
            ),
        ));
    }
    if let Some(expected) = options.expected_sha256.as_deref()
        && expected != pack.source.archive_sha256
    {
        return Err(CliFailure::new(
            CliErrorKind::InvalidInput,
            format!(
                "cached material '{}' archive SHA-256 {} does not match --expect-sha256 {expected}",
                entry.id, pack.source.archive_sha256
            ),
        ));
    }
    let manifest = std::fs::read(&manifest_path).map_err(|error| {
        CliFailure::new(
            CliErrorKind::Io,
            format!(
                "failed to read validated cached material pack '{}': {error}",
                manifest_path.display()
            ),
        )
    })?;
    let manifest = serde_json::from_slice(&manifest).map_err(|error| {
        CliFailure::new(
            CliErrorKind::InvalidInput,
            format!(
                "validated cached material pack '{}' is invalid JSON: {error}",
                manifest_path.display()
            ),
        )
    })?;
    Ok(Some(manifest))
}

#[cfg(not(all(feature = "material-library", not(target_arch = "wasm32"))))]
fn fetch_material(_args: &[String]) -> Result<CliOutcome, CliFailure> {
    Err(CliFailure::feature_unavailable(
        "materials fetch requires a native build with the material-library feature",
    ))
}

#[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
fn import_material(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let [id, archive_path, rest @ ..] = args else {
        return Err(CliFailure::invalid_arguments(
            "usage: scena materials import <id> <archive.zip> [--resolution 1k|2k|4k] [--out <dir>] [--expect-sha256 <hex>]",
        ));
    };
    let entry = resolve_entry(id)?;
    let options = parse_pack_options(rest, &entry)?;
    let archive_path = Path::new(archive_path);
    let archive = std::fs::read(archive_path).map_err(|error| {
        CliFailure::new(
            CliErrorKind::Io,
            format!(
                "failed to read material archive '{}': {error}",
                archive_path.display()
            ),
        )
    })?;
    compile_pack(entry, archive, options)
}

#[cfg(not(all(feature = "material-library", not(target_arch = "wasm32"))))]
fn import_material(_args: &[String]) -> Result<CliOutcome, CliFailure> {
    Err(CliFailure::feature_unavailable(
        "materials import requires a native build with the material-library feature",
    ))
}

#[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
struct PackOptions {
    output_dir: PathBuf,
    expected_sha256: Option<String>,
    resolution: Option<scena::PhotographicMaterialResolutionV1>,
}

#[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
fn parse_pack_options(
    args: &[String],
    entry: &scena::PhotographicMaterialCatalogEntryV2,
) -> Result<PackOptions, CliFailure> {
    let mut output_dir = None;
    let mut expected_sha256 = None;
    let mut resolution = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--out" => {
                let value = args.get(index + 1).ok_or_else(|| {
                    CliFailure::invalid_arguments("--out requires a material pack directory")
                })?;
                output_dir = Some(PathBuf::from(value));
                index += 2;
            }
            "--expect-sha256" => {
                let value = args.get(index + 1).ok_or_else(|| {
                    CliFailure::invalid_arguments("--expect-sha256 requires a 64-digit hex hash")
                })?;
                if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return Err(CliFailure::invalid_arguments(
                        "--expect-sha256 requires exactly 64 hexadecimal digits",
                    ));
                }
                expected_sha256 = Some(value.to_ascii_lowercase());
                index += 2;
            }
            "--resolution" => {
                let value = args.get(index + 1).ok_or_else(|| {
                    CliFailure::invalid_arguments("--resolution requires 1k, 2k, or 4k")
                })?;
                resolution = Some(
                    scena::PhotographicMaterialResolutionV1::from_name(value).ok_or_else(|| {
                        CliFailure::invalid_arguments(format!(
                            "unknown material resolution '{value}'; expected 1k, 2k, or 4k"
                        ))
                    })?,
                );
                index += 2;
            }
            other => {
                return Err(CliFailure::invalid_arguments(format!(
                    "unknown material pack option '{other}'"
                )));
            }
        }
    }
    Ok(PackOptions {
        output_dir: match output_dir {
            Some(path) => path,
            None => default_cache_path(entry, resolution)?,
        },
        expected_sha256,
        resolution,
    })
}

#[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
fn default_cache_path(
    entry: &scena::PhotographicMaterialCatalogEntryV2,
    resolution: Option<scena::PhotographicMaterialResolutionV1>,
) -> Result<PathBuf, CliFailure> {
    let with_resolution = |path: PathBuf| match resolution {
        Some(resolution) => path.join(resolution.as_str()),
        None => path,
    };
    if let Some(root) = std::env::var_os("XDG_CACHE_HOME").filter(|value| !value.is_empty()) {
        return Ok(with_resolution(
            PathBuf::from(root)
                .join("scena")
                .join("materials")
                .join(&entry.id),
        ));
    }
    let Some(home) = std::env::var_os("HOME").filter(|value| !value.is_empty()) else {
        return Err(CliFailure::invalid_arguments(
            "material cache location is unavailable; pass --out <dir>",
        ));
    };
    Ok(with_resolution(
        PathBuf::from(home)
            .join(".cache")
            .join("scena")
            .join("materials")
            .join(&entry.id),
    ))
}

#[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
fn resolve_entry(id: &str) -> Result<scena::PhotographicMaterialCatalogEntryV2, CliFailure> {
    let normalized = id.trim().to_ascii_lowercase();
    let catalog = scena::photographic_material_catalog_v2();
    catalog
        .entries
        .into_iter()
        .find(|entry| {
            entry.id.eq_ignore_ascii_case(&normalized)
                || entry.provider_asset_id.eq_ignore_ascii_case(&normalized)
        })
        .ok_or_else(|| {
            let candidates = scena::nearest_name_candidates(
                &normalized,
                scena::photographic_material_catalog_v2()
                    .entries
                    .iter()
                    .flat_map(|entry| [entry.id.as_str(), entry.provider_asset_id.as_str()]),
                5,
            );
            CliFailure::new(
                CliErrorKind::InvalidInput,
                format!(
                    "unknown photographic material '{id}'; run 'scena materials list'{}",
                    if candidates.is_empty() {
                        String::new()
                    } else {
                        format!("; nearest: {}", candidates.join(", "))
                    }
                ),
            )
        })
}

#[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
fn compile_pack(
    entry: scena::PhotographicMaterialCatalogEntryV2,
    archive: Vec<u8>,
    options: PackOptions,
) -> Result<CliOutcome, CliFailure> {
    let observed_sha256 = sha256_hex(&archive);
    if let Some(expected) = &options.expected_sha256
        && expected != &observed_sha256
    {
        return Err(CliFailure::new(
            CliErrorKind::InvalidInput,
            format!(
                "material archive SHA-256 mismatch for '{}': expected {expected}, observed {observed_sha256}",
                entry.id
            ),
        ));
    }
    if let Some(resolution) = options.resolution {
        let pack = scena::compile_photographic_material_archive_at_resolution(
            &entry,
            resolution,
            &archive,
            &options.output_dir,
        )
        .map_err(|error| {
            CliFailure::new(
                CliErrorKind::InvalidInput,
                format!(
                    "failed to compile {} material '{}' into '{}': {error}",
                    resolution.as_str(),
                    entry.id,
                    options.output_dir.display()
                ),
            )
        })?;
        json_success(
            &pack,
            "failed to serialize resolution-aware photographic material pack",
        )
    } else {
        let entry = entry
            .for_resolution(scena::PhotographicMaterialResolutionV1::OneK)
            .expect("v2 catalog always exposes its 1K compatibility archive");
        let pack =
            scena::compile_photographic_material_archive(&entry, &archive, &options.output_dir)
                .map_err(|error| {
                    CliFailure::new(
                        CliErrorKind::InvalidInput,
                        format!(
                            "failed to compile material '{}' into '{}': {error}",
                            entry.id,
                            options.output_dir.display()
                        ),
                    )
                })?;
        json_success(&pack, "failed to serialize photographic material pack")
    }
}

#[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    encoded
}

fn list_materials(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let mut category = None;
    let mut query = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--category" => {
                let value = args.get(index + 1).ok_or_else(|| {
                    CliFailure::invalid_arguments("--category requires a material category")
                })?;
                category = Some(
                    scena::PhotographicMaterialCategoryV1::from_name(value).ok_or_else(|| {
                        CliFailure::invalid_arguments(format!(
                            "unknown material category '{value}'; expected metal, plastic, fabric, leather, or rubber"
                        ))
                    })?,
                );
                index += 2;
            }
            "--query" => {
                let value = args.get(index + 1).ok_or_else(|| {
                    CliFailure::invalid_arguments("--query requires non-empty text")
                })?;
                let normalized = value.trim().to_ascii_lowercase();
                if normalized.is_empty() {
                    return Err(CliFailure::invalid_arguments(
                        "--query requires non-empty text",
                    ));
                }
                query = Some(normalized);
                index += 2;
            }
            other => {
                return Err(CliFailure::invalid_arguments(format!(
                    "unknown materials list option '{other}'"
                )));
            }
        }
    }

    let mut catalog = scena::photographic_material_catalog_v1();
    catalog.entries.retain(|entry| {
        category.is_none_or(|required| entry.category == required)
            && query.as_ref().is_none_or(|query| {
                entry.id.to_ascii_lowercase().contains(query)
                    || entry.label.to_ascii_lowercase().contains(query)
                    || entry.provider_asset_id.to_ascii_lowercase().contains(query)
                    || entry.tags.iter().any(|tag| tag.contains(query))
            })
    });
    json_success(
        &catalog,
        "failed to serialize photographic material catalog",
    )
}
