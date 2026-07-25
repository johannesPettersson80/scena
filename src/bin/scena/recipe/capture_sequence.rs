use crate::scena_cli_error::{CliErrorKind, CliFailure};
use std::f32::consts::TAU;
use std::path::PathBuf;

use serde_json::{Value, json};

use crate::scena_input::{RecipeReadError, read_recipe_text};
use crate::scena_output::{
    CliBackendSelectionV1, CliOutcome, json_outcome, json_outcome_with_backend_selection,
};

#[path = "capture_sequence/animation.rs"]
mod animation;
#[path = "capture_sequence/output.rs"]
mod output;
#[path = "capture_sequence/view.rs"]
pub(super) mod view;

use output::{capture_frame, ensure_output_dir, path_json, write_contact_sheet};
use view::CanonicalView;

const CAPTURE_SEQUENCE_SCHEMA_V1: &str = "scena.capture_sequence_result.v1";
const MAX_SEQUENCE_FRAMES: usize = 360;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CaptureSequenceArgs {
    recipe: PathBuf,
    out_dir: PathBuf,
    views: Vec<CanonicalView>,
    turntable: Option<usize>,
    clip: Option<String>,
    clip_frames: Option<usize>,
    gpu: bool,
    max_imports: Option<usize>,
}

pub(crate) fn run_recipe_capture_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let args = CaptureSequenceArgs::parse(args)?;
    let out_dir = ensure_output_dir(&args.out_dir)?;
    let mut policy = scena::RecipeBuildPolicy::testing();
    if let Some(max_imports) = args.max_imports {
        policy = policy.with_max_imports(max_imports);
    }
    let recipe_text = match read_recipe_text(&args.recipe, &policy) {
        Ok(text) => text,
        Err(RecipeReadError::TooLarge(report)) => {
            return json_outcome(
                &report,
                1,
                "failed to serialize scene recipe validation report",
            );
        }
        Err(RecipeReadError::Io(error)) => {
            return Err(CliFailure::new(
                CliErrorKind::InputNotFound,
                format!("failed to read recipe '{}': {error}", args.recipe.display()),
            ));
        }
    };
    let recipe_path = args.recipe.display().to_string();
    let build = if args.gpu {
        pollster::block_on(scena::SceneHostCore::build_recipe_json_gpu(
            &recipe_path,
            &recipe_text,
            policy,
        ))
    } else {
        pollster::block_on(scena::SceneHostCore::build_recipe_json(
            &recipe_path,
            &recipe_text,
            policy,
        ))
    };
    let build = match build {
        Ok(build) => build,
        Err(manifest) => {
            let result = scena::SceneRecipeRenderResultV1::build_failed(manifest);
            return json_outcome_with_backend_selection(
                &result,
                1,
                "failed to serialize recipe capture failure",
                CliBackendSelectionV1::new(args.gpu, None),
            );
        }
    };
    let manifest = build.manifest;
    let mut host = build.host;
    host.frame_all_with_overlays()
        .map_err(|error| format!("failed to frame capture subject: {error}"))?;
    let inspection: scena::SceneInspectionReportV1 = serde_json::from_str(
        &host
            .inspect_json()
            .map_err(|error| format!("failed to inspect capture subject: {error}"))?,
    )
    .map_err(|error| format!("failed to decode capture inspection: {error}"))?;
    let bounds = view::subject_bounds(&inspection)?;
    let base_camera = host.get_camera();

    let mut captures = Vec::new();
    let mut frames = Vec::new();
    for canonical in &args.views {
        let camera = canonical.camera_state(base_camera.target, base_camera.distance);
        push_frame(
            &mut host,
            &out_dir,
            &mut captures,
            &mut frames,
            "canonical_view",
            canonical.id(),
            camera,
            json!({
                "canonical_view": {
                    "id": canonical.id(),
                    "purpose": canonical.purpose(),
                }
            }),
        )?;
    }

    if let Some(sample_count) = args.turntable {
        for sample_index in 0..sample_count {
            let yaw_radians = TAU * sample_index as f32 / sample_count as f32;
            let camera = scena::SceneHostCameraState {
                target: base_camera.target,
                distance: base_camera.distance,
                yaw_radians,
                pitch_radians: 20.0_f32.to_radians(),
            };
            let label = format!("turntable-{sample_index:03}");
            push_frame(
                &mut host,
                &out_dir,
                &mut captures,
                &mut frames,
                "turntable",
                &label,
                camera,
                json!({
                    "turntable": {
                        "sample_index": sample_index,
                        "sample_count": sample_count,
                        "yaw_radians": yaw_radians,
                        "pitch_degrees": 20.0,
                    }
                }),
            )?;
        }
    }

    if let (Some(clip_name), Some(sample_count)) = (&args.clip, args.clip_frames) {
        let (handle, duration_seconds) = animation::resolve_clip(&mut host, &manifest, clip_name)?;
        let camera =
            CanonicalView::Isometric.camera_state(base_camera.target, base_camera.distance);
        for sample_index in 0..sample_count {
            let time_seconds = if sample_count == 1 {
                0.0
            } else {
                duration_seconds * sample_index as f32 / (sample_count - 1) as f32
            };
            host.seek_animation(handle, f64::from(time_seconds))
                .map_err(|error| format!("failed to seek clip '{clip_name}': {error}"))?;
            let label = format!("clip-{clip_name}-{sample_index:03}");
            push_frame(
                &mut host,
                &out_dir,
                &mut captures,
                &mut frames,
                "clip",
                &label,
                camera,
                json!({
                    "clip": {
                        "name": clip_name,
                        "sample_index": sample_index,
                        "sample_count": sample_count,
                        "time_seconds": time_seconds,
                        "duration_seconds": duration_seconds,
                    }
                }),
            )?;
        }
    }

    let contact_sheet = write_contact_sheet(&out_dir, &captures, &frames)?;
    let canonical_view_order = args.views.iter().map(|view| view.id()).collect::<Vec<_>>();
    let report = json!({
        "schema": CAPTURE_SEQUENCE_SCHEMA_V1,
        "ok": true,
        "source_recipe": path_json(&args.recipe),
        "output_dir": path_json(&args.out_dir),
        "coordinate_convention": {
            "handedness": "right_handed",
            "world_up": "+Y",
            "front_look_direction": "-Z",
            "right_look_direction": "-X",
            "top_look_direction": "near_-Y",
            "top_screen_up": "-Z",
            "top_pole_offset_degrees": 1.0,
            "isometric_eye_octant": "+X,+Y,+Z",
        },
        "canonical_view_order": canonical_view_order,
        "subject_bounds": {
            "min": vec3_json(bounds.min),
            "max": vec3_json(bounds.max),
            "extent": vec3_json(bounds.extent()),
            "center": vec3_json(bounds.center()),
            "radius": bounds.radius(),
        },
        "sequence_encoding": "png_frames_and_contact_sheet",
        "video_encoding": {
            "status": "not_requested",
            "reason": "core capture emits deterministic PNG frames; hosts may encode GIF/video externally without changing frame semantics",
        },
        "frames": frames,
        "contact_sheet": contact_sheet,
    });
    json_outcome_with_backend_selection(
        &report,
        0,
        "failed to serialize capture sequence result",
        CliBackendSelectionV1::new(args.gpu, Some(host.backend())),
    )
}

// Keep capture identity and sequence metadata explicit at the one frame-write
// boundary; a positional helper struct would only move these arguments.
#[allow(clippy::too_many_arguments)]
fn push_frame(
    host: &mut scena::SceneHostCore,
    out_dir: &std::path::Path,
    captures: &mut Vec<scena::CaptureRgba8>,
    frames: &mut Vec<Value>,
    kind: &str,
    label: &str,
    camera: scena::SceneHostCameraState,
    sequence: Value,
) -> Result<(), String> {
    let frame = capture_frame(host, out_dir, frames.len(), kind, label, camera, sequence)?;
    captures.push(frame.contact_sheet_capture);
    frames.push(frame.report);
    Ok(())
}

impl CaptureSequenceArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let Some(recipe) = args.first() else {
            return Err(capture_usage());
        };
        let mut out_dir = None;
        let mut views = None;
        let mut turntable = None;
        let mut clip = None;
        let mut clip_frames = None;
        let mut gpu = false;
        let mut max_imports = None;
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--out-dir" => {
                    out_dir = Some(PathBuf::from(flag_value(args, index, "--out-dir")?));
                    index += 2;
                }
                "--views" => {
                    views = Some(parse_views(&flag_value(args, index, "--views")?)?);
                    index += 2;
                }
                "--turntable" => {
                    turntable = Some(parse_frame_count(
                        "--turntable",
                        &flag_value(args, index, "--turntable")?,
                    )?);
                    index += 2;
                }
                "--clip" => {
                    clip = Some(flag_value(args, index, "--clip")?);
                    index += 2;
                }
                "--frames" => {
                    clip_frames = Some(parse_frame_count(
                        "--frames",
                        &flag_value(args, index, "--frames")?,
                    )?);
                    index += 2;
                }
                "--gpu" => {
                    gpu = true;
                    index += 1;
                }
                "--max-imports" => {
                    max_imports = Some(parse_positive_usize(
                        "--max-imports",
                        &flag_value(args, index, "--max-imports")?,
                    )?);
                    index += 2;
                }
                "--json" => index += 1,
                flag => {
                    return Err(format!(
                        "unknown recipe capture flag '{flag}'; {}",
                        capture_usage()
                    ));
                }
            }
        }
        if clip.is_some() != clip_frames.is_some() {
            return Err("--clip <name> and --frames <n> must be provided together".to_owned());
        }
        let views = views.unwrap_or_else(|| {
            vec![
                CanonicalView::Front,
                CanonicalView::Top,
                CanonicalView::Right,
                CanonicalView::Isometric,
            ]
        });
        if views.is_empty() && turntable.is_none() && clip.is_none() {
            return Err(
                "capture requires at least one view, turntable, or clip sequence".to_owned(),
            );
        }
        let total_frames = views.len() + turntable.unwrap_or(0) + clip_frames.unwrap_or(0);
        if total_frames > MAX_SEQUENCE_FRAMES {
            return Err(format!(
                "capture total frame count must be between 1 and {MAX_SEQUENCE_FRAMES}, got {total_frames}"
            ));
        }
        Ok(Self {
            recipe: PathBuf::from(recipe),
            out_dir: out_dir
                .ok_or_else(|| format!("missing --out-dir <dir>; {}", capture_usage()))?,
            views,
            turntable,
            clip,
            clip_frames,
            gpu,
            max_imports,
        })
    }
}

fn parse_views(value: &str) -> Result<Vec<CanonicalView>, String> {
    if value == "none" {
        return Ok(Vec::new());
    }
    let views = value
        .split(',')
        .map(str::trim)
        .map(CanonicalView::parse)
        .collect::<Result<Vec<_>, _>>()?;
    if views.is_empty() {
        return Err("--views requires a comma-separated view list or 'none'".to_owned());
    }
    Ok(views)
}

fn parse_frame_count(flag: &str, value: &str) -> Result<usize, String> {
    let count = value
        .parse::<usize>()
        .map_err(|_| format!("{flag} requires an integer, got '{value}'"))?;
    if count == 0 || count > MAX_SEQUENCE_FRAMES {
        return Err(format!(
            "{flag} must be between 1 and {MAX_SEQUENCE_FRAMES}, got {count}"
        ));
    }
    Ok(count)
}

fn parse_positive_usize(flag: &str, value: &str) -> Result<usize, String> {
    let count = value
        .parse::<usize>()
        .map_err(|_| format!("{flag} requires a positive integer, got '{value}'"))?;
    if count == 0 {
        return Err(format!("{flag} must be greater than zero"));
    }
    Ok(count)
}

fn flag_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn vec3_json(value: scena::Vec3) -> Value {
    json!([value.x, value.y, value.z])
}

fn capture_usage() -> String {
    "usage: scena recipe capture <recipe.json> --out-dir <dir> [--views front,top,right,isometric|none] [--turntable <frames>] [--clip <name> --frames <n>] [--gpu] [--max-imports <n>]"
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::CaptureSequenceArgs;

    #[test]
    fn capture_sequence_rejects_a_combined_frame_budget_above_the_limit() {
        let args = [
            "scene.json".to_owned(),
            "--out-dir".to_owned(),
            "out".to_owned(),
            "--turntable".to_owned(),
            "360".to_owned(),
        ];
        let error = CaptureSequenceArgs::parse(&args).expect_err("364 frames must be rejected");
        assert!(error.contains("total frame count"), "{error}");
    }
}
