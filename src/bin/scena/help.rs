pub(crate) fn help_json() -> String {
    serde_json::json!({
        "schema": "scena.cli_help.v1",
        "commands": [
            "schema list",
            "schema get <scena.*.vN>",
            "validate-recipe <recipe.json>",
            "place <recipe.json> --import <id> --verb <verb>",
            "examples agent <template> [--out <dir>]",
            "render <asset-or-recipe> --introspect --out <png>",
            "inspect <asset-or-recipe>",
            "diagnose <asset-or-recipe> --visibility [--handle <u64>]",
            "doctor <asset-or-recipe>",
            "repair <asset-or-recipe> --from <report.json>",
            "verify appearance <asset-or-recipe> --expect <appearance-expectation.json>",
            "verify animation <asset-or-recipe> --clip <name> --times <seconds> [--expect-change] [--expect-translations 'x,y,z;...']",
            "verify interaction <asset-or-recipe> --expect <interaction-expectation.json>"
        ],
        "global_options": [
            "--round-floats <0..6>"
        ]
    })
    .to_string()
}
