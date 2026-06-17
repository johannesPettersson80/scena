use std::env;
use std::process;

#[path = "scena/args.rs"]
mod scena_args;
#[path = "scena/browser_proof.rs"]
mod scena_browser_proof;
#[path = "scena/doctor.rs"]
mod scena_doctor;
#[cfg(feature = "inspection")]
#[path = "scena/examples_agent.rs"]
mod scena_examples_agent;
#[path = "scena/help.rs"]
mod scena_help;
#[path = "scena/input.rs"]
mod scena_input;
#[path = "scena/output.rs"]
mod scena_output;
#[path = "scena/place.rs"]
mod scena_place;
#[cfg(all(feature = "inspection", feature = "scene-host"))]
#[path = "scena/recipe.rs"]
mod scena_recipe;
#[cfg(feature = "inspection")]
#[path = "scena/scene_commands.rs"]
mod scena_scene_commands;
#[path = "scena/schema.rs"]
mod scena_schema;
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

use scena_output::{CliOutcome, apply_output_format, parse_output_format_args, success};

fn main() {
    match run(env::args().skip(1).collect()) {
        Ok(outcome) => {
            println!("{}", outcome.stdout);
            if outcome.exit_code != 0 {
                process::exit(outcome.exit_code);
            }
        }
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    }
}

fn run(args: Vec<String>) -> Result<CliOutcome, String> {
    let (args, output_format) = parse_output_format_args(args)?;
    if args.is_empty() || args == ["--help"] || args == ["-h"] {
        return Ok(success(scena_help::help_json()));
    }

    let mut outcome = match args.as_slice() {
        [command, subcommand] if command == "schema" && subcommand == "list" => {
            scena_schema::run_schema_list_command()
        }
        [command, subcommand, schema] if command == "schema" && subcommand == "get" => {
            scena_schema::run_schema_get_command(schema)
        }
        [command, rest @ ..] if command == "validate-recipe" => {
            scena_validate_recipe::run_validate_recipe_command(rest)
        }
        [command, rest @ ..] if command == "place" => scena_place::run_place_command(rest),
        [command, subcommand, rest @ ..] if command == "recipe" && subcommand == "render" => {
            run_recipe_render_command(rest)
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
        _ => Err(
            "unknown command; expected 'schema list', 'schema get <scena.*.vN>', \
             'validate-recipe <recipe.json>', \
             'place <recipe.json> --import <id> --verb <verb>', \
             'recipe render <recipe.json> --introspect --verify --out <png>', \
             'examples agent [get] <template> [--out <dir>]', \
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
        ),
    }?;
    apply_output_format(&mut outcome, output_format)?;
    Ok(outcome)
}

#[cfg(all(feature = "inspection", feature = "scene-host"))]
fn run_recipe_render_command(args: &[String]) -> Result<CliOutcome, String> {
    scena_recipe::run_recipe_render_command(args)
}

#[cfg(not(all(feature = "inspection", feature = "scene-host")))]
fn run_recipe_render_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err(
        "recipe render requires building the scena binary with the 'scene-host' feature"
            .to_string(),
    )
}

#[cfg(feature = "inspection")]
fn run_examples_agent_command(args: &[String]) -> Result<CliOutcome, String> {
    scena_examples_agent::run_examples_agent_command(args)
}

#[cfg(not(feature = "inspection"))]
fn run_examples_agent_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err(
        "examples agent requires building the scena binary with the 'inspection' feature"
            .to_string(),
    )
}

#[cfg(feature = "inspection")]
fn run_render_command(args: &[String]) -> Result<CliOutcome, String> {
    scena_scene_commands::run_render_command(args)
}

#[cfg(not(feature = "inspection"))]
fn run_render_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err(
        "render --introspect requires building the scena binary with the 'inspection' feature"
            .to_string(),
    )
}

#[cfg(feature = "inspection")]
fn run_inspect_command(args: &[String]) -> Result<CliOutcome, String> {
    scena_scene_commands::run_inspect_command(args)
}

#[cfg(not(feature = "inspection"))]
fn run_inspect_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err("inspect requires building the scena binary with the 'inspection' feature".to_string())
}

#[cfg(feature = "inspection")]
fn run_diagnose_command(args: &[String]) -> Result<CliOutcome, String> {
    scena_scene_commands::run_diagnose_command(args)
}

#[cfg(not(feature = "inspection"))]
fn run_diagnose_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err(
        "diagnose --visibility requires building the scena binary with the 'inspection' feature"
            .to_string(),
    )
}

#[cfg(feature = "inspection")]
fn run_repair_command(args: &[String]) -> Result<CliOutcome, String> {
    scena_scene_commands::run_repair_command(args)
}

#[cfg(not(feature = "inspection"))]
fn run_repair_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err("repair requires building the scena binary with the 'inspection' feature".to_string())
}

#[cfg(feature = "inspection")]
fn run_verify_appearance_command(args: &[String]) -> Result<CliOutcome, String> {
    scena_verify::run_verify_appearance_command(args)
}

#[cfg(not(feature = "inspection"))]
fn run_verify_appearance_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err(
        "verify appearance requires building the scena binary with the 'inspection' feature"
            .to_string(),
    )
}

#[cfg(feature = "inspection")]
fn run_verify_animation_command(args: &[String]) -> Result<CliOutcome, String> {
    scena_verify_animation::run_verify_animation_command(args)
}

#[cfg(not(feature = "inspection"))]
fn run_verify_animation_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err(
        "verify animation requires building the scena binary with the 'inspection' feature"
            .to_string(),
    )
}

#[cfg(feature = "scene-host")]
fn run_verify_interaction_command(args: &[String]) -> Result<CliOutcome, String> {
    scena_verify_interaction::run_verify_interaction_command(args)
}

#[cfg(not(feature = "scene-host"))]
fn run_verify_interaction_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err(
        "verify interaction requires building the scena binary with the 'scene-host' feature"
            .to_string(),
    )
}
