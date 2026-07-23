#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliExitClass {
    Comparison,
    Usage,
    Input,
    Unsupported,
    Internal,
    Io,
    Policy,
    Interrupted,
    Runtime,
}

impl CliExitClass {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Comparison => "comparison",
            Self::Usage => "usage",
            Self::Input => "input",
            Self::Unsupported => "unsupported",
            Self::Internal => "internal",
            Self::Io => "io",
            Self::Policy => "policy",
            Self::Interrupted => "interrupted",
            Self::Runtime => "runtime",
        }
    }

    pub(crate) const fn exit_code(self) -> i32 {
        match self {
            Self::Comparison => 1,
            Self::Usage => 2,
            Self::Input => 65,
            Self::Unsupported => 69,
            Self::Internal | Self::Runtime => 70,
            Self::Io => 74,
            Self::Policy => 77,
            Self::Interrupted => 130,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CliError {
    code: &'static str,
    exit_class: CliExitClass,
    message: String,
    path: Option<String>,
    command: String,
    help: String,
    candidates: Vec<String>,
    fix: Option<serde_json::Value>,
}

impl CliError {
    pub(crate) fn invalid_arguments(args: &[String], message: impl Into<String>) -> Self {
        Self::new(
            "invalid_arguments",
            CliExitClass::Usage,
            args,
            message,
            command_help(args, CliExitClass::Usage),
        )
    }

    pub(crate) fn invalid_command(args: &[String], message: impl Into<String>) -> Self {
        Self::new(
            "invalid_command",
            CliExitClass::Usage,
            args,
            message,
            "run `scena --help` and select one of the declared command paths".to_owned(),
        )
    }

    pub(crate) fn internal(args: &[String], message: impl Into<String>) -> Self {
        Self::new(
            "internal_error",
            CliExitClass::Internal,
            args,
            message,
            command_help(args, CliExitClass::Internal),
        )
    }

    pub(crate) fn classify(
        args: &[String],
        message: impl Into<String>,
        candidates: Vec<String>,
    ) -> Self {
        let message = message.into();
        let lower = message.to_ascii_lowercase();
        let (code, exit_class) = if lower.starts_with("unknown schema") {
            ("unknown_schema", CliExitClass::Input)
        } else if lower.starts_with("unknown vocabulary")
            || lower.starts_with("unknown agent template")
            || lower.starts_with("unknown examples agent template")
            || lower.starts_with("unknown template")
        {
            ("unknown_name", CliExitClass::Input)
        } else if lower.contains("unavailable in this build") {
            ("feature_unavailable", CliExitClass::Unsupported)
        } else if is_usage_error(&lower) {
            ("invalid_arguments", CliExitClass::Usage)
        } else if lower.contains("policy violation")
            || lower.contains("sandbox")
            || lower.contains("outside approved")
        {
            ("policy_violation", CliExitClass::Policy)
        } else if lower.contains("no such file")
            || lower.contains("not found")
            || lower.contains("failed to read")
        {
            ("input_not_found", CliExitClass::Input)
        } else if lower.contains("unsupported") {
            ("unsupported", CliExitClass::Unsupported)
        } else if lower.contains("gpu") || lower.contains("adapter") || lower.contains("backend") {
            ("backend_unavailable", CliExitClass::Unsupported)
        } else if lower.contains("interrupted") || lower.contains("cancelled") {
            ("interrupted", CliExitClass::Interrupted)
        } else if lower.contains("failed to write") || lower.contains("i/o") {
            ("io_error", CliExitClass::Io)
        } else if lower.contains("failed to serialize") || lower.contains("must be a json object") {
            ("internal_error", CliExitClass::Internal)
        } else {
            ("runtime_error", CliExitClass::Runtime)
        };
        let mut error = Self::new(
            code,
            exit_class,
            args,
            message,
            command_help(args, exit_class),
        );
        error.candidates = candidates;
        error
    }

    fn new(
        code: &'static str,
        exit_class: CliExitClass,
        args: &[String],
        message: impl Into<String>,
        help: String,
    ) -> Self {
        Self {
            code,
            exit_class,
            message: message.into(),
            path: None,
            command: command_path(args),
            help,
            candidates: Vec::new(),
            fix: None,
        }
    }

    pub(crate) const fn exit_code(&self) -> i32 {
        self.exit_class.exit_code()
    }

    pub(crate) fn report(&self) -> serde_json::Value {
        serde_json::json!({
            "schema": "scena.cli_error.v1",
            "ok": false,
            "code": self.code,
            "exit_class": self.exit_class.name(),
            "exit_code": self.exit_code(),
            "message": self.message,
            "path": self.path,
            "context": { "command": self.command },
            "help": self.help,
            "candidates": self.candidates,
            "fix": self.fix,
        })
    }
}

fn is_usage_error(message: &str) -> bool {
    message.starts_with("usage:")
        || message.starts_with("unknown ")
        || message.starts_with("duplicate ")
        || message.starts_with("missing ")
        || message.contains(" requires a value")
        || message.contains(" requires an integer")
        || message.contains(" expected ")
        || message.contains("; usage:")
}

fn command_help(args: &[String], class: CliExitClass) -> String {
    let command = command_path(args);
    match class {
        CliExitClass::Usage => format!("run `scena {command} --help` for accepted arguments"),
        CliExitClass::Input => {
            format!("validate the input contract, then retry `scena {command}`")
        }
        CliExitClass::Unsupported => {
            "inspect `scena --version` features and `scena capabilities --live --json` before retrying"
                .to_owned()
        }
        CliExitClass::Policy => {
            "inspect `scena policy recipe` and add only an operator-approved `--allow-root`"
                .to_owned()
        }
        CliExitClass::Io => "check the reported path, stream, permissions, and free space".to_owned(),
        CliExitClass::Comparison => {
            "inspect the comparison report; exit 1 means a valid comparison found inequality"
                .to_owned()
        }
        CliExitClass::Interrupted => "retry after the interrupting condition clears".to_owned(),
        CliExitClass::Runtime => {
            "inspect the structured command diagnostics and apply their suggested fix".to_owned()
        }
        CliExitClass::Internal => {
            "preserve this JSON report and file a scena issue with the CLI version".to_owned()
        }
    }
}

fn command_path(args: &[String]) -> String {
    match args {
        [first, second, ..]
            if matches!(
                (first.as_str(), second.as_str()),
                ("schema", "list" | "get")
                    | ("guide", "agent")
                    | ("vocab", "list" | "get")
                    | ("policy", "recipe")
                    | (
                        "recipe",
                        "build" | "render" | "inspect-cad" | "capture" | "aov"
                    )
                    | ("examples", "agent")
                    | ("verify", "appearance" | "animation" | "interaction")
            ) =>
        {
            format!("{first} {second}")
        }
        [first, ..] => first.clone(),
        [] => "--help".to_owned(),
    }
}

pub(crate) fn error_taxonomy_json() -> serde_json::Value {
    serde_json::json!([
        taxonomy_row(
            CliExitClass::Comparison,
            "valid comparison found inequality"
        ),
        taxonomy_row(CliExitClass::Usage, "unknown command or invalid arguments"),
        taxonomy_row(
            CliExitClass::Input,
            "missing, malformed, or unknown input contract"
        ),
        taxonomy_row(
            CliExitClass::Unsupported,
            "feature, capability, or backend unavailable"
        ),
        taxonomy_row(
            CliExitClass::Runtime,
            "command execution failed after valid dispatch"
        ),
        taxonomy_row(CliExitClass::Internal, "serialization or invariant failure"),
        taxonomy_row(CliExitClass::Io, "output or filesystem I/O failure"),
        taxonomy_row(
            CliExitClass::Policy,
            "sandbox or operator policy rejected the request"
        ),
        taxonomy_row(
            CliExitClass::Interrupted,
            "process interrupted or cancelled"
        ),
    ])
}

fn taxonomy_row(class: CliExitClass, meaning: &'static str) -> serde_json::Value {
    serde_json::json!({
        "class": class.name(),
        "exit_code": class.exit_code(),
        "meaning": meaning,
    })
}

#[cfg(test)]
mod tests {
    use super::{CliError, CliExitClass, error_taxonomy_json};

    #[test]
    fn every_exit_class_has_one_stable_code_and_help() {
        let taxonomy = error_taxonomy_json();
        let rows = taxonomy.as_array().expect("taxonomy is an array");
        assert_eq!(rows.len(), 9);
        for row in rows {
            assert!(row["class"].as_str().is_some_and(|value| !value.is_empty()));
            assert!(row["exit_code"].as_i64().is_some());
            assert!(
                row["meaning"]
                    .as_str()
                    .is_some_and(|value| !value.is_empty())
            );
        }
        assert_eq!(CliExitClass::Interrupted.exit_code(), 130);
    }

    #[test]
    fn runtime_text_is_not_reclassified_as_invalid_arguments() {
        let args = vec!["render".to_owned(), "scene.gltf".to_owned()];
        let error = CliError::classify(&args, "renderer failed after prepare", Vec::new());
        let report = error.report();
        assert_eq!(report["code"], "runtime_error");
        assert_eq!(report["exit_class"], "runtime");
        assert_eq!(report["exit_code"], 70);
    }
}
