use crate::diagnostics::AssetError;

use super::super::AssetPath;

pub(super) fn is_glb(bytes: &[u8]) -> bool {
    bytes.starts_with(&GLB_MAGIC.to_le_bytes())
}

pub(super) fn parse_glb(
    path: &AssetPath,
    bytes: &[u8],
) -> Result<(String, Option<Vec<u8>>), AssetError> {
    if bytes.len() < GLB_HEADER_LEN {
        return Err(glb_error(path, "GLB file is shorter than its header"));
    }
    let magic = read_u32_le(path, bytes, 0)?;
    let version = read_u32_le(path, bytes, 4)?;
    let length = read_u32_le(path, bytes, 8)? as usize;
    if magic != GLB_MAGIC {
        return Err(glb_error(path, "invalid GLB magic"));
    }
    if version != 2 {
        return Err(glb_error(path, "expected GLB version 2"));
    }
    if length > bytes.len() {
        return Err(glb_error(path, "GLB declared length exceeds fetched bytes"));
    }

    let mut offset = GLB_HEADER_LEN;
    let mut json = None;
    let mut binary = None;
    while offset + GLB_CHUNK_HEADER_LEN <= length {
        let chunk_length = read_u32_le(path, bytes, offset)? as usize;
        let chunk_type = read_u32_le(path, bytes, offset + 4)?;
        offset += GLB_CHUNK_HEADER_LEN;
        let end = offset
            .checked_add(chunk_length)
            .ok_or_else(|| glb_error(path, "GLB chunk length overflow"))?;
        if end > length {
            return Err(glb_error(path, "GLB chunk exceeds declared length"));
        }
        let chunk = &bytes[offset..end];
        match chunk_type {
            GLB_JSON_CHUNK => {
                json = Some(
                    std::str::from_utf8(chunk).map_err(|error| AssetError::Parse {
                        path: path.as_str().to_string(),
                        reason: format!("invalid GLB JSON chunk UTF-8: {error}"),
                    })?,
                );
            }
            GLB_BIN_CHUNK => {
                binary = Some(chunk.to_vec());
            }
            _ => {}
        }
        offset = end;
    }

    let json = json.ok_or_else(|| glb_error(path, "GLB is missing JSON chunk"))?;
    Ok((json.to_string(), binary))
}

fn read_u32_le(path: &AssetPath, bytes: &[u8], offset: usize) -> Result<u32, AssetError> {
    let chunk = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| glb_error(path, "unexpected end of GLB while reading u32"))?;
    Ok(u32::from_le_bytes(
        chunk.try_into().expect("slice length checked above"),
    ))
}

fn glb_error(path: &AssetPath, reason: impl Into<String>) -> AssetError {
    AssetError::Parse {
        path: path.as_str().to_string(),
        reason: reason.into(),
    }
}

const GLB_MAGIC: u32 = 0x4654_6C67;
const GLB_JSON_CHUNK: u32 = 0x4E4F_534A;
const GLB_BIN_CHUNK: u32 = 0x004E_4942;
const GLB_HEADER_LEN: usize = 12;
const GLB_CHUNK_HEADER_LEN: usize = 8;
