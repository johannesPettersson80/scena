use serde::{Deserialize, Serialize};

use super::AssetPath;

/// Source metadata attached to loaded assets.
///
/// `AssetProvenance` is intentionally domain-neutral: it records where bytes
/// came from, their hash when bytes were available, optional license/generator
/// metadata, and any derived files generated from that source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetProvenance {
    source_path: AssetPath,
    source_sha256: Option<String>,
    license: Option<String>,
    generator: Option<String>,
    derivatives: Vec<AssetDerivative>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetDerivative {
    path: AssetPath,
    sha256: String,
}

impl AssetProvenance {
    pub fn new(source_path: impl Into<AssetPath>) -> Self {
        Self {
            source_path: source_path.into(),
            source_sha256: None,
            license: None,
            generator: None,
            derivatives: Vec::new(),
        }
    }

    pub fn from_source_bytes(source_path: impl Into<AssetPath>, bytes: &[u8]) -> Self {
        Self::new(source_path).with_source_sha256(sha256_hex(bytes))
    }

    pub fn with_source_sha256(mut self, source_sha256: impl Into<String>) -> Self {
        self.source_sha256 = Some(source_sha256.into());
        self
    }

    pub fn with_license(mut self, license: impl Into<String>) -> Self {
        self.license = Some(license.into());
        self
    }

    pub fn with_generator(mut self, generator: impl Into<String>) -> Self {
        self.generator = Some(generator.into());
        self
    }

    pub fn with_derivatives(
        mut self,
        derivatives: impl IntoIterator<Item = AssetDerivative>,
    ) -> Self {
        self.derivatives = derivatives.into_iter().collect();
        self
    }

    pub fn source_path(&self) -> &AssetPath {
        &self.source_path
    }

    pub fn source_sha256(&self) -> Option<&str> {
        self.source_sha256.as_deref()
    }

    pub fn license(&self) -> Option<&str> {
        self.license.as_deref()
    }

    pub fn generator(&self) -> Option<&str> {
        self.generator.as_deref()
    }

    pub fn derivatives(&self) -> &[AssetDerivative] {
        &self.derivatives
    }
}

impl AssetDerivative {
    pub fn new(path: impl Into<AssetPath>, sha256: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            sha256: sha256.into(),
        }
    }

    pub fn path(&self) -> &AssetPath {
        &self.path
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
