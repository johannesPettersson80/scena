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
    pub(crate) no_surface_detail: bool,
}

/// Settings of a requested path-traced delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PhotoPathTraceArgs {
    pub(crate) samples: u32,
    pub(crate) bounces: u32,
    pub(crate) denoise: bool,
    pub(crate) surface_detail: bool,
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
                (self.no_surface_detail, "--no-surface-detail"),
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
            surface_detail: !self.no_surface_detail,
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
pub(crate) const EXPOSURE_MAX_ATTEMPTS: usize = 6;

/// The traced frame and the account of it that goes into the report.
pub(crate) struct PathTracedDelivery {
    pub(crate) image: scena::PathTracedImage,
    pub(crate) report: Value,
}

/// Share of the subject's radius to which the edges of its parts are
/// rounded where the model states no finish of its own: 0.6 mm on a subject
/// 30 cm across, about the break a machinist leaves on an edge.
const EDGE_RADIUS_SHARE: f32 = 0.004;

/// Edge radius, in metres, for a subject with these world bounds.
pub(crate) fn subject_edge_radius_m(bounds: Option<scena::Aabb>) -> f32 {
    bounds.map_or(0.0, |bounds| {
        let radius = bounds.bounding_sphere_radius() * EDGE_RADIUS_SHARE;
        if radius.is_finite() {
            radius.clamp(0.0, scena::PathTraceOptions::MAX_EDGE_RADIUS_M)
        } else {
            0.0
        }
    })
}

/// Traces the host's scene as it stands, through its active camera.
///
/// `stand_in_shadows` are the blended decals the rasterized photo places
/// under a subject to ground it. Traced light casts that shadow itself, so
/// the trace leaves them out. `edge_radius_m` rounds the edges of parts
/// whose material has no finish.
pub(crate) fn deliver<F: scena::AssetFetcher>(
    host: &scena::SceneHostCore<F>,
    stand_in_shadows: &[u64],
    args: PhotoPathTraceArgs,
    edge_radius_m: f32,
) -> Result<PathTracedDelivery, CliFailure> {
    let failure =
        |error: &dyn std::fmt::Display| CliFailure::new(CliErrorKind::Runtime, error.to_string());
    let options = scena::PathTraceOptions::new()
        .with_samples_per_pixel(args.samples)
        .with_max_bounces(args.bounces)
        .with_denoise(args.denoise)
        .with_surface_detail(args.surface_detail)
        .with_edge_radius_m(edge_radius_m);
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
        "surface_detail": {
            "enabled": args.surface_detail,
            "edge_radius_m": if args.surface_detail { edge_radius_m } else { 0.0 },
            "finished_surfaces": stats.finished_surfaces,
            "rounded_surfaces": stats.rounded_surfaces,
        },
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
                surface_detail: true,
            }))
        );
        let tuned = PhotoPathTraceFlags {
            enabled: true,
            samples: Some(512),
            bounces: Some(4),
            no_denoise: true,
            no_surface_detail: true,
        };
        assert_eq!(
            tuned.resolve(),
            Ok(Some(PhotoPathTraceArgs {
                samples: 512,
                bounces: 4,
                denoise: false,
                surface_detail: false,
            }))
        );
        let stray = PhotoPathTraceFlags {
            no_surface_detail: true,
            ..PhotoPathTraceFlags::default()
        };
        let error = stray
            .resolve()
            .expect_err("--no-surface-detail alone is a usage error");
        assert!(
            error
                .to_string()
                .contains("--no-surface-detail requires --path-trace"),
            "{error}"
        );
    }

    #[test]
    fn edges_are_rounded_to_a_share_of_the_subject() {
        assert_eq!(subject_edge_radius_m(None), 0.0);
        let cube = scena::Aabb {
            min: scena::Vec3::splat(-0.15),
            max: scena::Vec3::splat(0.15),
        };
        let radius = subject_edge_radius_m(Some(cube));
        // The bounding sphere of a 30 cm cube has a radius of about 26 cm.
        assert!((radius - 0.001_04).abs() < 2.0e-5, "{radius}");
        let endless = scena::Aabb {
            min: scena::Vec3::splat(f32::NEG_INFINITY),
            max: scena::Vec3::splat(f32::INFINITY),
        };
        assert_eq!(subject_edge_radius_m(Some(endless)), 0.0);
        let vast = scena::Aabb {
            min: scena::Vec3::splat(-1.0e6),
            max: scena::Vec3::splat(1.0e6),
        };
        assert_eq!(
            subject_edge_radius_m(Some(vast)),
            scena::PathTraceOptions::MAX_EDGE_RADIUS_M
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
