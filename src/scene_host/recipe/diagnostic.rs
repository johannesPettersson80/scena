use super::*;

pub(in crate::scene_host) fn scene_host_error_diagnostic(
    path: impl Into<String>,
    code: impl Into<String>,
    error: SceneHostError,
) -> SceneRecipeDiagnosticV1 {
    error_diagnostic(
        path,
        code,
        error.to_string(),
        "fix the recipe input and retry",
    )
}

pub(in crate::scene_host) fn error_diagnostic(
    path: impl Into<String>,
    code: impl Into<String>,
    message: impl Into<String>,
    help: impl Into<String>,
) -> SceneRecipeDiagnosticV1 {
    build_diagnostic(code, "error", path, message, help, None, false)
}
