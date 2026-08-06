use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssetPath(String);

impl AssetPath {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for AssetPath {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for AssetPath {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<std::path::PathBuf> for AssetPath {
    fn from(value: std::path::PathBuf) -> Self {
        Self(value.display().to_string())
    }
}

impl From<&std::path::Path> for AssetPath {
    fn from(value: &std::path::Path) -> Self {
        Self(value.display().to_string())
    }
}
