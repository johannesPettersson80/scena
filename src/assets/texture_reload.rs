use super::TextureDesc;
use crate::assets::AssetProvenance;
use crate::diagnostics::AssetError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum TextureCacheUpdatePolicy {
    #[default]
    Immutable,
    ReplaceChangedSource,
}

impl TextureDesc {
    pub(crate) fn replace_changed_source_bytes(
        &mut self,
        source_bytes: &[u8],
    ) -> Result<bool, AssetError> {
        let incoming_provenance =
            AssetProvenance::from_source_bytes(self.path.clone(), source_bytes);
        if self.has_source_payload() && self.provenance == incoming_provenance {
            return Ok(false);
        }
        *self = Self::new_with_bytes(
            self.path.clone(),
            self.color_space,
            self.sampler,
            self.source_format,
            Some(source_bytes),
        )?;
        Ok(true)
    }
}
