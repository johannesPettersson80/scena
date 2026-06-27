use crate::diagnostics::{Diagnostic, DiagnosticSeverity, RendererStats};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenaViewerInspectorSnapshot {
    diagnostics: Vec<ScenaViewerInspectorDiagnostic>,
    draw_calls: u64,
    triangles: u64,
    target_width: u32,
    target_height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenaViewerInspectorDiagnostic {
    severity: DiagnosticSeverity,
    code: String,
    message: String,
    help: Option<String>,
}

impl ScenaViewerInspectorSnapshot {
    pub fn from_renderer_state(diagnostics: &[Diagnostic], stats: RendererStats) -> Self {
        Self {
            diagnostics: diagnostics
                .iter()
                .map(ScenaViewerInspectorDiagnostic::from_diagnostic)
                .collect(),
            draw_calls: stats.draw_calls,
            triangles: stats.triangles,
            target_width: stats.target_width,
            target_height: stats.target_height,
        }
    }

    pub fn diagnostics(&self) -> &[ScenaViewerInspectorDiagnostic] {
        &self.diagnostics
    }

    pub const fn draw_calls(&self) -> u64 {
        self.draw_calls
    }

    pub const fn triangles(&self) -> u64 {
        self.triangles
    }

    pub const fn target_width(&self) -> u32 {
        self.target_width
    }

    pub const fn target_height(&self) -> u32 {
        self.target_height
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning)
            .count()
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
            .count()
    }

    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }

    pub fn status_text(&self) -> String {
        format!(
            "{}, {}; {} draws; {} triangles at {}x{}",
            count_label(self.error_count(), "error"),
            count_label(self.warning_count(), "warning"),
            self.draw_calls,
            self.triangles,
            self.target_width,
            self.target_height,
        )
    }
}

impl ScenaViewerInspectorDiagnostic {
    fn from_diagnostic(diagnostic: &Diagnostic) -> Self {
        Self {
            severity: diagnostic.severity(),
            code: format!("{:?}", diagnostic.code()),
            message: diagnostic.message().to_string(),
            help: diagnostic.help().map(str::to_string),
        }
    }

    pub const fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }
}

fn count_label(count: usize, singular: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {singular}s")
    }
}
