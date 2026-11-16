//! Checked entry point for every typed accessor read in the glTF importer.
//!
//! The `gltf` crate's typed readers assume an already-validated accessor
//! layout: they reach `unreachable!()` on an unexpected component type,
//! underflow on `count == 0`, slice past a stride that is shorter than the
//! element, and allocate whatever a sparse accessor's `count` declares.
//! Importer streams are therefore decoded through [`decode_accessor`], which
//! proves those preconditions, reserves the decoded size from the import's
//! accessor byte budget, and reports a structured error that names the stream
//! being decoded. `xtask doctor` rejects any other accessor read under
//! `src/assets/gltf`.

use std::mem::size_of;

use ::gltf::accessor::{Accessor, Item, Iter as AccessorIter};

use crate::diagnostics::AssetError;

use super::AssetPath;
use super::buffers::ResolvedGltfBuffers;

/// Decodes `accessor` into a vector, converting each `T` element with
/// `convert`.
///
/// `T` must be the exact element layout of the accessor; callers choose it by
/// matching on the accessor's component type and dimensions, and a mismatch is
/// reported instead of being reinterpreted. `label` names the stream for the
/// error message, for example `mesh 0 primitive 2 TEXCOORD_0 attribute`.
///
/// The decoded size is charged to the accessor byte budget before the vector
/// is allocated, so every read is bounded, including a zero-initialised sparse
/// accessor whose storage exists only as a declared `count`.
pub(super) fn decode_accessor<T: Item, Out>(
    path: &AssetPath,
    buffers: &ResolvedGltfBuffers,
    accessor: &Accessor<'_>,
    label: &str,
    convert: impl FnMut(T) -> Out,
) -> Result<Vec<Out>, AssetError> {
    let elements = typed_accessor_iter::<T>(path, buffers, accessor, label)?;
    buffers.charge_accessor_bytes(path, label, accessor.count(), size_of::<Out>())?;
    Ok(elements.map(convert).collect())
}

/// Opens `accessor` as a stream of `T` elements after validating its layout.
fn typed_accessor_iter<'b, T: Item>(
    path: &AssetPath,
    buffers: &'b ResolvedGltfBuffers,
    accessor: &Accessor<'_>,
    label: &str,
) -> Result<AccessorIter<'b, T>, AssetError> {
    let index = accessor.index();
    let element_size = size_of::<T>();
    if accessor.size() != element_size {
        return Err(accessor_error(
            path,
            label,
            format!(
                "accessor {index} stores {}-byte elements where {element_size}-byte elements are required",
                accessor.size()
            ),
        ));
    }
    if accessor.count() == 0 {
        return Err(accessor_error(
            path,
            label,
            format!(
                "accessor {index} declares count 0; an accessor must hold at least one element"
            ),
        ));
    }
    match accessor.sparse() {
        Some(sparse) => {
            if sparse.count() == 0 {
                return Err(accessor_error(
                    path,
                    label,
                    format!(
                        "accessor {index} declares sparse.count 0; sparse storage must override at least one element"
                    ),
                ));
            }
            let index_size = sparse.indices().index_type().size();
            require_stride_covers_element(
                path,
                label,
                index,
                "sparse.indices",
                sparse.indices().view().stride(),
                index_size,
            )?;
            require_stride_covers_element(
                path,
                label,
                index,
                "sparse.values",
                sparse.values().view().stride(),
                element_size,
            )?;
        }
        None if accessor.view().is_none() => {
            return Err(accessor_error(
                path,
                label,
                format!("accessor {index} has neither a bufferView nor sparse storage to read"),
            ));
        }
        None => {}
    }
    let get_buffer = |buffer: ::gltf::Buffer<'_>| buffers.reader_buffer(buffer.index());
    AccessorIter::<T>::new(accessor.clone(), get_buffer).ok_or_else(|| {
        accessor_error(
            path,
            label,
            format!(
                "accessor {index} byte range is not contained in the bytes its buffer provides"
            ),
        )
    })
}

fn require_stride_covers_element(
    path: &AssetPath,
    label: &str,
    accessor_index: usize,
    part: &str,
    stride: Option<usize>,
    element_size: usize,
) -> Result<(), AssetError> {
    match stride {
        Some(stride) if stride < element_size => Err(accessor_error(
            path,
            label,
            format!(
                "accessor {accessor_index} {part} bufferView byteStride {stride} is smaller than its {element_size}-byte elements"
            ),
        )),
        _ => Ok(()),
    }
}

pub(super) fn accessor_error(path: &AssetPath, label: &str, reason: String) -> AssetError {
    AssetError::Parse {
        path: path.as_str().to_string(),
        reason: format!("glTF {label} {reason}"),
    }
}

/// Decodes a normalized signed byte; glTF maps -128 and -127 to -1.0.
pub(super) fn normalize_i8(value: i8) -> f32 {
    (f32::from(value) / 127.0).max(-1.0)
}

pub(super) fn normalize_u8(value: u8) -> f32 {
    f32::from(value) / 255.0
}

/// Decodes a normalized signed short; glTF maps -32768 and -32767 to -1.0.
pub(super) fn normalize_i16(value: i16) -> f32 {
    (f32::from(value) / 32767.0).max(-1.0)
}

pub(super) fn normalize_u16(value: u16) -> f32 {
    f32::from(value) / 65535.0
}
