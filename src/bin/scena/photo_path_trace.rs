//! `scena photo render --path-trace`: delivers the photo's final frame from
//! the path tracer instead of the rasterizer.
//!
//! The camera-behavior loop still runs on the rasterizer, which is fast
//! enough to try several framings and exposures. Once it has settled on a
//! camera, exposure, focus and lighting, the delivered pixels are traced from
//! that same scene, so the quality policy judges the image that is written.

use serde_json::{Value, json};

use super::scena_cli_error::{CliErrorKind, CliFailure, CliUsageError};

pub(crate) const DEFAULT_SAMPLES: u32 = 128;
pub(crate) const DEFAULT_BOUNCES: u32 = 8;

/// Path-tracing choices given on the command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct PhotoPathTraceFlags {
    pub(crate) enabled: bool,
    pub(crate) samples: Option<u32>,
    pub(crate) bounces: Option<u32>,
    pub(crate) no_denoise: bool,
}

/// Settings of a requested path-traced delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PhotoPathTraceArgs {
    pub(crate) samples: u32,
    pub(crate) bounces: u32,
    pub(crate) denoise: bool,
}

impl PhotoPathTraceFlags {
    /// The delivery these flags ask for, or `None` for a rasterized photo.
    /// The tuning flags mean nothing without `--path-trace`, and saying so
    /// beats silently rasterizing.
    pub(crate) fn resolve(self) -> Result<Option<PhotoPathTraceArgs>, CliUsageError> {
        if !self.enabled {
            let stray = [
                (self.samples.is_some(), "--samples"),
                (self.bounces.is_some(), "--bounces"),
                (self.no_denoise, "--no-denoise"),
            ];
            return match stray.iter().find(|(given, _)| *given) {
                Some((_, flag)) => {
                    Err(CliUsageError::from(format!("{flag} requires --path-trace")))
                }
                None => Ok(None),
            };
        }
        Ok(Some(PhotoPathTraceArgs {
            samples: self.samples.unwrap_or(DEFAULT_SAMPLES),
            bounces: self.bounces.unwrap_or(DEFAULT_BOUNCES),
            denoise: !self.no_denoise,
        }))
    }
}

pub(crate) fn parse_count(flag: &str, value: &str, limit: u32) -> Result<u32, CliUsageError> {
    match value.parse::<u32>() {
        Ok(count) if (1..=limit).contains(&count) => Ok(count),
        _ => Err(CliUsageError::from(format!(
            "{flag} requires an integer from 1 to {limit}, got '{value}'"
        ))),
    }
}

/// Exposure corrections tried on a traced frame before it is delivered. The
/// frame is developed again from its radiance each time, not traced again.
pub(crate) const EXPOSURE_MAX_ATTEMPTS: usize = 4;

/// The traced frame and the account of it that goes into the report.
pub(crate) struct PathTracedDelivery {
    pub(crate) image: scena::PathTracedImage,
    pub(crate) report: Value,
}

/// Traces the host's scene as it stands, through its active camera.
///
/// `stand_in_shadows` are the blended decals the rasterized photo places
/// under a subject to ground it. Traced light casts that shadow itself, so
/// the trace leaves them out.
pub(crate) fn deliver<F: scena::AssetFetcher>(
    host: &scena::SceneHostCore<F>,
    stand_in_shadows: &[u64],
    args: PhotoPathTraceArgs,
) -> Result<PathTracedDelivery, CliFailure> {
    let failure =
        |error: &dyn std::fmt::Display| CliFailure::new(CliErrorKind::Runtime, error.to_string());
    let options = scena::PathTraceOptions::new()
        .with_samples_per_pixel(args.samples)
        .with_max_bounces(args.bounces)
        .with_denoise(args.denoise);
    let started = std::time::Instant::now();
    let traced = host.path_trace(&options, stand_in_shadows);
    let elapsed = started.elapsed();
    let image = traced.map_err(|error| failure(&error))?;
    let stats = image.stats();
    let report = json!({
        "samples_per_pixel": stats.samples_per_pixel,
        "max_bounces": stats.max_bounces,
        "denoised": stats.denoised,
        "camera_paths": stats.camera_paths,
        "triangles": stats.triangles,
        "emissive_triangles": stats.emissive_triangles,
        "lights": stats.lights,
        "materials": stats.materials,
        "textures": stats.textures,
        "environment_resolution": stats.environment_resolution,
        "hidden_stand_in_shadow_nodes": stand_in_shadows.len(),
        "skipped": {
            "stroke_nodes": stats.skipped_stroke_nodes,
            "label_nodes": stats.skipped_label_nodes,
            "particle_nodes": stats.skipped_particle_nodes,
            "degenerate_triangles": stats.skipped_degenerate_triangles,
        },
        "elapsed_ms": elapsed.as_millis() as u64,
    });
    Ok(PathTracedDelivery { image, report })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuning_flags_without_path_trace_are_refused() {
        assert_eq!(PhotoPathTraceFlags::default().resolve(), Ok(None));
        let stray = PhotoPathTraceFlags {
            samples: Some(64),
            ..PhotoPathTraceFlags::default()
        };
        let error = stray
            .resolve()
            .expect_err("--samples alone is a usage error");
        assert!(
            error
                .to_string()
                .contains("--samples requires --path-trace"),
            "{error}"
        );
    }

    #[test]
    fn path_trace_defaults_and_overrides_resolve() {
        let plain = PhotoPathTraceFlags {
            enabled: true,
            ..PhotoPathTraceFlags::default()
        };
        assert_eq!(
            plain.resolve(),
            Ok(Some(PhotoPathTraceArgs {
                samples: DEFAULT_SAMPLES,
                bounces: DEFAULT_BOUNCES,
                denoise: true,
            }))
        );
        let tuned = PhotoPathTraceFlags {
            enabled: true,
            samples: Some(512),
            bounces: Some(4),
            no_denoise: true,
        };
        assert_eq!(
            tuned.resolve(),
            Ok(Some(PhotoPathTraceArgs {
                samples: 512,
                bounces: 4,
                denoise: false,
            }))
        );
    }

    #[test]
    fn counts_outside_their_range_are_refused() {
        assert_eq!(parse_count("--samples", "256", 65_536), Ok(256));
        for bad in ["0", "-3", "many", "70000"] {
            assert!(
                parse_count("--samples", bad, 65_536).is_err(),
                "{bad} must be refused"
            );
        }
    }
}
