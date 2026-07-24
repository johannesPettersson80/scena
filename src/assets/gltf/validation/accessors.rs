use super::*;

pub(super) fn validate_accessor_storage_range(
    path: &AssetPath,
    accessor_index: usize,
    accessor: &Value,
    buffer_views: &[Value],
) -> Result<(), AssetError> {
    let Some(view_index) = accessor.get("bufferView").and_then(Value::as_u64) else {
        return Ok(());
    };
    let view_index = usize::try_from(view_index).map_err(|_| {
        parse_error(
            path,
            format!("$.accessors[{accessor_index}].bufferView does not fit usize"),
        )
    })?;
    let Some(view) = buffer_views.get(view_index) else {
        return Ok(());
    };
    let element_size = accessor_element_size(path, accessor_index, accessor)?;
    let stride = view
        .get("byteStride")
        .and_then(Value::as_u64)
        .unwrap_or(element_size);
    if stride < element_size {
        return Err(parse_error(
            path,
            format!(
                "$.accessors[{accessor_index}] element size {element_size} exceeds bufferView {view_index} byteStride {stride}"
            ),
        ));
    }
    let count = required_u64(path, accessor, "count", accessor_index)?;
    let offset = accessor
        .get("byteOffset")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let required = strided_storage_len(path, accessor_index, count, stride, element_size)?;
    validate_relative_range(
        path,
        &format!("$.accessors[{accessor_index}]"),
        offset,
        required,
        view.get("byteLength").and_then(Value::as_u64).unwrap_or(0),
    )
}

pub(super) fn validate_sparse_accessor_storage_ranges(
    path: &AssetPath,
    accessor_index: usize,
    accessor: &Value,
    sparse: &Value,
    buffer_views: &[Value],
) -> Result<(), AssetError> {
    let sparse_count = sparse.get("count").and_then(Value::as_u64).unwrap_or(0);
    let accessor_count = accessor.get("count").and_then(Value::as_u64).unwrap_or(0);
    if sparse_count > accessor_count {
        return Err(parse_error(
            path,
            format!(
                "$.accessors[{accessor_index}].sparse.count {sparse_count} exceeds accessor count {accessor_count}"
            ),
        ));
    }
    let indices = sparse.get("indices").ok_or_else(|| {
        parse_error(
            path,
            format!("$.accessors[{accessor_index}].sparse.indices is required"),
        )
    })?;
    let index_size = match indices.get("componentType").and_then(Value::as_u64) {
        Some(5121) => 1,
        Some(5123) => 2,
        Some(5125) => 4,
        other => {
            return Err(parse_error(
                path,
                format!(
                    "$.accessors[{accessor_index}].sparse.indices.componentType must be UNSIGNED_BYTE, UNSIGNED_SHORT, or UNSIGNED_INT; found {other:?}"
                ),
            ));
        }
    };
    validate_sparse_range(
        path,
        accessor_index,
        "indices",
        indices,
        sparse_count,
        index_size,
        buffer_views,
    )?;
    let values = sparse.get("values").ok_or_else(|| {
        parse_error(
            path,
            format!("$.accessors[{accessor_index}].sparse.values is required"),
        )
    })?;
    validate_sparse_range(
        path,
        accessor_index,
        "values",
        values,
        sparse_count,
        accessor_element_size(path, accessor_index, accessor)?,
        buffer_views,
    )
}

fn validate_sparse_range(
    path: &AssetPath,
    accessor_index: usize,
    kind: &str,
    sparse_part: &Value,
    count: u64,
    element_size: u64,
    buffer_views: &[Value],
) -> Result<(), AssetError> {
    let Some(view_index) = sparse_part.get("bufferView").and_then(Value::as_u64) else {
        return Ok(());
    };
    let view_index = usize::try_from(view_index).map_err(|_| {
        parse_error(
            path,
            format!("$.accessors[{accessor_index}].sparse.{kind}.bufferView does not fit usize"),
        )
    })?;
    let Some(view) = buffer_views.get(view_index) else {
        return Ok(());
    };
    let offset = sparse_part
        .get("byteOffset")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let required = count.checked_mul(element_size).ok_or_else(|| {
        parse_error(
            path,
            format!("$.accessors[{accessor_index}].sparse.{kind} byte length overflowed"),
        )
    })?;
    validate_relative_range(
        path,
        &format!("$.accessors[{accessor_index}].sparse.{kind}"),
        offset,
        required,
        view.get("byteLength").and_then(Value::as_u64).unwrap_or(0),
    )
}

fn accessor_element_size(
    path: &AssetPath,
    accessor_index: usize,
    accessor: &Value,
) -> Result<u64, AssetError> {
    let component_size: u64 = match accessor.get("componentType").and_then(Value::as_u64) {
        Some(5120 | 5121) => 1,
        Some(5122 | 5123) => 2,
        Some(5125 | 5126) => 4,
        other => {
            return Err(parse_error(
                path,
                format!("$.accessors[{accessor_index}].componentType is unsupported: {other:?}"),
            ));
        }
    };
    let (rows, columns) = match accessor.get("type").and_then(Value::as_str) {
        Some("SCALAR") => (1_u64, 1_u64),
        Some("VEC2") => (2, 1),
        Some("VEC3") => (3, 1),
        Some("VEC4") => (4, 1),
        Some("MAT2") => (2, 2),
        Some("MAT3") => (3, 3),
        Some("MAT4") => (4, 4),
        other => {
            return Err(parse_error(
                path,
                format!("$.accessors[{accessor_index}].type is unsupported: {other:?}"),
            ));
        }
    };
    let column_size = component_size.checked_mul(rows).ok_or_else(|| {
        parse_error(
            path,
            format!("$.accessors[{accessor_index}] element size overflowed"),
        )
    })?;
    let stored_column_size = if columns > 1 {
        column_size.checked_add(3).map(|size| size / 4 * 4)
    } else {
        Some(column_size)
    }
    .ok_or_else(|| {
        parse_error(
            path,
            format!("$.accessors[{accessor_index}] matrix column size overflowed"),
        )
    })?;
    stored_column_size.checked_mul(columns).ok_or_else(|| {
        parse_error(
            path,
            format!("$.accessors[{accessor_index}] element size overflowed"),
        )
    })
}

fn required_u64(
    path: &AssetPath,
    value: &Value,
    field: &str,
    accessor_index: usize,
) -> Result<u64, AssetError> {
    value.get(field).and_then(Value::as_u64).ok_or_else(|| {
        parse_error(
            path,
            format!("$.accessors[{accessor_index}].{field} must be an unsigned integer"),
        )
    })
}

fn strided_storage_len(
    path: &AssetPath,
    accessor_index: usize,
    count: u64,
    stride: u64,
    element_size: u64,
) -> Result<u64, AssetError> {
    if count == 0 {
        return Ok(0);
    }
    count
        .checked_sub(1)
        .and_then(|count| count.checked_mul(stride))
        .and_then(|prefix| prefix.checked_add(element_size))
        .ok_or_else(|| {
            parse_error(
                path,
                format!("$.accessors[{accessor_index}] strided byte length overflowed"),
            )
        })
}

fn validate_relative_range(
    path: &AssetPath,
    label: &str,
    offset: u64,
    required: u64,
    available: u64,
) -> Result<(), AssetError> {
    let end = offset.checked_add(required).ok_or_else(|| {
        parse_error(
            path,
            format!("{label} byte range overflowed: {offset} + {required}"),
        )
    })?;
    if end > available {
        return Err(parse_error(
            path,
            format!("{label} byte range 0..{end} exceeds bufferView byteLength {available}"),
        ));
    }
    Ok(())
}
