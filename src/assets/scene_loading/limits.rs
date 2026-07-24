use crate::diagnostics::AssetError;

use super::super::AssetPath;

pub(super) fn asset_error_path(error: &AssetError) -> Option<&str> {
    match error {
        AssetError::NotFound { path } | AssetError::Io { path, .. } => Some(path),
        _ => None,
    }
}

pub(crate) fn check_fetch_byte_limit_before_fetch(
    path: &AssetPath,
    limit: Option<usize>,
) -> Result<(), AssetError> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (path, limit);
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let Some(limit) = limit else {
            return Ok(());
        };
        if let Ok(metadata) = std::fs::metadata(path.as_str())
            && metadata.is_file()
        {
            let source_bytes = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
            if source_bytes > limit {
                return Err(AssetError::PolicyViolation {
                    path: path.as_str().to_string(),
                    reason: format!(
                        "source is {source_bytes} bytes, exceeding fetch_byte_limit {limit}"
                    ),
                    help: "use a smaller asset or raise the operator-owned fetch_byte_limit policy",
                });
            }
        }
        Ok(())
    }
}

pub(crate) fn check_fetch_byte_limit_after_fetch(
    path: &AssetPath,
    bytes: usize,
    limit: Option<usize>,
) -> Result<(), AssetError> {
    let Some(limit) = limit else {
        return Ok(());
    };
    if bytes > limit {
        return Err(AssetError::PolicyViolation {
            path: path.as_str().to_string(),
            reason: format!("source is {bytes} bytes, exceeding fetch_byte_limit {limit}"),
            help: "use a smaller asset or raise the operator-owned fetch_byte_limit policy",
        });
    }
    Ok(())
}
