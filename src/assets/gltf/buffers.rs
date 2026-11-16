use std::cell::Cell;
use std::collections::BTreeMap;

use ::gltf::Gltf;
use ::gltf::buffer::{Source as BufferSource, View};

use crate::diagnostics::AssetError;

use super::AssetPath;

/// The bytes behind every glTF buffer of one import, plus the running total
/// of what the importer has materialized from accessors so far.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResolvedGltfBuffers {
    raw_buffers: Vec<Vec<u8>>,
    accessor_buffers: Vec<Vec<u8>>,
    decompressed_views: BTreeMap<usize, Vec<u8>>,
    max_accessor_bytes: u64,
    accessor_bytes: Cell<u64>,
}

impl ResolvedGltfBuffers {
    pub(super) fn new(raw_buffers: Vec<Vec<u8>>, max_accessor_bytes: u64) -> Self {
        Self {
            accessor_buffers: raw_buffers.clone(),
            raw_buffers,
            decompressed_views: BTreeMap::new(),
            max_accessor_bytes,
            accessor_bytes: Cell::new(0),
        }
    }

    /// Resolves every buffer of the document to its bytes.
    ///
    /// Each source must provide at least the declared `byteLength`. Structural
    /// validation already proved that every bufferView lies inside its
    /// buffer's declared length, so after this check no view or accessor can
    /// address bytes that do not exist.
    pub(super) fn resolve(
        path: &AssetPath,
        gltf: &Gltf,
        binary_chunk: Option<&[u8]>,
        external_buffers: &BTreeMap<usize, Vec<u8>>,
        max_accessor_bytes: u64,
    ) -> Result<Self, AssetError> {
        let parse_error = |reason: String| AssetError::Parse {
            path: path.as_str().to_string(),
            reason,
        };
        let raw_buffers = gltf
            .document
            .buffers()
            .map(|buffer| {
                let byte_length = buffer.length();
                match buffer.source() {
                    BufferSource::Bin => {
                        let bin = binary_chunk.ok_or_else(|| {
                            parse_error(
                                "glTF buffer without uri requires a GLB binary chunk".to_string(),
                            )
                        })?;
                        bin.get(..byte_length).map(<[u8]>::to_vec).ok_or_else(|| {
                            parse_error(
                                "GLB binary chunk is shorter than buffer byteLength".to_string(),
                            )
                        })
                    }
                    BufferSource::Uri(uri) if uri.starts_with("data:") => {
                        let (_, encoded) = uri.split_once(";base64,").ok_or_else(|| {
                            parse_error(
                                "only embedded base64 glTF buffers are supported in this loader slice"
                                    .to_string(),
                            )
                        })?;
                        use base64::Engine;
                        let bytes = base64::engine::general_purpose::STANDARD
                            .decode(encoded)
                            .map_err(|error| {
                                parse_error(format!("invalid embedded buffer base64: {error}"))
                            })?;
                        if bytes.len() < byte_length {
                            return Err(parse_error(format!(
                                "embedded glTF buffer {} holds {} bytes but declares byteLength {byte_length}",
                                buffer.index(),
                                bytes.len()
                            )));
                        }
                        Ok(bytes)
                    }
                    BufferSource::Uri(_) => {
                        let bytes = external_buffers.get(&buffer.index()).ok_or_else(|| {
                            parse_error("external glTF buffer was not fetched".to_string())
                        })?;
                        bytes.get(..byte_length).map(<[u8]>::to_vec).ok_or_else(|| {
                            parse_error(
                                "external glTF buffer is shorter than byteLength".to_string(),
                            )
                        })
                    }
                }
            })
            .collect::<Result<Vec<_>, AssetError>>()?;
        Ok(Self::new(raw_buffers, max_accessor_bytes))
    }

    /// Reserves `elements * element_bytes` of this import's accessor byte
    /// budget for `what`, or reports that the stream does not fit.
    ///
    /// Callers reserve before they allocate, so a declared `count` can never
    /// decide how much memory a load takes. `what` names the stream in the
    /// error, for example `mesh 0 primitive 2 POSITION attribute`.
    pub(super) fn charge_accessor_bytes(
        &self,
        path: &AssetPath,
        what: &str,
        elements: usize,
        element_bytes: usize,
    ) -> Result<(), AssetError> {
        let bytes = (elements as u64).saturating_mul(element_bytes as u64);
        let total = self.accessor_bytes.get().saturating_add(bytes);
        if total > self.max_accessor_bytes {
            return Err(AssetError::PolicyViolation {
                path: path.as_str().to_string(),
                reason: format!(
                    "glTF {what} needs {bytes} bytes, which brings the asset to {total} bytes materialized from accessors and exceeds max_accessor_bytes {}",
                    self.max_accessor_bytes
                ),
                help: "simplify the asset or raise the limit with AssetLoadOptions::with_max_accessor_bytes",
            });
        }
        self.accessor_bytes.set(total);
        Ok(())
    }

    pub(super) fn reader_buffer(&self, buffer_index: usize) -> Option<&[u8]> {
        self.accessor_buffers.get(buffer_index).map(Vec::as_slice)
    }

    pub(super) fn raw_buffer(&self, buffer_index: usize) -> Option<&[u8]> {
        self.raw_buffers.get(buffer_index).map(Vec::as_slice)
    }

    pub(super) fn view_bytes(&self, view: &View<'_>) -> Option<&[u8]> {
        if let Some(decoded) = self.decompressed_views.get(&view.index()) {
            return Some(decoded.as_slice());
        }
        let buffer = self.raw_buffer(view.buffer().index())?;
        let start = view.offset();
        let end = start.checked_add(view.length())?;
        buffer.get(start..end)
    }

    #[cfg(feature = "meshopt")]
    pub(super) fn store_decompressed_view(
        &mut self,
        path: &AssetPath,
        view: &View<'_>,
        decoded: Vec<u8>,
    ) -> Result<(), AssetError> {
        if decoded.len() != view.length() {
            return Err(AssetError::Parse {
                path: path.as_str().to_string(),
                reason: format!(
                    "decompressed bufferView {} has {} byte(s), expected {}",
                    view.index(),
                    decoded.len(),
                    view.length()
                ),
            });
        }
        let destination_buffer_index = view.buffer().index();
        let destination_start = view.offset();
        let destination_end = destination_start
            .checked_add(view.length())
            .ok_or_else(|| AssetError::Parse {
                path: path.as_str().to_string(),
                reason: format!("decoded bufferView {} byte range overflowed", view.index()),
            })?;
        let Some(destination) = self.accessor_buffers.get_mut(destination_buffer_index) else {
            return Err(AssetError::Parse {
                path: path.as_str().to_string(),
                reason: format!(
                    "decoded bufferView {} targets missing buffer {destination_buffer_index}",
                    view.index()
                ),
            });
        };
        if destination.len() < destination_end {
            destination.resize(destination_end, 0);
        }
        destination[destination_start..destination_end].copy_from_slice(&decoded);
        self.decompressed_views.insert(view.index(), decoded);
        Ok(())
    }
}
