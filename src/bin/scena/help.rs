pub(crate) fn help_json() -> String {
    serde_json::json!({
        "schema": "scena.cli_help.v1",
        "commands": [
            "--version",
            "schema list",
            "schema get <scena.*.vN>",
            "vocab list",
            "vocab get <name>",
            "policy recipe",
            "validate-recipe <recipe.json> [--max-imports <n>]",
            "place <recipe.json> --import <id> --verb <verb> [--apply] [--expect-source-sha256 <hex>]",
            "diff <before.recipe.json> <after.recipe.json> [--numeric-tolerance <n>] [--render --out-dir <dir>]",
            "recipe build <recipe.json> [--max-imports <n>]",
            "recipe render <recipe.json> --introspect [--verify] --out <png> [--gpu] [--max-imports <n>]",
            "recipe inspect-cad <recipe.json> --out-dir <dir> [--width 2560] [--height 1920] [--gpu]",
            "recipe capture <recipe.json> --out-dir <dir> [--views front,top,right,isometric|none] [--turntable <frames>] [--clip <name> --frames <n>] [--gpu] [--max-imports <n>]",
            "recipe aov <recipe.json> --out-dir <dir> [--passes id,depth,normal] [--max-imports <n>]",
            "examples agent [get] <template> [--out <dir>]",
            "render <asset-or-recipe> --introspect --out <png> [--gpu]",
            "inspect <asset-or-recipe>",
            "diagnose <asset-or-recipe> --visibility [--handle <u64>]",
            "doctor <asset-or-recipe>",
            "browser-proof [scene-host|m6] [--backend webgl2] [--dry-run]",
            "repair <asset-or-recipe> --from <report.json>",
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
            emits("policy recipe", &["scena.recipe_policy.v1"], &["scena.cli_error.v1"]),
            emits("validate-recipe <recipe.json> [--max-imports <n>]", &["scena.scene_recipe_validation.v1"], &["scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits("place <recipe.json> --import <id> --verb <verb> [--apply] [--expect-source-sha256 <hex>]", &["scena.placement_result.v1", "scena.recipe_patch.v1"], &["scena.placement_result.v1", "scena.recipe_patch.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits("diff <before.recipe.json> <after.recipe.json> [--numeric-tolerance <n>] [--render --out-dir <dir>]", &["scena.scene_recipe_diff_result.v1"], &["scena.scene_recipe_validation.v1", "scena.scene_recipe_build.v1", "scena.cli_error.v1"]),
            emits("recipe build <recipe.json> [--max-imports <n>]", &["scena.recipe_build_result.v1"], &["scena.recipe_build_result.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits("recipe render <recipe.json> --introspect [--verify] --out <png> [--gpu] [--max-imports <n>]", &["scena.render_introspection.v1", "scena.recipe_render_result.v1"], &["scena.recipe_render_result.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits("recipe inspect-cad <recipe.json> --out-dir <dir> [--width 2560] [--height 1920] [--gpu]", &["scena.cad_inspection_result.v1"], &["scena.recipe_render_result.v1", "scena.cli_error.v1"]),
            emits("recipe capture <recipe.json> --out-dir <dir> [--views front,top,right,isometric|none] [--turntable <frames>] [--clip <name> --frames <n>] [--gpu] [--max-imports <n>]", &["scena.capture_sequence_result.v1"], &["scena.recipe_render_result.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits("recipe aov <recipe.json> --out-dir <dir> [--passes id,depth,normal] [--max-imports <n>]", &["scena.semantic_aov_result.v1"], &["scena.recipe_render_result.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits("examples agent [get] <template> [--out <dir>]", &["scena.agent_smoke_template.v1"], &["scena.cli_error.v1"]),
            emits("render <asset-or-recipe> --introspect --out <png> [--gpu]", &["scena.render_introspection.v1"], &["scena.asset_doctor.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits("inspect <asset-or-recipe>", &["scena.scene_inspection.v1"], &["scena.asset_doctor.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits("diagnose <asset-or-recipe> --visibility [--handle <u64>]", &["scena.visibility_diagnosis.v1"], &["scena.asset_doctor.v1", "scena.scene_recipe_validation.v1", "scena.cli_error.v1"]),
            emits("doctor <asset-or-recipe>", &["scena.asset_doctor.v1"], &["scena.asset_doctor.v1", "scena.cli_error.v1"]),
            emits("browser-proof [scene-host|m6] [--backend webgl2] [--dry-run]", &["scena.browser_proof_run.v1"], &["scena.cli_error.v1"]),
            emits("repair <asset-or-recipe> --from <report.json>", &["scena.visual_repair_plan.v1", "scena.agent_loop_result.v1"], &["scena.cli_error.v1"]),
            emits("verify appearance <asset-or-recipe> --expect <appearance-expectation.json>", &["scena.appearance_introspection.v1"], &["scena.cli_error.v1"]),
            emits("verify animation <asset-or-recipe> --clip <name> --times <seconds> [--expect-change] [--expect-translations 'x,y,z;...']", &["scena.animation_introspection.v1"], &["scena.cli_error.v1"]),
            emits("verify interaction <asset-or-recipe> --expect <interaction-expectation.json>", &["scena.interaction_verification.v1"], &["scena.cli_error.v1"]),
        ],
        "global_options": [
            "--version",
            "--round-floats <0..6>"
        ],
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

fn emits(command: &str, success: &[&str], error: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "command": command,
        "emits": {
            "success": success,
            "error": error,
        }
    })
}
