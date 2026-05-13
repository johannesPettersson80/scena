use ::gltf::Document;
#[cfg(feature = "meshopt")]
use serde_json::Value;

use crate::diagnostics::AssetError;

use super::AssetPath;
use super::buffers::ResolvedGltfBuffers;

const EXTENSION: &str = "EXT_meshopt_compression";

pub(super) fn decode_meshopt_buffer_views(
    path: &AssetPath,
    document: &Document,
    buffers: &mut ResolvedGltfBuffers,
) -> Result<(), AssetError> {
    #[cfg(not(feature = "meshopt"))]
    let required = document
        .extensions_required()
        .any(|extension| extension == EXTENSION);

    for view in document.views() {
        let Some(extension) = view.extension_value(EXTENSION) else {
            continue;
        };

        #[cfg(not(feature = "meshopt"))]
        {
            let _ = extension;
            if required {
                return Err(AssetError::UnsupportedRequiredExtension {
                    path: path.as_str().to_string(),
                    extension: EXTENSION.to_string(),
                });
            }
            if buffers.view_bytes(&view).is_none() {
                return Err(AssetError::UnsupportedOptionalExtensionUsed {
                    path: path.as_str().to_string(),
                    extension: EXTENSION.to_string(),
                    help: "enable the meshopt feature or provide a valid non-extension bufferView fallback"
                        .to_string(),
                });
            }
            continue;
        }

        #[cfg(feature = "meshopt")]
        {
            let spec = MeshoptBufferView::parse(path, view.index(), extension)?;
            let decoded = spec.decode(path, view.index(), buffers)?;
            buffers.store_decompressed_view(path, &view, decoded)?;
        }
    }
    Ok(())
}

#[cfg(feature = "meshopt")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MeshoptMode {
    Attributes,
    Triangles,
    Indices,
}

#[cfg(feature = "meshopt")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MeshoptFilter {
    None,
    Octahedral,
    Quaternion,
    Exponential,
}

#[cfg(feature = "meshopt")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct MeshoptBufferView {
    buffer: usize,
    byte_offset: usize,
    byte_length: usize,
    byte_stride: usize,
    count: usize,
    mode: MeshoptMode,
    filter: MeshoptFilter,
}

#[cfg(feature = "meshopt")]
impl MeshoptBufferView {
    fn parse(path: &AssetPath, view_index: usize, value: &Value) -> Result<Self, AssetError> {
        Ok(Self {
            buffer: required_usize(path, view_index, value, "buffer")?,
            byte_offset: optional_usize(path, view_index, value, "byteOffset")?.unwrap_or(0),
            byte_length: required_usize(path, view_index, value, "byteLength")?,
            byte_stride: required_usize(path, view_index, value, "byteStride")?,
            count: required_usize(path, view_index, value, "count")?,
            mode: required_mode(path, view_index, value)?,
            filter: optional_filter(path, view_index, value)?,
        })
    }

    fn decode(
        self,
        path: &AssetPath,
        view_index: usize,
        buffers: &ResolvedGltfBuffers,
    ) -> Result<Vec<u8>, AssetError> {
        if self.byte_stride == 0 {
            return Err(parse_error(
                path,
                format!("EXT_meshopt_compression view {view_index} has byteStride 0"),
            ));
        }
        let decoded_len = checked_mul(
            path,
            self.count,
            self.byte_stride,
            "decoded meshopt byte length",
        )?;
        let Some(source_buffer) = buffers.raw_buffer(self.buffer) else {
            return Err(parse_error(
                path,
                format!(
                    "EXT_meshopt_compression view {view_index} references missing source buffer {}",
                    self.buffer
                ),
            ));
        };
        let source_end = checked_add(
            path,
            self.byte_offset,
            self.byte_length,
            "encoded meshopt view range",
        )?;
        let Some(encoded) = source_buffer.get(self.byte_offset..source_end) else {
            return Err(parse_error(
                path,
                format!(
                    "EXT_meshopt_compression view {view_index} encoded range {}..{} exceeds source buffer length {}",
                    self.byte_offset,
                    source_end,
                    source_buffer.len()
                ),
            ));
        };

        let mut decoded = vec![0; decoded_len];
        let result = match self.mode {
            MeshoptMode::Attributes => {
                validate_attribute_stride(path, view_index, self.byte_stride)?;
                unsafe {
                    meshopt::ffi::meshopt_decodeVertexBuffer(
                        decoded.as_mut_ptr().cast(),
                        self.count,
                        self.byte_stride,
                        encoded.as_ptr(),
                        encoded.len(),
                    )
                }
            }
            MeshoptMode::Triangles => {
                validate_index_stride(path, view_index, self.byte_stride)?;
                unsafe {
                    meshopt::ffi::meshopt_decodeIndexBuffer(
                        decoded.as_mut_ptr().cast(),
                        self.count,
                        self.byte_stride,
                        encoded.as_ptr(),
                        encoded.len(),
                    )
                }
            }
            MeshoptMode::Indices => {
                validate_index_stride(path, view_index, self.byte_stride)?;
                unsafe {
                    meshopt::ffi::meshopt_decodeIndexSequence(
                        decoded.as_mut_ptr().cast(),
                        self.count,
                        self.byte_stride,
                        encoded.as_ptr(),
                        encoded.len(),
                    )
                }
            }
        };
        if result != 0 {
            return Err(parse_error(
                path,
                format!(
                    "EXT_meshopt_compression view {view_index} decode failed with meshopt error code {result}"
                ),
            ));
        }
        self.apply_filter(path, view_index, &mut decoded)?;
        Ok(decoded)
    }

    fn apply_filter(
        self,
        path: &AssetPath,
        view_index: usize,
        decoded: &mut [u8],
    ) -> Result<(), AssetError> {
        match self.filter {
            MeshoptFilter::None => {}
            MeshoptFilter::Octahedral => {
                if self.byte_stride != 4 && self.byte_stride != 8 {
                    return Err(parse_error(
                        path,
                        format!(
                            "EXT_meshopt_compression view {view_index} OCTAHEDRAL filter requires byteStride 4 or 8"
                        ),
                    ));
                }
                unsafe {
                    meshopt::ffi::meshopt_decodeFilterOct(
                        decoded.as_mut_ptr().cast(),
                        self.count,
                        self.byte_stride,
                    );
                }
            }
            MeshoptFilter::Quaternion => {
                if self.byte_stride != 8 {
                    return Err(parse_error(
                        path,
                        format!(
                            "EXT_meshopt_compression view {view_index} QUATERNION filter requires byteStride 8"
                        ),
                    ));
                }
                unsafe {
                    meshopt::ffi::meshopt_decodeFilterQuat(
                        decoded.as_mut_ptr().cast(),
                        self.count,
                        self.byte_stride,
                    );
                }
            }
            MeshoptFilter::Exponential => {
                if !self.byte_stride.is_multiple_of(4) {
                    return Err(parse_error(
                        path,
                        format!(
                            "EXT_meshopt_compression view {view_index} EXPONENTIAL filter requires byteStride divisible by 4"
                        ),
                    ));
                }
                unsafe {
                    meshopt::ffi::meshopt_decodeFilterExp(
                        decoded.as_mut_ptr().cast(),
                        self.count,
                        self.byte_stride,
                    );
                }
            }
        }
        Ok(())
    }
}

#[cfg(feature = "meshopt")]
fn required_usize(
    path: &AssetPath,
    view_index: usize,
    value: &Value,
    key: &str,
) -> Result<usize, AssetError> {
    let Some(raw) = value.get(key).and_then(Value::as_u64) else {
        return Err(parse_error(
            path,
            format!("EXT_meshopt_compression view {view_index} missing integer field {key}"),
        ));
    };
    usize::try_from(raw).map_err(|_| {
        parse_error(
            path,
            format!(
                "EXT_meshopt_compression view {view_index} field {key}={raw} exceeds platform usize"
            ),
        )
    })
}

#[cfg(feature = "meshopt")]
fn optional_usize(
    path: &AssetPath,
    view_index: usize,
    value: &Value,
    key: &str,
) -> Result<Option<usize>, AssetError> {
    let Some(raw) = value.get(key) else {
        return Ok(None);
    };
    let Some(raw) = raw.as_u64() else {
        return Err(parse_error(
            path,
            format!("EXT_meshopt_compression view {view_index} field {key} must be an integer"),
        ));
    };
    usize::try_from(raw).map(Some).map_err(|_| {
        parse_error(
            path,
            format!(
                "EXT_meshopt_compression view {view_index} field {key}={raw} exceeds platform usize"
            ),
        )
    })
}

#[cfg(feature = "meshopt")]
fn required_mode(
    path: &AssetPath,
    view_index: usize,
    value: &Value,
) -> Result<MeshoptMode, AssetError> {
    let Some(mode) = value.get("mode").and_then(Value::as_str) else {
        return Err(parse_error(
            path,
            format!("EXT_meshopt_compression view {view_index} missing string field mode"),
        ));
    };
    match mode {
        "ATTRIBUTES" => Ok(MeshoptMode::Attributes),
        "TRIANGLES" => Ok(MeshoptMode::Triangles),
        "INDICES" => Ok(MeshoptMode::Indices),
        _ => Err(parse_error(
            path,
            format!("EXT_meshopt_compression view {view_index} has unsupported mode {mode}"),
        )),
    }
}

#[cfg(feature = "meshopt")]
fn optional_filter(
    path: &AssetPath,
    view_index: usize,
    value: &Value,
) -> Result<MeshoptFilter, AssetError> {
    let Some(filter) = value.get("filter") else {
        return Ok(MeshoptFilter::None);
    };
    let Some(filter) = filter.as_str() else {
        return Err(parse_error(
            path,
            format!("EXT_meshopt_compression view {view_index} field filter must be a string"),
        ));
    };
    match filter {
        "NONE" => Ok(MeshoptFilter::None),
        "OCTAHEDRAL" => Ok(MeshoptFilter::Octahedral),
        "QUATERNION" => Ok(MeshoptFilter::Quaternion),
        "EXPONENTIAL" => Ok(MeshoptFilter::Exponential),
        _ => Err(parse_error(
            path,
            format!("EXT_meshopt_compression view {view_index} has unsupported filter {filter}"),
        )),
    }
}

#[cfg(feature = "meshopt")]
fn validate_index_stride(
    path: &AssetPath,
    view_index: usize,
    byte_stride: usize,
) -> Result<(), AssetError> {
    if byte_stride == 2 || byte_stride == 4 {
        Ok(())
    } else {
        Err(parse_error(
            path,
            format!(
                "EXT_meshopt_compression view {view_index} index mode requires byteStride 2 or 4"
            ),
        ))
    }
}

#[cfg(feature = "meshopt")]
fn validate_attribute_stride(
    path: &AssetPath,
    view_index: usize,
    byte_stride: usize,
) -> Result<(), AssetError> {
    if byte_stride.is_multiple_of(4) {
        Ok(())
    } else {
        Err(parse_error(
            path,
            format!(
                "EXT_meshopt_compression view {view_index} ATTRIBUTES mode requires byteStride divisible by 4"
            ),
        ))
    }
}

#[cfg(feature = "meshopt")]
fn checked_mul(
    path: &AssetPath,
    left: usize,
    right: usize,
    label: &str,
) -> Result<usize, AssetError> {
    left.checked_mul(right).ok_or_else(|| {
        parse_error(
            path,
            format!("{label} overflowed while multiplying {left} by {right}"),
        )
    })
}

#[cfg(feature = "meshopt")]
fn checked_add(
    path: &AssetPath,
    left: usize,
    right: usize,
    label: &str,
) -> Result<usize, AssetError> {
    left.checked_add(right).ok_or_else(|| {
        parse_error(
            path,
            format!("{label} overflowed while adding {left} and {right}"),
        )
    })
}

#[cfg(feature = "meshopt")]
fn parse_error(path: &AssetPath, reason: String) -> AssetError {
    AssetError::Parse {
        path: path.as_str().to_string(),
        reason,
    }
}
