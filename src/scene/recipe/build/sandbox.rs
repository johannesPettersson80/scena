use super::*;

impl RecipeBuildPolicy {
    pub(crate) fn resolve_import_uri(
        &self,
        recipe_path: &str,
        uri: &str,
        diagnostic_path: impl Into<String>,
    ) -> Result<String, Box<SceneRecipeDiagnosticV1>> {
        let diagnostic_path = diagnostic_path.into();
        let scheme = uri_scheme(uri).map(str::to_ascii_lowercase);
        if let Some(scheme) = scheme.as_deref() {
            if matches!(scheme, "http" | "https") && !self.allow_network {
                return Err(Box::new(policy_error(
                    diagnostic_path,
                    format!("network uri scheme '{scheme}' is disabled by RecipeBuildPolicy"),
                    "enable network loading in the operator-owned RecipeBuildPolicy or use a local file",
                )));
            }
            if !self.allowed_uri_schemes.contains(scheme) {
                return Err(Box::new(policy_error(
                    diagnostic_path,
                    format!("uri scheme '{scheme}' is not allowed by RecipeBuildPolicy"),
                    "use an allowed URI scheme or update the operator-owned policy",
                )));
            }
        }

        let local_uri = match scheme {
            Some(ref scheme) if scheme == "file" => {
                strip_file_scheme(uri, diagnostic_path.clone())?
            }
            Some(_) => return Ok(uri.to_owned()),
            None => uri,
        };
        let resolved = resolve_recipe_asset_uri(recipe_path, local_uri);
        self.validate_local_path(&resolved, diagnostic_path)
    }

    fn validate_local_path(
        &self,
        resolved: &str,
        diagnostic_path: String,
    ) -> Result<String, Box<SceneRecipeDiagnosticV1>> {
        let path = Path::new(resolved);
        let allowed_roots = self.canonical_allowed_roots(diagnostic_path.clone())?;
        if has_parent_dir(path) && !path.exists() {
            return Err(Box::new(policy_error(
                diagnostic_path,
                format!("local path '{resolved}' contains parent traversal and does not resolve"),
                "use a canonical path under an allowed RecipeBuildPolicy root",
            )));
        }
        let canonical = match path.canonicalize() {
            Ok(canonical) => canonical,
            Err(error) => {
                let parent = path
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                    .unwrap_or_else(|| Path::new("."));
                let parent = parent.canonicalize().map_err(|parent_error| {
                    Box::new(policy_error(
                        diagnostic_path.clone(),
                        format!(
                            "local path '{resolved}' cannot be validated under allowed roots: {error}; parent validation failed: {parent_error}"
                        ),
                        "use an existing parent directory under an allowed RecipeBuildPolicy root",
                    ))
                })?;
                if !allowed_roots.iter().any(|root| parent.starts_with(root)) {
                    return Err(Box::new(policy_error(
                        diagnostic_path,
                        format!(
                            "local path '{}' is outside the allowed recipe roots; allowed roots: {}",
                            parent.display(),
                            format_allowed_roots(&allowed_roots)
                        ),
                        "put assets under an allowed root or update the operator-owned RecipeBuildPolicy",
                    )));
                }
                return Ok(resolved.to_owned());
            }
        };
        if !allowed_roots.iter().any(|root| canonical.starts_with(root)) {
            return Err(Box::new(policy_error(
                diagnostic_path,
                format!(
                    "local path '{}' is outside the allowed recipe roots; allowed roots: {}",
                    canonical.display(),
                    format_allowed_roots(&allowed_roots)
                ),
                "put assets under an allowed root or update the operator-owned RecipeBuildPolicy",
            )));
        }
        self.validate_source_size(&canonical, diagnostic_path)?;
        Ok(stable_canonical_path(&canonical))
    }

    fn canonical_allowed_roots(
        &self,
        diagnostic_path: String,
    ) -> Result<Vec<PathBuf>, Box<SceneRecipeDiagnosticV1>> {
        if self.allowed_roots.is_empty() {
            return Err(Box::new(policy_error(
                diagnostic_path,
                "RecipeBuildPolicy has no allowed local roots",
                "configure at least one existing allowed root; scena does not silently run unsandboxed",
            )));
        }
        self.allowed_roots
            .iter()
            .map(|root| {
                root.canonicalize().map_err(|error| {
                    Box::new(policy_error(
                        diagnostic_path.clone(),
                        format!(
                            "RecipeBuildPolicy allowed root '{}' could not be canonicalized: {error}",
                            root.display()
                        ),
                        "configure only existing allowed roots; missing roots deny local file loading",
                    ))
                })
            })
            .collect()
    }

    fn validate_source_size(
        &self,
        canonical: &Path,
        diagnostic_path: String,
    ) -> Result<(), Box<SceneRecipeDiagnosticV1>> {
        let Ok(metadata) = std::fs::metadata(canonical) else {
            return Ok(());
        };
        if !metadata.is_file() {
            return Ok(());
        }
        let source_bytes = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
        if source_bytes > self.fetch_byte_limit {
            return Err(Box::new(policy_error(
                diagnostic_path,
                format!(
                    "local resource is {source_bytes} bytes, exceeding RecipeBuildPolicy fetch_byte_limit {}",
                    self.fetch_byte_limit
                ),
                "use a smaller resource or raise the operator-owned fetch_byte_limit policy",
            )));
        }
        Ok(())
    }
}
