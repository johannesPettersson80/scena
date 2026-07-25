use std::env;
use std::io;
use std::process;

#[path = "scena/process_output_shared.rs"]
mod process_output;

#[path = "scena/args.rs"]
mod scena_args;
#[path = "scena/browser_proof.rs"]
mod scena_browser_proof;
#[path = "scena/capabilities.rs"]
mod scena_capabilities;
#[path = "scena/cli_error.rs"]
mod scena_cli_error;
#[cfg(all(feature = "inspection", feature = "scene-host"))]
#[path = "scena/diff.rs"]
mod scena_diff;
#[path = "scena/doctor.rs"]
mod scena_doctor;
#[cfg(feature = "inspection")]
#[path = "scena/examples_agent.rs"]
mod scena_examples_agent;
#[path = "scena/guide.rs"]
mod scena_guide;
#[path = "scena/help.rs"]
mod scena_help;
#[path = "scena/input.rs"]
mod scena_input;
#[path = "scena/output.rs"]
mod scena_output;
#[path = "scena/place.rs"]
mod scena_place;
#[path = "scena/policy.rs"]
mod scena_policy;
#[cfg(all(feature = "inspection", feature = "scene-host"))]
#[path = "scena/recipe.rs"]
mod scena_recipe;
#[cfg(feature = "inspection")]
#[path = "scena/scene_commands.rs"]
mod scena_scene_commands;
#[path = "scena/schema.rs"]
mod scena_schema;
#[path = "scena/validate.rs"]
mod scena_validate;
#[path = "scena/validate_recipe.rs"]
mod scena_validate_recipe;
#[cfg(feature = "inspection")]
#[path = "scena/verify.rs"]
mod scena_verify;
#[cfg(feature = "inspection")]
#[path = "scena/verify_animation.rs"]
mod scena_verify_animation;
#[cfg(feature = "scene-host")]
#[path = "scena/verify_interaction.rs"]
mod scena_verify_interaction;
#[path = "scena/vocab.rs"]
mod scena_vocab;

use scena_cli_error::{CliError, CliFailure};
use scena_output::{
    CliJsonStyle, CliOutcome, CliOutputFormatError, apply_output_format, parse_output_format_args,
    requested_json_style, serialize_json, success,
};

/// X01/X02: response-shaping failures classify on the error *type*.
/// `JsonShapingOnNonJsonPayload` is the caller's flag choice (usage, exit 2);
/// `Internal` is a genuine fault (exit 70).
fn output_format_error(args: &[String], error: CliOutputFormatError) -> Box<CliError> {
    Box::new(match error {
        CliOutputFormatError::JsonShapingOnNonJsonPayload(message) => {
            CliError::invalid_arguments(args, message)
        }
        CliOutputFormatError::Internal(message) => CliError::internal(args, message),
    })
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let json_style = requested_json_style(&args);
    match run(args.clone()) {
        Ok(outcome) => {
            if let Err(error) = process_output::write_stdout_line(&outcome.stdout) {
                if error.kind() == io::ErrorKind::BrokenPipe {
                    return;
                }
                process_output::write_stdout_error(
                    &error,
                    matches!(json_style, CliJsonStyle::Pretty),
                );
                process::exit(process_output::IO_ERROR_EXIT_CODE);
            }
            if outcome.exit_code != 0 {
                process::exit(outcome.exit_code);
            }
        }
        Err(error) => {
            let report = error.report();
            let text = serialize_json(&report, json_style)
                .unwrap_or_else(|serialize_error| format!("{{\"schema\":\"scena.cli_error.v1\",\"ok\":false,\"code\":\"internal_error\",\"message\":\"failed to serialize CLI error: {serialize_error}\"}}"));
            process_output::write_stderr_line(&text);
            process::exit(error.exit_code());
        }
    }
}

fn cli_error_candidates(args: &[String]) -> Vec<String> {
    if args.first().map(String::as_str) == Some("schema")
        && args.get(1).map(String::as_str) == Some("get")
        && let Some(name) = args.get(2)
    {
        return scena::nearest_name_candidates(
            name,
            scena::schema_catalog_v1()
                .entries
                .iter()
                .map(|entry| entry.schema.as_str()),
            3,
        );
    }
    examples_agent_error_candidates(args)
}

#[cfg(feature = "inspection")]
fn examples_agent_error_candidates(args: &[String]) -> Vec<String> {
    if args.first().map(String::as_str) != Some("examples")
        || args.get(1).map(String::as_str) != Some("agent")
    {
        return Vec::new();
    }
    let name = match args.get(2).map(String::as_str) {
        Some("get") => args.get(3),
        Some("list") | None => None,
        Some(_) => args.get(2),
    };
    name.map_or_else(Vec::new, |name| {
        scena_examples_agent::template_name_candidates(name)
    })
}

#[cfg(not(feature = "inspection"))]
fn examples_agent_error_candidates(_args: &[String]) -> Vec<String> {
    Vec::new()
}

fn run(args: Vec<String>) -> Result<CliOutcome, Box<CliError>> {
    let original_args = args.clone();
    let (args, output_format) = parse_output_format_args(args)
        .map_err(|message| Box::new(CliError::invalid_arguments(&original_args, message)))?;
    if args.is_empty() || args == ["--help"] || args == ["-h"] {
        let mut outcome = success(scena_help::help_json());
        apply_output_format(&mut outcome, output_format)
            .map_err(|error| output_format_error(&args, error))?;
        return Ok(outcome);
    }
    if let Some(help) = scena_help::command_help_json(&args) {
        let mut outcome = success(help);
        apply_output_format(&mut outcome, output_format)
            .map_err(|error| output_format_error(&args, error))?;
        return Ok(outcome);
    }
    if args == ["--version"] || args == ["version"] {
        let mut outcome = success(version_json());
        apply_output_format(&mut outcome, output_format)
            .map_err(|error| output_format_error(&args, error))?;
        return Ok(outcome);
    }

    let mut outcome = match args.as_slice() {
        [command, subcommand] if command == "schema" && subcommand == "list" => {
            scena_schema::run_schema_list_command()
        }
        [command, subcommand, schema] if command == "schema" && subcommand == "get" => {
            scena_schema::run_schema_get_command(schema)
        }
        [command, subcommand, schema] if command == "schema" && subcommand == "json" => {
            scena_schema::run_schema_json_command(schema)
        }
        [command, subcommand, rest @ ..] if command == "guide" && subcommand == "agent" => {
            scena_guide::run_agent_guide_command(rest)
        }
        [command, subcommand] if command == "vocab" && subcommand == "list" => {
            scena_vocab::run_vocab_list_command()
        }
        [command, rest @ ..] if command == "capabilities" => {
            scena_capabilities::run_capabilities_command(rest)
        }
        [command, subcommand, name] if command == "vocab" && subcommand == "get" => {
            scena_vocab::run_vocab_get_command(name)
        }
        [command, subcommand, rest @ ..] if command == "policy" && subcommand == "recipe" => {
            scena_policy::run_recipe_policy_command(rest)
        }
        [command, rest @ ..] if command == "validate-recipe" => {
            scena_validate_recipe::run_validate_recipe_command(rest)
        }
        [command, rest @ ..] if command == "validate" => {
            scena_validate::run_validate_command(rest)
        }
        [command, rest @ ..] if command == "place" => scena_place::run_place_command(rest),
        [command, rest @ ..] if command == "diff" => run_diff_command(rest),
        [command, subcommand, rest @ ..] if command == "recipe" && subcommand == "render" => {
            run_recipe_render_command(rest)
        }
        [command, subcommand, rest @ ..] if command == "recipe" && subcommand == "build" => {
            run_recipe_build_command(rest)
        }
        [command, subcommand, rest @ ..] if command == "recipe" && subcommand == "inspect-cad" => {
            run_recipe_inspect_cad_command(rest)
        }
        [command, subcommand, rest @ ..] if command == "recipe" && subcommand == "capture" => {
            run_recipe_capture_command(rest)
        }
        [command, subcommand, rest @ ..] if command == "recipe" && subcommand == "aov" => {
            run_recipe_aov_command(rest)
        }
        [command, subcommand, rest @ ..] if command == "examples" && subcommand == "agent" => {
            run_examples_agent_command(rest)
        }
        [command, rest @ ..] if command == "render" => run_render_command(rest),
        [command, rest @ ..] if command == "inspect" => run_inspect_command(rest),
        [command, rest @ ..] if command == "diagnose" => run_diagnose_command(rest),
        [command, rest @ ..] if command == "doctor" => scena_doctor::run_doctor_command(rest),
        [command, rest @ ..] if command == "browser-proof" => {
            scena_browser_proof::run_browser_proof_command(rest)
        }
        [command, rest @ ..] if command == "repair" => run_repair_command(rest),
        [command, subcommand, rest @ ..] if command == "verify" && subcommand == "appearance" => {
            run_verify_appearance_command(rest)
        }
        [command, subcommand, rest @ ..] if command == "verify" && subcommand == "animation" => {
            run_verify_animation_command(rest)
        }
        [command, subcommand, rest @ ..] if command == "verify" && subcommand == "interaction" => {
            run_verify_interaction_command(rest)
        }
        _ => {
            return Err(Box::new(CliError::invalid_command(
                &args,
                "unknown command; expected 'schema list', 'schema get <scena.*.vN>', 'schema json <scena.*.vN>', 'guide agent [--json|--markdown]', \
             'vocab list', 'vocab get <name>', 'policy recipe [--allow-root <directory>]...', \
             'capabilities [--live] [--json]', \
             'validate <file>', \
             'validate-recipe <recipe.json> [--allow-root <directory>]...', \
             'place <recipe.json> (--import <id>|--node <id>) --verb <verb>', \
             'diff <before.recipe.json> <after.recipe.json> [--render --out-dir <dir>] [--exit-code]', \
             'recipe build <recipe.json> [--max-imports <n>] [--allow-root <directory>]...', \
             'recipe render <recipe.json> --verify --out <png> [--allow-root <directory>]...', \
             'recipe inspect-cad <recipe.json> --out-dir <dir>', \
             'recipe capture <recipe.json> --out-dir <dir> [--views front,top,right,isometric|none] [--turntable <frames>] [--clip <name> --frames <n>] [--gpu] [--max-imports <n>]', \
             'recipe aov <recipe.json> --out-dir <dir> [--passes id,depth,normal] [--max-imports <n>]', \
             'examples agent list', 'examples agent get <template> [--out <dir>]', \
             'render <asset> --introspect --out <png>', or \
             'inspect <asset>', or \
             'diagnose <asset> --visibility [--handle <u64>]', or \
             'doctor <asset-or-recipe>', or \
             'browser-proof [scene-host|m6] [--dry-run]', or \
             'repair <asset-or-recipe> --from <report.json>', or \
             'verify appearance <asset-or-recipe> --expect <json>', or \
             'verify animation <asset-or-recipe> --clip <name> --times <seconds>', or \
             'verify interaction <asset-or-recipe> --expect <json>'"
                .to_string(),
            )));
        }
    }
    .map_err(|failure| {
        Box::new(CliError::from_failure(
            &args,
            failure,
            cli_error_candidates(&args),
        ))
    })?;
    apply_output_format(&mut outcome, output_format)
        .map_err(|error| output_format_error(&args, error))?;
    Ok(outcome)
}

#[cfg(not(all(feature = "inspection", feature = "scene-host")))]
fn feature_required(command: &str, feature: &str) -> CliFailure {
    CliFailure::feature_unavailable(format!(
        "{command} is unavailable in this build; reinstall with `cargo install scena --features {feature}` or run from source with `cargo run --features {feature} -- {command}`"
    ))
}

#[cfg(all(feature = "inspection", feature = "scene-host"))]
fn run_diff_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    scena_diff::run_diff_command(args)
}

#[cfg(not(all(feature = "inspection", feature = "scene-host")))]
fn run_diff_command(_args: &[String]) -> Result<CliOutcome, CliFailure> {
    Err(feature_required("diff", "agent"))
}

fn version_json() -> String {
    let git_commit = option_env!("SCENA_GIT_COMMIT").filter(|value| !value.is_empty());
    serde_json::to_string_pretty(&serde_json::json!({
        "schema": "scena.cli_version.v1",
        "package_name": "scena",
        "package_version": env!("CARGO_PKG_VERSION"),
        "git_commit": git_commit,
        "features": {
            "agent": cfg!(feature = "agent"),
            "browser_probe": cfg!(feature = "browser-probe"),
            "controls": cfg!(feature = "controls"),
            "controls_web": cfg!(feature = "controls-web"),
            "controls_winit": cfg!(feature = "controls-winit"),
            "demo_page": cfg!(feature = "demo-page"),
            "hot_reload": cfg!(feature = "hot-reload"),
            "inspection": cfg!(feature = "inspection"),
            "khronos_samples": cfg!(feature = "khronos-samples"),
            "ktx2": cfg!(feature = "ktx2"),
            "meshopt": cfg!(feature = "meshopt"),
            "obj": cfg!(feature = "obj"),
            "production_assets": cfg!(feature = "production-assets"),
            "proof_harness": cfg!(feature = "proof-harness"),
            "scene_host": cfg!(feature = "scene-host"),
            "viewer_element": cfg!(feature = "viewer-element")
        }
    }))
    .expect("CLI version serialization is infallible")
}

#[cfg(all(feature = "inspection", feature = "scene-host"))]
fn run_recipe_render_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    scena_recipe::run_recipe_render_command(args)
}

#[cfg(all(feature = "inspection", feature = "scene-host"))]
fn run_recipe_build_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    scena_recipe::run_recipe_build_command(args)
}

#[cfg(not(all(feature = "inspection", feature = "scene-host")))]
fn run_recipe_build_command(_args: &[String]) -> Result<CliOutcome, CliFailure> {
    Err(feature_required("recipe build", "agent"))
}

#[cfg(not(all(feature = "inspection", feature = "scene-host")))]
fn run_recipe_render_command(_args: &[String]) -> Result<CliOutcome, CliFailure> {
    Err(feature_required("recipe render", "agent"))
}

#[cfg(all(feature = "inspection", feature = "scene-host"))]
fn run_recipe_inspect_cad_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    scena_recipe::run_recipe_inspect_cad_command(args)
}

#[cfg(not(all(feature = "inspection", feature = "scene-host")))]
fn run_recipe_inspect_cad_command(_args: &[String]) -> Result<CliOutcome, CliFailure> {
    Err(feature_required("recipe inspect-cad", "agent"))
}

#[cfg(all(feature = "inspection", feature = "scene-host"))]
fn run_recipe_capture_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    scena_recipe::run_recipe_capture_command(args)
}

#[cfg(not(all(feature = "inspection", feature = "scene-host")))]
fn run_recipe_capture_command(_args: &[String]) -> Result<CliOutcome, CliFailure> {
    Err(feature_required("recipe capture", "agent"))
}

#[cfg(all(feature = "inspection", feature = "scene-host"))]
fn run_recipe_aov_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    scena_recipe::run_recipe_aov_command(args)
}

#[cfg(not(all(feature = "inspection", feature = "scene-host")))]
fn run_recipe_aov_command(_args: &[String]) -> Result<CliOutcome, CliFailure> {
    Err(feature_required("recipe aov", "agent"))
}

#[cfg(feature = "inspection")]
fn run_examples_agent_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    scena_examples_agent::run_examples_agent_command(args)
}

#[cfg(not(feature = "inspection"))]
fn run_examples_agent_command(_args: &[String]) -> Result<CliOutcome, CliFailure> {
    Err(feature_required("examples agent", "agent"))
}

#[cfg(feature = "inspection")]
fn run_render_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    scena_scene_commands::run_render_command(args)
}

#[cfg(not(feature = "inspection"))]
fn run_render_command(_args: &[String]) -> Result<CliOutcome, CliFailure> {
    Err(feature_required("render", "agent"))
}

#[cfg(feature = "inspection")]
fn run_inspect_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    scena_scene_commands::run_inspect_command(args)
}

#[cfg(not(feature = "inspection"))]
fn run_inspect_command(_args: &[String]) -> Result<CliOutcome, CliFailure> {
    Err(feature_required("inspect", "agent"))
}

#[cfg(feature = "inspection")]
fn run_diagnose_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    scena_scene_commands::run_diagnose_command(args)
}

#[cfg(not(feature = "inspection"))]
fn run_diagnose_command(_args: &[String]) -> Result<CliOutcome, CliFailure> {
    Err(feature_required("diagnose --visibility", "agent"))
}

#[cfg(feature = "inspection")]
fn run_repair_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    scena_scene_commands::run_repair_command(args)
}

#[cfg(not(feature = "inspection"))]
fn run_repair_command(_args: &[String]) -> Result<CliOutcome, CliFailure> {
    Err(feature_required("repair", "agent"))
}

#[cfg(feature = "inspection")]
fn run_verify_appearance_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    scena_verify::run_verify_appearance_command(args)
}

#[cfg(not(feature = "inspection"))]
fn run_verify_appearance_command(_args: &[String]) -> Result<CliOutcome, CliFailure> {
    Err(feature_required("verify appearance", "agent"))
}

#[cfg(feature = "inspection")]
fn run_verify_animation_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    scena_verify_animation::run_verify_animation_command(args)
}

#[cfg(not(feature = "inspection"))]
fn run_verify_animation_command(_args: &[String]) -> Result<CliOutcome, CliFailure> {
    Err(feature_required("verify animation", "agent"))
}

#[cfg(feature = "scene-host")]
fn run_verify_interaction_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    scena_verify_interaction::run_verify_interaction_command(args)
}

#[cfg(not(feature = "scene-host"))]
fn run_verify_interaction_command(_args: &[String]) -> Result<CliOutcome, CliFailure> {
    Err(feature_required("verify interaction", "agent"))
}
