use serde_json::Value;

use crate::diagnostics::AssetError;

use super::super::super::AssetPath;

pub(super) fn required_index(
    path: &AssetPath,
    value: &Value,
    json_path: &str,
    length: usize,
) -> Result<usize, AssetError> {
    let Some(index) = value.as_u64() else {
        return Err(parse_error(
            path,
            format!("{json_path} must be a non-negative integer index"),
        ));
    };
    if index >= length as u64 {
        return Err(parse_error(
            path,
            format!(
                "{json_path} references index {index}, but the target array length is {length}"
            ),
        ));
    }
    usize::try_from(index).map_err(|_| {
        parse_error(
            path,
            format!("{json_path} index {index} does not fit usize"),
        )
    })
}

pub(super) fn values(value: Option<&Value>) -> impl Iterator<Item = &Value> {
    value.and_then(Value::as_array).into_iter().flatten()
}

pub(super) fn array_len(value: Option<&Value>) -> usize {
    value.and_then(Value::as_array).map_or(0, Vec::len)
}

pub(super) fn parse_error(path: &AssetPath, reason: impl Into<String>) -> AssetError {
    AssetError::Parse {
        path: path.as_str().to_string(),
        reason: reason.into(),
    }
}
