pub(crate) fn help_json() -> String {
    serde_json::json!({
        "schema": "scena.cli_help.v1",
        "commands": [
            "schema list",
            "schema get <scena.*.vN>",
            "validate-recipe <recipe.json>",
            "place <recipe.json> --import <id> --verb <verb>",
            "recipe render <recipe.json> --introspect [--verify] --out <png> [--gpu]",
            "recipe inspect-cad <recipe.json> --out-dir <dir> [--width 2560] [--height 1920] [--gpu]",
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
        "global_options": [
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
