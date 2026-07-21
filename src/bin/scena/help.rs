const POLICY_RECIPE_COMMAND: &str = "policy recipe [--allow-root <directory>]...";
const VALIDATE_RECIPE_COMMAND: &str = "validate-recipe <recipe.json> [--full|--syntax-only] [--max-imports <n>] [--allow-root <directory>]...";
const RECIPE_BUILD_COMMAND: &str =
    "recipe build <recipe.json> [--max-imports <n>] [--allow-root <directory>]...";
const RECIPE_RENDER_COMMAND: &str = "recipe render <recipe.json> --introspect [--verify] --out <png> [--gpu] [--max-imports <n>] [--allow-root <directory>]...";
const RENDER_COMMAND: &str =
    "render <asset-or-recipe> --introspect --out <png> [--gpu] [--allow-root <directory>]...";
const INSPECT_COMMAND: &str = "inspect <asset-or-recipe> [--allow-root <directory>]...";
const DIAGNOSE_COMMAND: &str =
    "diagnose <asset-or-recipe> --visibility [--handle <u64>] [--allow-root <directory>]...";
const DOCTOR_COMMAND: &str = "doctor <asset-or-recipe> [--allow-root <directory>]...";
const REPAIR_COMMAND: &str =
    "repair <asset-or-recipe> --from <report.json> [--allow-root <directory>]...";

pub(crate) fn help_json() -> String {
    serde_json::json!({
        "schema": "scena.cli_help.v1",
        "scope": "global",
        "commands": [
            "--version",
            "schema list",
            "schema get <scena.*.vN>",
            "vocab list",
            "vocab get <name>",
            "capabilities [--live] [--json]",
            POLICY_RECIPE_COMMAND,
            VALIDATE_RECIPE_COMMAND,
            "place <recipe.json> --import <id> --verb <verb> [--apply] [--expect-source-sha256 <hex>]",
            "diff <before.recipe.json> <after.recipe.json> [--numeric-tolerance <n>] [--render --out-dir <dir>] [--exit-code]",
            RECIPE_BUILD_COMMAND,
            RECIPE_RENDER_COMMAND,
            "recipe inspect-cad <recipe.json> --out-dir <dir> [--width 2560] [--height 1920] [--gpu]",
            "recipe capture <recipe.json> --out-dir <dir> [--views front,top,right,isometric|none] [--turntable <frames>] [--clip <name> --frames <n>] [--gpu] [--max-imports <n>]",
            "recipe aov <recipe.json> --out-dir <dir> [--passes id,depth,normal] [--max-imports <n>]",
            "examples agent list",
            "examples agent get <template> [--out <dir>]",
            RENDER_COMMAND,
            INSPECT_COMMAND,
            DIAGNOSE_COMMAND,
            DOCTOR_COMMAND,
            "browser-proof [scene-host|m6] [--backend webgl2] [--dry-run]",
            REPAIR_COMMAND,
            "verify appearance <asset-or-recipe> --expect <appearance-expectation.json>",
            "verify animation <asset-or-recipe> --clip <name> --times <seconds> [--expect-change] [--expect-translations 'x,y,z;...']",
            "verify interaction <asset-or-recipe> --expect <interaction-expectation.json>"
        ],
        "command_contracts": [
            emits("--version", &["scena.cli_version.v1"], &["scena.cli_error.v1"]),
            emits("schema list", &["scena.schema_catalog.v1"], &["scena.cli_error.v1"]),
            emits("schema get <scena.*.vN>", &["scena.schema_entry.v1"], &["scena.cli_error.v1"]),
            emits("vocab list", &["scena.vocab.v1"], &["scena.cli_error.v1"]),
            emits("vocab get <name>", &["scena.vocab.v1"], &["scena.cli_error.v1"]),
            emits("capabilities [--live] [--json]", &["scena.capability_report.v1"], &["scena.capability_report.v1", "scena.cli_error.v1"]),
            emits(POLICY_RECIPE_COMMAND, &["scena.recipe_policy.v1"], &["scena.cli_error.v1"]),
            emits(VALIDATE_RECIPE_COMMAND, &["scena.scene_recipe_validation.v1"], &["scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits("place <recipe.json> --import <id> --verb <verb> [--apply] [--expect-source-sha256 <hex>]", &["scena.placement_result.v1", "scena.recipe_patch.v1"], &["scena.placement_result.v1", "scena.recipe_patch.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits("diff <before.recipe.json> <after.recipe.json> [--numeric-tolerance <n>] [--render --out-dir <dir>] [--exit-code]", &["scena.scene_recipe_diff_result.v1"], &["scena.scene_recipe_validation.v1", "scena.scene_recipe_build.v1", "scena.cli_error.v1"]),
            emits(RECIPE_BUILD_COMMAND, &["scena.recipe_build_result.v1"], &["scena.recipe_build_result.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits(RECIPE_RENDER_COMMAND, &["scena.render_introspection.v1", "scena.recipe_render_result.v1"], &["scena.recipe_render_result.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits("recipe inspect-cad <recipe.json> --out-dir <dir> [--width 2560] [--height 1920] [--gpu]", &["scena.cad_inspection_result.v1"], &["scena.recipe_render_result.v1", "scena.cli_error.v1"]),
            emits("recipe capture <recipe.json> --out-dir <dir> [--views front,top,right,isometric|none] [--turntable <frames>] [--clip <name> --frames <n>] [--gpu] [--max-imports <n>]", &["scena.capture_sequence_result.v1"], &["scena.recipe_render_result.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits("recipe aov <recipe.json> --out-dir <dir> [--passes id,depth,normal] [--max-imports <n>]", &["scena.semantic_aov_result.v1"], &["scena.recipe_render_result.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits("examples agent list", &["scena.agent_template_catalog.v1"], &["scena.cli_error.v1"]),
            emits("examples agent get <template> [--out <dir>]", &["scena.agent_smoke_template.v1"], &["scena.cli_error.v1"]),
            emits(RENDER_COMMAND, &["scena.render_introspection.v1"], &["scena.asset_doctor.v1", "scena.recipe_build_result.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits(INSPECT_COMMAND, &["scena.scene_inspection.v1"], &["scena.asset_doctor.v1", "scena.recipe_build_result.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits(DIAGNOSE_COMMAND, &["scena.visibility_diagnosis.v1"], &["scena.asset_doctor.v1", "scena.recipe_build_result.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits(DOCTOR_COMMAND, &["scena.asset_doctor.v1", "scena.recipe_build_result.v1"], &["scena.asset_doctor.v1", "scena.recipe_build_result.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits("browser-proof [scene-host|m6] [--backend webgl2] [--dry-run]", &["scena.browser_proof_run.v1"], &["scena.cli_error.v1"]),
            emits(REPAIR_COMMAND, &["scena.visual_repair_plan.v1", "scena.agent_loop_result.v1"], &["scena.recipe_build_result.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits("verify appearance <asset-or-recipe> --expect <appearance-expectation.json>", &["scena.appearance_introspection.v1"], &["scena.recipe_build_result.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits("verify animation <asset-or-recipe> --clip <name> --times <seconds> [--expect-change] [--expect-translations 'x,y,z;...']", &["scena.animation_introspection.v1"], &["scena.recipe_build_result.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits("verify interaction <asset-or-recipe> --expect <interaction-expectation.json>", &["scena.interaction_verification.v1"], &["scena.recipe_build_result.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
        ],
        "global_options": [
            "--version",
            "--round-floats <0..6>"
        ],
        "backend_selection": {
            "default": "headless",
            "gpu_flag": "--gpu requests headless_gpu with an explicitly reported headless fallback",
            "environment": "SCENA_USE_GPU is test/proof metadata and is ignored by CLI execution",
            "result_field": "backend_selection"
        },
        "recipe_policy": {
            "allow_root": "repeat --allow-root <directory> to add only that canonical external directory to compiled recipe roots",
            "scope": "validation, build, render, inspect, diagnose, doctor, and repair use the same effective policy",
            "direct_assets": "--allow-root is rejected for direct asset inputs because it governs authored recipe references",
            "result_field": "policy"
        },
        "guides": [
            {
                "name": "llm-app-builder",
                "path": "docs/guides/llm-app-builder.md",
                "url": "https://github.com/johannesPettersson80/scena/blob/main/docs/guides/llm-app-builder.md",
                "summary": "LLM workflow for building and verifying scena apps through public schemas, recipes, CLI diagnostics, and machine-checkable reports."
            }
        ]
    })
    .to_string()
}

pub(crate) fn command_help_json(args: &[String]) -> Option<String> {
    let normalized: Vec<_> = args
        .iter()
        .map(String::as_str)
        .filter(|arg| *arg != "--json")
        .collect();
    let (path, last) = normalized.split_last()?;
    if *path != "--help" && *path != "-h" {
        return None;
    }
    let (command, usage) = command_usage(last)?;
    let global: serde_json::Value =
        serde_json::from_str(&help_json()).expect("global CLI help is valid JSON");
    let contract = global["command_contracts"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["command"] == command))
        .cloned();
    Some(
        serde_json::json!({
            "schema": "scena.cli_help.v1",
            "scope": "command",
            "command": command,
            "usage": usage,
            "contract": contract,
            "notes": command_notes(command),
            "global_help": "scena --help"
        })
        .to_string(),
    )
}

fn command_notes(command: &str) -> &'static [&'static str] {
    match command {
        REPAIR_COMMAND => &[
            "the target asset is loaded through asset doctor, or the target recipe is fully built through its effective policy, before a repair plan is derived from the report",
            "a second positional target is invalid",
        ],
        _ => &[],
    }
}

fn command_usage(path: &[&str]) -> Option<(&'static str, &'static str)> {
    Some(match path {
        ["version"] => ("--version", "scena --version"),
        ["schema"] => ("schema", "scena schema <list|get>"),
        ["schema", "list"] => ("schema list", "scena schema list"),
        ["schema", "get"] => ("schema get <scena.*.vN>", "scena schema get <scena.*.vN>"),
        ["vocab"] => ("vocab", "scena vocab <list|get>"),
        ["vocab", "list"] => ("vocab list", "scena vocab list"),
        ["vocab", "get"] => ("vocab get <name>", "scena vocab get <name>"),
        ["policy"] => (
            "policy",
            "scena policy recipe [--allow-root <directory>]...",
        ),
        ["policy", "recipe"] => (
            POLICY_RECIPE_COMMAND,
            "scena policy recipe [--allow-root <directory>]...",
        ),
        ["capabilities"] => (
            "capabilities [--live] [--json]",
            "scena capabilities [--live] [--json]",
        ),
        ["validate-recipe"] => (
            VALIDATE_RECIPE_COMMAND,
            "scena validate-recipe <recipe.json> [--full|--syntax-only] [--max-imports <n>] [--allow-root <directory>]...",
        ),
        ["place"] => (
            "place <recipe.json> --import <id> --verb <verb> [--apply] [--expect-source-sha256 <hex>]",
            "scena place <recipe.json> --import <id> --verb <verb> [--apply] [--expect-source-sha256 <hex>]",
        ),
        ["diff"] => (
            "diff <before.recipe.json> <after.recipe.json> [--numeric-tolerance <n>] [--render --out-dir <dir>] [--exit-code]",
            "scena diff <before.recipe.json> <after.recipe.json> [--numeric-tolerance <n>] [--render --out-dir <dir>] [--exit-code]",
        ),
        ["recipe"] => (
            "recipe",
            "scena recipe <build|render|inspect-cad|capture|aov>",
        ),
        ["recipe", "build"] => (
            RECIPE_BUILD_COMMAND,
            "scena recipe build <recipe.json> [--max-imports <n>] [--allow-root <directory>]...",
        ),
        ["recipe", "render"] => (
            RECIPE_RENDER_COMMAND,
            "scena recipe render <recipe.json> --introspect [--verify] --out <png> [--gpu] [--max-imports <n>] [--allow-root <directory>]...",
        ),
        ["recipe", "inspect-cad"] => (
            "recipe inspect-cad <recipe.json> --out-dir <dir> [--width 2560] [--height 1920] [--gpu]",
            "scena recipe inspect-cad <recipe.json> --out-dir <dir> [--width 2560] [--height 1920] [--gpu]",
        ),
        ["recipe", "capture"] => (
            "recipe capture <recipe.json> --out-dir <dir> [--views front,top,right,isometric|none] [--turntable <frames>] [--clip <name> --frames <n>] [--gpu] [--max-imports <n>]",
            "scena recipe capture <recipe.json> --out-dir <dir> [--views front,top,right,isometric|none] [--turntable <frames>] [--clip <name> --frames <n>] [--gpu] [--max-imports <n>]",
        ),
        ["recipe", "aov"] => (
            "recipe aov <recipe.json> --out-dir <dir> [--passes id,depth,normal] [--max-imports <n>]",
            "scena recipe aov <recipe.json> --out-dir <dir> [--passes id,depth,normal] [--max-imports <n>]",
        ),
        ["examples"] => ("examples", "scena examples agent <list|get>"),
        ["examples", "agent"] => (
            "examples agent get <template> [--out <dir>]",
            "scena examples agent list | scena examples agent get <template> [--out <dir>]",
        ),
        ["examples", "agent", "list"] => {
            ("examples agent list", "scena examples agent list [--json]")
        }
        ["examples", "agent", "get"] => (
            "examples agent get <template> [--out <dir>]",
            "scena examples agent get <template> [--out <dir>]",
        ),
        ["render"] => (
            RENDER_COMMAND,
            "scena render <asset-or-recipe> --introspect --out <png> [--gpu] [--allow-root <directory>]...",
        ),
        ["inspect"] => (
            INSPECT_COMMAND,
            "scena inspect <asset-or-recipe> [--allow-root <directory>]...",
        ),
        ["diagnose"] => (
            DIAGNOSE_COMMAND,
            "scena diagnose <asset-or-recipe> --visibility [--handle <u64>] [--allow-root <directory>]...",
        ),
        ["doctor"] => (
            DOCTOR_COMMAND,
            "scena doctor <asset-or-recipe> [--allow-root <directory>]...",
        ),
        ["browser-proof"] => (
            "browser-proof [scene-host|m6] [--backend webgl2] [--dry-run]",
            "scena browser-proof [scene-host|m6] [--backend webgl2] [--dry-run]",
        ),
        ["repair"] => (
            REPAIR_COMMAND,
            "scena repair <asset-or-recipe> --from <report.json> [--allow-root <directory>]...",
        ),
        ["verify"] => ("verify", "scena verify <appearance|animation|interaction>"),
        ["verify", "appearance"] => (
            "verify appearance <asset-or-recipe> --expect <appearance-expectation.json>",
            "scena verify appearance <asset-or-recipe> --expect <appearance-expectation.json>",
        ),
        ["verify", "animation"] => (
            "verify animation <asset-or-recipe> --clip <name> --times <seconds> [--expect-change] [--expect-translations 'x,y,z;...']",
            "scena verify animation <asset-or-recipe> --clip <name> --times <seconds> [--expect-change] [--expect-translations 'x,y,z;...']",
        ),
        ["verify", "interaction"] => (
            "verify interaction <asset-or-recipe> --expect <interaction-expectation.json>",
            "scena verify interaction <asset-or-recipe> --expect <interaction-expectation.json>",
        ),
        _ => return None,
    })
}

fn emits(command: &str, success: &[&str], error: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "command": command,
        "emits": {
            "success": success,
            "error": error,
        }
    })
}
