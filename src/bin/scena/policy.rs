use super::scena_cli_error::{CliErrorKind, CliFailure, CliUsageError};
use std::path::{Path, PathBuf};

use super::scena_output::{CliOutcome, json_success};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RecipePolicyCliArgs {
    pub(crate) allow_roots: Vec<PathBuf>,
}

impl RecipePolicyCliArgs {
    fn parse(args: &[String]) -> Result<Self, CliUsageError> {
        let mut policy = Self::default();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--allow-root" => {
                    push_allow_root(args, index, &mut policy.allow_roots)?;
                    index += 2;
                }
                "--json" => index += 1,
                flag => {
                    return Err(CliUsageError::from(format!(
                        "unknown policy recipe flag '{flag}'; {}",
                        recipe_policy_usage()
                    )));
                }
            }
        }
        Ok(policy)
    }
}

pub(crate) fn push_allow_root(
    args: &[String],
    index: usize,
    roots: &mut Vec<PathBuf>,
) -> Result<(), CliUsageError> {
    let value = args
        .get(index + 1)
        .ok_or_else(|| CliUsageError::from("--allow-root requires a directory path".to_owned()))?;
    if value.is_empty() {
        return Err(CliUsageError::from(
            "--allow-root requires a non-empty directory path".to_owned(),
        ));
    }
    roots.push(PathBuf::from(value));
    Ok(())
}

pub(crate) fn effective_recipe_policy(
    roots: &[PathBuf],
    max_imports: Option<usize>,
) -> Result<scena::RecipeBuildPolicy, CliFailure> {
    let mut policy = scena::RecipeBuildPolicy::testing();
    if let Some(max_imports) = max_imports {
        policy = policy.with_max_imports(max_imports);
    }
    for root in roots {
        let canonical = canonical_root(root)?;
        policy = policy.with_allowed_root(canonical);
    }
    Ok(policy)
}

#[cfg(feature = "inspection")]
pub(crate) fn ensure_recipe_policy_applies(
    input_is_recipe: bool,
    roots: &[PathBuf],
) -> Result<(), CliUsageError> {
    if !input_is_recipe && !roots.is_empty() {
        return Err(CliUsageError::from(
            "--allow-root applies only to scene-recipe resolution; direct asset inputs are loaded explicitly"
                .to_owned(),
        ));
    }
    Ok(())
}

fn canonical_root(root: &Path) -> Result<PathBuf, CliFailure> {
    let canonical = root.canonicalize().map_err(|error| {
        CliFailure::new(
            CliErrorKind::InputNotFound,
            format!(
                "--allow-root '{}' must name an existing directory that can be canonicalized: {error}",
                root.display()
            ),
        )
    })?;
    let metadata = std::fs::metadata(&canonical).map_err(|error| {
        CliFailure::new(
            CliErrorKind::InputNotFound,
            format!(
                "--allow-root '{}' must name an existing directory: {error}",
                root.display()
            ),
        )
    })?;
    if !metadata.is_dir() {
        return Err(CliFailure::new(
            CliErrorKind::InputNotFound,
            format!(
                "--allow-root '{}' must name an existing directory",
                root.display()
            ),
        ));
    }
    Ok(canonical)
}

pub(crate) fn run_recipe_policy_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let args = RecipePolicyCliArgs::parse(args)?;
    let policy = effective_recipe_policy(&args.allow_roots, None)?;
    json_success(
        &policy.to_schema_report(),
        "failed to serialize effective recipe policy",
    )
}

fn recipe_policy_usage() -> &'static str {
    "usage: scena policy recipe [--allow-root <directory>]..."
}
