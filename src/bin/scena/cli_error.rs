/// X01 (`N4`): the typed classification of a command failure.
///
/// Before this existed, `CliError::classify` inferred `code` and
/// `CliExitClass` by matching substrings of the human-readable message, so
/// rewording an error could silently move it to a different exit code. A
/// producer that names its kind here is classified on the type; the prose
/// heuristic only runs for [`CliErrorKind::Unclassified`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// Which variants have a producer depends on the enabled feature set: the
// `inspection` and `scene-host` commands produce `InvalidInput`, `Io`, and
// `Runtime`, while the reduced-feature stubs that replace them produce
// `Unsupported`. No single build constructs all of them.
#[allow(dead_code, reason = "variant producers are feature-conditional")]
pub(crate) enum CliErrorKind {
    /// The caller passed arguments the command cannot accept.
    InvalidArguments,
    /// The named input could not be found or read.
    InputNotFound,
    /// The input was found but violates its contract.
    InvalidInput,
    /// This build was compiled without the Cargo feature the command needs.
    ///
    /// Distinct from [`Self::Unsupported`]: the caller fixes it by reinstalling
    /// with the named feature, which is why it keeps its own `feature_unavailable`
    /// code rather than sharing the generic one.
    FeatureUnavailable,
    /// The host lacks a capability or backend the command needs.
    Unsupported,
    /// The caller explicitly requested the complete final-photo contract on a
    /// backend where that contract is unavailable.
    FinalPhotoUnsupported,
    /// Writing or reading a stream or path failed.
    Io,
    /// A fault in scena itself; the caller cannot fix it.
    Internal,
    /// The command ran but the operation failed at runtime.
    Runtime,
    /// Not yet typed. Falls back to the legacy prose heuristic.
    ///
    /// This variant is the migration surface: every remaining producer that
    /// returns a bare `String` lands here. It is not a defect on its own, but
    /// a message-wording change *can* still alter the exit class for these.
    Unclassified,
}

impl CliErrorKind {
    /// The stable `code` and exit class for a typed kind.
    const fn code_and_class(self) -> Option<(&'static str, CliExitClass)> {
        match self {
            Self::InvalidArguments => Some(("invalid_arguments", CliExitClass::Usage)),
            Self::InputNotFound => Some(("input_not_found", CliExitClass::Input)),
            Self::InvalidInput => Some(("invalid_input", CliExitClass::Input)),
            Self::FeatureUnavailable => Some(("feature_unavailable", CliExitClass::Unsupported)),
            Self::Unsupported => Some(("unsupported", CliExitClass::Unsupported)),
            Self::FinalPhotoUnsupported => {
                Some(("final_photo_unsupported", CliExitClass::Unsupported))
            }
            Self::Io => Some(("io_error", CliExitClass::Io)),
            Self::Internal => Some(("internal_error", CliExitClass::Internal)),
            Self::Runtime => Some(("runtime_error", CliExitClass::Runtime)),
            Self::Unclassified => None,
        }
    }
}

/// A command failure carrying its typed kind alongside the human message.
///
/// Commands return `Result<CliOutcome, CliFailure>`. `From<String>` keeps every
/// not-yet-migrated producer compiling while marking it
/// [`CliErrorKind::Unclassified`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliFailure {
    pub(crate) kind: CliErrorKind,
    pub(crate) message: String,
}

impl CliFailure {
    pub(crate) fn new(kind: CliErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// The caller passed arguments the command cannot accept.
    pub(crate) fn invalid_arguments(message: impl Into<String>) -> Self {
        Self::new(CliErrorKind::InvalidArguments, message)
    }

    /// This build was compiled without the Cargo feature the command needs.
    #[cfg_attr(
        all(feature = "inspection", feature = "scene-host"),
        expect(dead_code, reason = "only feature-gated stubs construct this")
    )]
    pub(crate) fn feature_unavailable(message: impl Into<String>) -> Self {
        Self::new(CliErrorKind::FeatureUnavailable, message)
    }
}

impl From<String> for CliFailure {
    fn from(message: String) -> Self {
        Self::new(CliErrorKind::Unclassified, message)
    }
}

/// X01: a failure produced while parsing CLI arguments.
///
/// Every such failure is a usage error by construction, so the argument layer
/// returns this type instead of a bare `String`. `?` converts it to a
/// [`CliFailure`] with [`CliErrorKind::InvalidArguments`], which means no
/// argument-parsing message is ever routed through the prose heuristic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliUsageError(String);

impl From<String> for CliUsageError {
    fn from(message: String) -> Self {
        Self(message)
    }
}

impl From<&str> for CliUsageError {
    fn from(message: &str) -> Self {
        Self(message.to_owned())
    }
}

impl From<CliUsageError> for CliFailure {
    fn from(error: CliUsageError) -> Self {
        Self::new(CliErrorKind::InvalidArguments, error.0)
    }
}

impl std::fmt::Display for CliUsageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<&str> for CliFailure {
    fn from(message: &str) -> Self {
        Self::new(CliErrorKind::Unclassified, message)
    }
}

impl std::fmt::Display for CliFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

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

    /// Classifies a typed command failure.
    ///
    /// X01: when the producer named its kind, the `code` and exit class come
    /// from the type and no message text is inspected. Only
    /// [`CliErrorKind::Unclassified`] falls through to the legacy heuristic.
    pub(crate) fn from_failure(
        args: &[String],
        failure: CliFailure,
        candidates: Vec<String>,
    ) -> Self {
        let Some((code, exit_class)) = failure.kind.code_and_class() else {
            return Self::classify(args, failure.message, candidates);
        };
        let mut error = Self::new(
            code,
            exit_class,
            args,
            failure.message,
            command_help(args, exit_class),
        );
        error.candidates = candidates;
        error
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

#[cfg(test)]
mod x01_typed_classification_tests {
    use super::*;

    /// X01 (`N4`): a typed failure must be classified by its kind, never by the
    /// words in its message.
    ///
    /// Each case deliberately pairs a kind with a message whose *prose* the
    /// legacy heuristic classifies differently, and asserts the heuristic really
    /// would disagree before asserting the type wins. If classification ever
    /// regresses to reading text, the exit class changes and this fails.
    #[test]
    fn typed_failures_ignore_message_wording() {
        let args = ["render".to_owned()];
        let cases = [
            (
                CliErrorKind::InvalidArguments,
                "no such file",
                "invalid_arguments",
                CliExitClass::Usage,
            ),
            (
                CliErrorKind::InvalidArguments,
                "unsupported interaction action 'zoom'",
                "invalid_arguments",
                CliExitClass::Usage,
            ),
            (
                CliErrorKind::Io,
                "unknown schema",
                "io_error",
                CliExitClass::Io,
            ),
            (
                CliErrorKind::Runtime,
                "usage: scena render <asset>",
                "runtime_error",
                CliExitClass::Runtime,
            ),
            (
                CliErrorKind::InputNotFound,
                "failed to serialize the report",
                "input_not_found",
                CliExitClass::Input,
            ),
            (
                CliErrorKind::Internal,
                "unknown vocabulary term",
                "internal_error",
                CliExitClass::Internal,
            ),
        ];
        for (kind, message, code, class) in cases {
            let untyped = CliError::from_failure(
                &args,
                CliFailure::new(CliErrorKind::Unclassified, message),
                Vec::new(),
            );
            assert_ne!(
                untyped.exit_class, class,
                "case {message:?} proves nothing: the prose heuristic already yields {class:?}",
            );
            let typed = CliError::from_failure(&args, CliFailure::new(kind, message), Vec::new());
            assert_eq!(typed.code, code, "typed {kind:?} must set the code");
            assert_eq!(
                typed.exit_class, class,
                "typed {kind:?} must set the exit class regardless of message {message:?}",
            );
            assert_eq!(
                typed.message, message,
                "classification must not rewrite the message",
            );
        }
    }

    /// Every argument-parsing failure is a usage error by construction, so no
    /// rewording in the argument layer can change its exit code.
    #[test]
    fn argument_parsing_failures_are_always_usage_errors() {
        for message in [
            "no such file",
            "unsupported action",
            "gpu adapter unavailable",
            "failed to write output",
            "interrupted",
        ] {
            let failure = CliFailure::from(CliUsageError::from(message));
            assert_eq!(failure.kind, CliErrorKind::InvalidArguments);
            let error = CliError::from_failure(&["verify".to_owned()], failure, Vec::new());
            assert_eq!(
                error.exit_class.exit_code(),
                2,
                "argument-layer message {message:?} must stay exit 2",
            );
        }
    }
}
