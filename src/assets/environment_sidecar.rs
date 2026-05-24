use bytemuck::{Pod, Zeroable};

use super::AssetPath;
use crate::diagnostics::AssetError;

pub const SIDECAR_FILE_SUFFIX: &str = ".prefilter.bin";
const SIDECAR_MAGIC: [u8; 16] = *b"SCENA_ENV_PF_V1\0";
const SIDECAR_VERSION: u32 = 1;
const SIDE_CAR_FACE_COUNT: usize = 6;
const RGBA_CHANNELS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum EnvironmentSidecarProfile {
    InteractiveWebGl2 = 1,
    Reference = 2,
}

impl EnvironmentSidecarProfile {
    pub const fn name(self) -> &'static str {
        match self {
            Self::InteractiveWebGl2 => "InteractiveWebGl2",
            Self::Reference => "Reference",
        }
    }

    fn from_raw(value: u32) -> Option<Self> {
        match value {
            1 => Some(Self::InteractiveWebGl2),
            2 => Some(Self::Reference),
            _ => None,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct EnvironmentSidecarHeader {
    magic: [u8; 16],
    version: u32,
    profile: u32,
    source_sha256: [u8; 32],
    cubemap_resolution: u32,
    mip_count: u32,
    brdf_lut_size: u32,
    reserved: u32,
    diffuse_rgb: [f32; 3],
    reserved_f32: f32,
    specular_float_count: u64,
    brdf_lut_float_count: u64,
}

impl EnvironmentSidecarHeader {
    fn new(input: EnvironmentSidecarHeaderInput<'_>) -> Result<Self, AssetError> {
        Ok(Self {
            magic: SIDECAR_MAGIC,
            version: SIDECAR_VERSION,
            profile: input.profile as u32,
            source_sha256: decode_sha256_hex(input.source_sha256_hex)?,
            cubemap_resolution: input.cubemap_resolution,
            mip_count: input.mip_count,
            brdf_lut_size: input.brdf_lut_size,
            reserved: 0,
            diffuse_rgb: input.diffuse_rgb,
            reserved_f32: 0.0,
            specular_float_count: input.specular_float_count,
            brdf_lut_float_count: input.brdf_lut_float_count,
        })
    }

    pub fn profile(&self) -> Option<EnvironmentSidecarProfile> {
        EnvironmentSidecarProfile::from_raw(self.profile)
    }

    pub fn profile_name(&self) -> &'static str {
        self.profile()
            .map(EnvironmentSidecarProfile::name)
            .unwrap_or("unknown")
    }

    pub const fn cubemap_resolution(&self) -> u32 {
        self.cubemap_resolution
    }

    pub const fn mip_count(&self) -> u32 {
        self.mip_count
    }

    pub const fn brdf_lut_size(&self) -> u32 {
        self.brdf_lut_size
    }

    pub const fn source_sha256_bytes(&self) -> [u8; 32] {
        self.source_sha256
    }

    pub fn source_sha256_hex(&self) -> String {
        encode_sha256_hex(&self.source_sha256)
    }

    fn validate(&self, path: &AssetPath) -> Result<(), AssetError> {
        if self.magic != SIDECAR_MAGIC {
            return Err(sidecar_parse_error(path, "invalid sidecar magic"));
        }
        if self.version != SIDECAR_VERSION {
            return Err(sidecar_parse_error(
                path,
                format!("unsupported sidecar version {}", self.version),
            ));
        }
        if self.profile().is_none() {
            return Err(sidecar_parse_error(
                path,
                format!("unsupported sidecar profile {}", self.profile),
            ));
        }
        if self.cubemap_resolution == 0 || self.mip_count == 0 || self.brdf_lut_size == 0 {
            return Err(sidecar_parse_error(
                path,
                "sidecar dimensions must be non-zero",
            ));
        }
        if self.specular_float_count == 0 || self.brdf_lut_float_count == 0 {
            return Err(sidecar_parse_error(
                path,
                "sidecar payload counts must be non-zero",
            ));
        }
        Ok(())
    }
}

struct EnvironmentSidecarHeaderInput<'a> {
    profile: EnvironmentSidecarProfile,
    source_sha256_hex: &'a str,
    cubemap_resolution: u32,
    mip_count: u32,
    brdf_lut_size: u32,
    diffuse_rgb: [f32; 3],
    specular_float_count: u64,
    brdf_lut_float_count: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentPrefilterSidecar {
    header: EnvironmentSidecarHeader,
    mips: Vec<[Vec<f32>; 6]>,
    brdf_lut: Vec<f32>,
}

impl EnvironmentPrefilterSidecar {
    pub fn new(
        profile: EnvironmentSidecarProfile,
        source_sha256_hex: &str,
        cubemap_resolution: u32,
        mips: Vec<[Vec<f32>; 6]>,
        brdf_lut: Vec<f32>,
        brdf_lut_size: u32,
        diffuse_rgb: [f32; 3],
    ) -> Result<Self, AssetError> {
        let specular_float_count = mips
            .iter()
            .flat_map(|mip| mip.iter())
            .map(Vec::len)
            .sum::<usize>();
        let header = EnvironmentSidecarHeader::new(EnvironmentSidecarHeaderInput {
            profile,
            source_sha256_hex,
            cubemap_resolution,
            mip_count: mips.len() as u32,
            brdf_lut_size,
            diffuse_rgb,
            specular_float_count: specular_float_count as u64,
            brdf_lut_float_count: brdf_lut.len() as u64,
        })?;
        Ok(Self {
            header,
            mips,
            brdf_lut,
        })
    }

    pub const fn header(&self) -> &EnvironmentSidecarHeader {
        &self.header
    }

    pub fn profile(&self) -> EnvironmentSidecarProfile {
        self.header
            .profile()
            .expect("EnvironmentPrefilterSidecar always carries a valid profile")
    }

    pub fn source_sha256_hex(&self) -> String {
        self.header.source_sha256_hex()
    }

    pub const fn cubemap_resolution(&self) -> u32 {
        self.header.cubemap_resolution
    }

    pub const fn mip_count(&self) -> u32 {
        self.header.mip_count
    }

    pub const fn brdf_lut_size(&self) -> u32 {
        self.header.brdf_lut_size
    }

    pub const fn diffuse_rgb(&self) -> [f32; 3] {
        self.header.diffuse_rgb
    }

    pub fn mips(&self) -> &[[Vec<f32>; 6]] {
        &self.mips
    }

    pub fn brdf_lut(&self) -> &[f32] {
        &self.brdf_lut
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(
            std::mem::size_of::<EnvironmentSidecarHeader>()
                + (self.header.specular_float_count as usize + self.brdf_lut.len())
                    * std::mem::size_of::<f32>(),
        );
        bytes.extend_from_slice(bytemuck::bytes_of(&self.header));
        for mip in &self.mips {
            for face in mip {
                bytes.extend_from_slice(bytemuck::cast_slice(face));
            }
        }
        bytes.extend_from_slice(bytemuck::cast_slice(&self.brdf_lut));
        bytes
    }

    pub fn parse(path: impl Into<AssetPath>, bytes: &[u8]) -> Result<Self, AssetError> {
        let path = path.into();
        let header_size = std::mem::size_of::<EnvironmentSidecarHeader>();
        if bytes.len() < header_size {
            return Err(sidecar_parse_error(
                &path,
                format!(
                    "sidecar is {} byte(s), smaller than {header_size}-byte header",
                    bytes.len()
                ),
            ));
        }
        let header =
            bytemuck::pod_read_unaligned::<EnvironmentSidecarHeader>(&bytes[..header_size]);
        header.validate(&path)?;
        let payload = &bytes[header_size..];
        if !payload.len().is_multiple_of(std::mem::size_of::<f32>()) {
            return Err(sidecar_parse_error(
                &path,
                "sidecar payload length is not f32-aligned",
            ));
        }
        let floats = payload
            .chunks_exact(std::mem::size_of::<f32>())
            .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("chunk size is 4")))
            .collect::<Vec<_>>();
        let expected_float_count =
            (header.specular_float_count + header.brdf_lut_float_count) as usize;
        if floats.len() != expected_float_count {
            return Err(sidecar_parse_error(
                &path,
                format!(
                    "sidecar payload contains {} f32 value(s), expected {expected_float_count}",
                    floats.len()
                ),
            ));
        }
        let mut cursor = 0usize;
        let mut mips = Vec::with_capacity(header.mip_count as usize);
        for mip in 0..header.mip_count {
            let mip_resolution = (header.cubemap_resolution >> mip).max(1);
            let face_float_count = (mip_resolution as usize).pow(2) * RGBA_CHANNELS;
            let mut faces: [Vec<f32>; SIDE_CAR_FACE_COUNT] = std::array::from_fn(|_| Vec::new());
            for face in &mut faces {
                let end = cursor.saturating_add(face_float_count);
                let Some(slice) = floats.get(cursor..end) else {
                    return Err(sidecar_parse_error(
                        &path,
                        "sidecar mip chain ended before all faces were decoded",
                    ));
                };
                *face = slice.to_vec();
                cursor = end;
            }
            mips.push(faces);
        }
        if cursor != header.specular_float_count as usize {
            return Err(sidecar_parse_error(
                &path,
                "sidecar specular mip count does not match header",
            ));
        }
        let brdf_lut = floats[cursor..].to_vec();
        let expected_brdf_len = (header.brdf_lut_size as usize).pow(2) * 2;
        if brdf_lut.len() != expected_brdf_len {
            return Err(sidecar_parse_error(
                &path,
                format!(
                    "BRDF LUT has {} f32 value(s), expected {expected_brdf_len}",
                    brdf_lut.len()
                ),
            ));
        }
        Ok(Self {
            header,
            mips,
            brdf_lut,
        })
    }
}

pub fn sidecar_path_for_environment(path: &AssetPath) -> AssetPath {
    AssetPath::from(format!("{}{}", path.as_str(), SIDECAR_FILE_SUFFIX))
}

pub fn parse_sidecar_header(
    path: impl Into<AssetPath>,
    bytes: &[u8],
) -> Result<EnvironmentSidecarHeader, AssetError> {
    let path = path.into();
    let header_size = std::mem::size_of::<EnvironmentSidecarHeader>();
    if bytes.len() < header_size {
        return Err(sidecar_parse_error(
            &path,
            format!(
                "sidecar is {} byte(s), smaller than {header_size}-byte header",
                bytes.len()
            ),
        ));
    }
    let header = bytemuck::pod_read_unaligned::<EnvironmentSidecarHeader>(&bytes[..header_size]);
    header.validate(&path)?;
    Ok(header)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(bytes);
    encode_sha256_hex(&hasher.finalize().into())
}

fn decode_sha256_hex(value: &str) -> Result<[u8; 32], AssetError> {
    if value.len() != 64 {
        return Err(AssetError::Parse {
            path: "environment-sidecar".to_string(),
            reason: format!("SHA-256 must be 64 hex characters, got {}", value.len()),
        });
    }
    let mut out = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        out[index] = decode_hex_byte(pair[0], pair[1]).ok_or_else(|| AssetError::Parse {
            path: "environment-sidecar".to_string(),
            reason: "SHA-256 contains non-hex characters".to_string(),
        })?;
    }
    Ok(out)
}

fn encode_sha256_hex(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn decode_hex_byte(high: u8, low: u8) -> Option<u8> {
    Some(hex_nibble(high)? << 4 | hex_nibble(low)?)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn sidecar_parse_error(path: &AssetPath, reason: impl Into<String>) -> AssetError {
    AssetError::Parse {
        path: path.as_str().to_string(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_round_trip_preserves_header_and_payload() {
        let mips = vec![
            std::array::from_fn(|face| vec![face as f32; 4 * 4 * 4]),
            std::array::from_fn(|face| vec![10.0 + face as f32; 2 * 2 * 4]),
        ];
        let source_sha = "ae94a965734e6306216feb48d6dd7154b1dbc484a605200bf13cb9ae23799b7b";
        let sidecar = EnvironmentPrefilterSidecar::new(
            EnvironmentSidecarProfile::InteractiveWebGl2,
            source_sha,
            4,
            mips,
            vec![0.25; 8],
            2,
            [0.1, 0.2, 0.3],
        )
        .expect("sidecar builds");

        let parsed =
            EnvironmentPrefilterSidecar::parse("test.hdr.prefilter.bin", &sidecar.to_bytes())
                .expect("sidecar parses");

        assert_eq!(
            parsed.profile(),
            EnvironmentSidecarProfile::InteractiveWebGl2
        );
        assert_eq!(parsed.source_sha256_hex(), source_sha);
        assert_eq!(parsed.cubemap_resolution(), 4);
        assert_eq!(parsed.mip_count(), 2);
        assert_eq!(parsed.brdf_lut_size(), 2);
        assert_eq!(parsed.diffuse_rgb(), [0.1, 0.2, 0.3]);
        assert_eq!(parsed.mips()[1][3], vec![13.0; 16]);
        assert_eq!(parsed.brdf_lut(), &[0.25; 8]);
    }
}
