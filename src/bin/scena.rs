use std::env;
use std::process;

fn main() {
    match run(env::args().skip(1).collect()) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    }
}

fn run(args: Vec<String>) -> Result<String, String> {
    if args.is_empty() || args == ["--help"] || args == ["-h"] {
        return Ok(help_json());
    }

    match args.as_slice() {
        [command, subcommand] if command == "schema" && subcommand == "list" => {
            serde_json::to_string_pretty(&scena::schema_catalog_v1())
                .map_err(|error| format!("failed to serialize schema catalog: {error}"))
        }
        [command, subcommand, schema] if command == "schema" && subcommand == "get" => {
            let report = scena::schema_entry_report_v1(schema).ok_or_else(|| {
                let suggestion = scena::nearest_schema_name(schema)
                    .map(|name| format!("; did you mean '{name}'?"))
                    .unwrap_or_default();
                format!("unknown schema '{schema}'{suggestion}")
            })?;
            serde_json::to_string_pretty(&report)
                .map_err(|error| format!("failed to serialize schema entry: {error}"))
        }
        _ => {
            Err("unknown command; expected 'schema list' or 'schema get <scena.*.vN>'".to_string())
        }
    }
}

fn help_json() -> String {
    serde_json::json!({
        "schema": "scena.cli_help.v1",
        "commands": [
            "schema list",
            "schema get <scena.*.vN>"
        ]
    })
    .to_string()
}
