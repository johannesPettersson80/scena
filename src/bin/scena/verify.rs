use std::fs;

use super::scena_input::{
    appearance_introspection_options, ensure_parent_dir, resolve_scene_input, viewer_builder,
};
use super::scena_output::{CliOutcome, json_outcome};

#[cfg(feature = "scene-host")]
use super::scena_input::scene_host_build_from_resolved_recipe;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifyAppearanceCommandArgs {
    input: String,
    expect: std::path::PathBuf,
    out: Option<std::path::PathBuf>,
    width: Option<u32>,
    height: Option<u32>,
    detail: bool,
}

pub(crate) fn run_verify_appearance_command(args: &[String]) -> Result<CliOutcome, String> {
    let args = VerifyAppearanceCommandArgs::parse(args)?;
    let text = fs::read_to_string(&args.expect).map_err(|error| {
        format!(
            "failed to read appearance expectation '{}': {error}",
            args.expect.display()
        )
    })?;
    let expectation: scena::AppearanceExpectationV1 =
        serde_json::from_str(&text).map_err(|error| {
            format!(
                "failed to parse appearance expectation '{}': {error}",
                args.expect.display()
            )
        })?;
    expectation.validate_schema()?;

    let input = match resolve_scene_input(&args.input) {
        Ok(input) => input,
        Err(outcome) => return Ok(outcome),
    };
    let width = args.width.or(input.width).unwrap_or(800);
    let height = args.height.or(input.height).unwrap_or(600);
    if input.has_scene_host_directives() {
        return run_verify_recipe_appearance(
            input,
            width,
            height,
            expectation,
            args.out.as_deref(),
            args.detail,
        );
    }
    let mut viewer = pollster::block_on(
        viewer_builder(input.asset.as_str(), width, height, input.transform, false)
            .with_default_light()
            .build(),
    )
    .map_err(|error| format!("failed to verify appearance for '{}': {error}", input.asset))?;

    if let Some(variant) = expectation.first_requested_variant()
        && viewer
            .material_variants()
            .iter()
            .any(|candidate| candidate == variant)
    {
        viewer
            .set_active_material_variant(Some(variant))
            .map_err(|error| format!("failed to apply material variant '{variant}': {error}"))?;
    }

    viewer
        .render_next_frame()
        .map_err(|error| format!("failed to render '{}': {error}", input.asset))?;
    let capture = viewer
        .capture()
        .map_err(|error| format!("failed to capture '{}': {error}", input.asset))?;

    if let Some(out) = args.out.as_ref() {
        ensure_parent_dir(out)?;
        capture
            .write_png(out)
            .map_err(|error| format!("failed to write PNG '{}': {error}", out.display()))?;
    }

    let inspection = viewer
        .scene()
        .inspect_with_assets(viewer.assets())
        .to_schema_report();
    let options = appearance_introspection_options(args.detail)
        .with_active_material_variant(viewer.active_material_variant())
        .with_available_material_variants(viewer.material_variants().to_vec());
    let report =
        viewer
            .renderer()
            .introspect_appearance(&capture, &inspection, &expectation, options);
    let exit_code = if report.ok { 0 } else { 1 };
    json_outcome(
        &report,
        exit_code,
        "failed to serialize appearance introspection report",
    )
}

#[cfg(feature = "scene-host")]
fn run_verify_recipe_appearance(
    input: super::scena_input::ResolvedSceneInput,
    width: u32,
    height: u32,
    expectation: scena::AppearanceExpectationV1,
    out: Option<&std::path::Path>,
    detail: bool,
) -> Result<CliOutcome, String> {
    let mut build = pollster::block_on(scene_host_build_from_resolved_recipe(
        &input, width, height, false,
    ))?;
    let requested_variant = expectation.first_requested_variant();
    let mut active_variant = None;
    let mut available_variants = Vec::new();
    if let Some(import) = build.manifest.imports.first() {
        available_variants = build
            .host
            .material_variants(import.import_handle)
            .map_err(|error| format!("failed to list recipe material variants: {error}"))?;
        if let Some(variant) = requested_variant
            && available_variants
                .iter()
                .any(|candidate| candidate == variant)
        {
            build
                .host
                .set_active_material_variant(import.import_handle, Some(variant))
                .map_err(|error| {
                    format!("failed to apply recipe material variant '{variant}': {error}")
                })?;
        }
        active_variant = build
            .host
            .active_material_variant(import.import_handle)
            .map_err(|error| format!("failed to inspect recipe material variant: {error}"))?;
    }

    build
        .host
        .prepare()
        .map_err(|error| format!("failed to prepare recipe appearance scene: {error}"))?;
    build
        .host
        .render()
        .map_err(|error| format!("failed to render recipe appearance scene: {error}"))?;
    let capture = build
        .host
        .capture()
        .map_err(|error| format!("failed to capture recipe appearance scene: {error}"))?;
    if let Some(out) = out {
        ensure_parent_dir(out)?;
        capture
            .write_png(out)
            .map_err(|error| format!("failed to write PNG '{}': {error}", out.display()))?;
    }
    let inspection_json = build
        .host
        .inspect_json()
        .map_err(|error| format!("failed to inspect recipe appearance scene: {error}"))?;
    let inspection: scena::SceneInspectionReportV1 = serde_json::from_str(&inspection_json)
        .map_err(|error| format!("failed to decode recipe scene inspection report: {error}"))?;
    let options = appearance_introspection_options(detail)
        .with_active_material_variant(active_variant)
        .with_available_material_variants(available_variants);
    let report =
        build
            .host
            .renderer()
            .introspect_appearance(&capture, &inspection, &expectation, options);
    let exit_code = if report.ok { 0 } else { 1 };
    json_outcome(
        &report,
        exit_code,
        "failed to serialize recipe appearance introspection report",
    )
}

#[cfg(not(feature = "scene-host"))]
fn run_verify_recipe_appearance(
    _input: super::scena_input::ResolvedSceneInput,
    _width: u32,
    _height: u32,
    _expectation: scena::AppearanceExpectationV1,
    _out: Option<&std::path::Path>,
    _detail: bool,
) -> Result<CliOutcome, String> {
    Err("verify appearance for authored recipes requires the scene-host feature".to_owned())
}

impl VerifyAppearanceCommandArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let Some(input) = args.first() else {
            return Err(verify_appearance_usage());
        };
        let mut expect = None;
        let mut out = None;
        let mut width = None;
        let mut height = None;
        let mut detail = false;

        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--expect" => {
                    expect = Some(std::path::PathBuf::from(flag_value(
                        args, index, "--expect",
                    )?));
                    index += 2;
                }
                "--out" => {
                    out = Some(std::path::PathBuf::from(flag_value(args, index, "--out")?));
                    index += 2;
                }
                "--width" => {
                    width = Some(parse_positive_u32(
                        "--width",
                        flag_value(args, index, "--width")?,
                    )?);
                    index += 2;
                }
                "--height" => {
                    height = Some(parse_positive_u32(
                        "--height",
                        flag_value(args, index, "--height")?,
                    )?);
                    index += 2;
                }
                "--detail" => {
                    detail = true;
                    index += 1;
                }
                "--json" => {
                    index += 1;
                }
                flag => {
                    return Err(format!(
                        "unknown verify appearance flag '{flag}'; {}",
                        verify_appearance_usage()
                    ));
                }
            }
        }

        Ok(Self {
            input: input.clone(),
            expect: expect
                .ok_or_else(|| format!("missing --expect <json>; {}", verify_appearance_usage()))?,
            out,
            width,
            height,
            detail,
        })
    }
}

fn flag_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_positive_u32(flag: &str, value: String) -> Result<u32, String> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| format!("{flag} requires an unsigned integer, got '{value}'"))?;
    if parsed == 0 {
        return Err(format!("{flag} requires a positive integer, got 0"));
    }
    Ok(parsed)
}

fn verify_appearance_usage() -> String {
    "usage: scena verify appearance <asset-or-recipe> --expect <appearance-expectation.json> [--out <png>] [--width <px>] [--height <px>] [--detail]"
        .to_string()
}
