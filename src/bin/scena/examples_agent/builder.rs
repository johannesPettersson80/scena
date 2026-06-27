use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn write_json_file(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| {
            format!("failed to create directory '{}': {error}", parent.display())
        })?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(value)
            .map_err(|error| format!("failed to serialize '{}': {error}", path.display()))?,
    )
    .map_err(|error| format!("failed to write '{}': {error}", path.display()))
}

pub(super) fn capture_descriptor_path(png_path: &Path) -> PathBuf {
    let stem = png_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("capture");
    png_path.with_file_name(format!("{stem}.capture.json"))
}

pub(super) fn path_for_json(path: &Path) -> String {
    path.display().to_string()
}

pub(super) struct TemplateBuilder {
    pub(super) name: String,
    status: String,
    required_features: Vec<String>,
    files: Vec<scena::AgentSmokeTemplateFileV1>,
    commands: Vec<scena::AgentSmokeTemplateCommandV1>,
    pub(super) notes: Vec<String>,
}

impl TemplateBuilder {
    pub(super) fn ready(name: &str, required_features: &[&str]) -> Self {
        Self::new(name, "ready", required_features)
    }

    fn new(name: &str, status: &str, required_features: &[&str]) -> Self {
        Self {
            name: name.to_string(),
            status: status.to_string(),
            required_features: required_features
                .iter()
                .map(|feature| feature.to_string())
                .collect(),
            files: Vec::new(),
            commands: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub(super) fn file(&mut self, kind: &str, path: &Path, schema: &str) {
        self.files.push(scena::AgentSmokeTemplateFileV1 {
            kind: kind.to_string(),
            path: path_for_json(path),
            schema: schema.to_string(),
        });
    }

    pub(super) fn command(
        &mut self,
        name: &str,
        args: Vec<&str>,
        expected_schema: &str,
        expected_ok: bool,
        artifacts: Vec<PathBuf>,
    ) {
        let mut argv = Vec::with_capacity(args.len() + 1);
        argv.push("scena".to_string());
        argv.extend(args.into_iter().map(str::to_string));
        self.commands.push(scena::AgentSmokeTemplateCommandV1 {
            name: name.to_string(),
            argv,
            expected_schema: expected_schema.to_string(),
            expected_ok,
            artifacts: artifacts.iter().map(|path| path_for_json(path)).collect(),
        });
    }

    pub(super) fn finish(self) -> scena::AgentSmokeTemplateV1 {
        scena::AgentSmokeTemplateV1 {
            schema: scena::AGENT_SMOKE_TEMPLATE_SCHEMA_V1.to_string(),
            name: self.name,
            status: self.status,
            required_features: self.required_features,
            files: self.files,
            commands: self.commands,
            notes: self.notes,
        }
    }
}
