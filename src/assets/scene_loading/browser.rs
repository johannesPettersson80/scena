use super::*;

impl<F: AssetFetcher> Assets<F> {
    #[cfg(target_arch = "wasm32")]
    pub(super) async fn decode_browser_texture_images(
        &self,
    ) -> Result<Vec<load::AssetLoadWarning>, AssetError> {
        let requests = {
            let storage = self.storage();
            storage
                .textures
                .iter()
                .filter_map(|(handle, texture)| {
                    texture
                        .browser_decode_source()
                        .map(|bytes| (handle, texture.path().clone(), bytes))
                })
                .collect::<Vec<_>>()
        };

        let mut warnings = Vec::new();
        for (handle, path, bytes) in requests {
            let (image, warning) =
                crate::assets::texture::decode_browser_image_bitmap(&path, bytes).await?;
            if let Some(texture) = self.storage().textures.get_mut(handle) {
                std::sync::Arc::make_mut(texture).set_browser_image(image);
            }
            if let Some(warning) = warning {
                self.storage().texture_warnings.push(warning.clone());
                warnings.push(warning);
            }
        }
        Ok(warnings)
    }
}
