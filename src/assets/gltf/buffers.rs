use std::collections::BTreeMap;

use ::gltf::buffer::View;

#[cfg(feature = "meshopt")]
use crate::diagnostics::AssetError;

#[cfg(feature = "meshopt")]
use super::AssetPath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResolvedGltfBuffers {
    raw_buffers: Vec<Vec<u8>>,
    accessor_buffers: Vec<Vec<u8>>,
    decompressed_views: BTreeMap<usize, Vec<u8>>,
}

impl ResolvedGltfBuffers {
    pub(super) fn new(raw_buffers: Vec<Vec<u8>>) -> Self {
        Self {
            accessor_buffers: raw_buffers.clone(),
            raw_buffers,
            decompressed_views: BTreeMap::new(),
        }
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
