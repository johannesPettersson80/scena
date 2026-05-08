#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GltfExtensionStatus {
    Supported,
    Degraded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GltfDecoderPolicy {
    BuiltIn,
    FeatureFlag {
        feature: &'static str,
        crate_name: &'static str,
        license: &'static str,
    },
    External {
        feature: &'static str,
        crate_name: &'static str,
        license: &'static str,
    },
    V1xDeferred,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GltfExtensionDiagnostic {
    extension: String,
    status: GltfExtensionStatus,
    help: &'static str,
    decoder_policy: GltfDecoderPolicy,
}

impl GltfExtensionDiagnostic {
    pub fn extension(&self) -> &str {
        &self.extension
    }

    pub const fn status(&self) -> GltfExtensionStatus {
        self.status
    }

    pub const fn help(&self) -> &'static str {
        self.help
    }

    pub const fn decoder_policy(&self) -> GltfDecoderPolicy {
        self.decoder_policy
    }
}

pub(super) fn is_v1_required_gltf_extension(extension: &str) -> bool {
    matches!(
        extension,
        "KHR_lights_punctual"
            | "KHR_materials_unlit"
            | "KHR_materials_emissive_strength"
            | "KHR_texture_transform"
            | "KHR_mesh_quantization"
    ) || (cfg!(feature = "ktx2") && extension == "KHR_texture_basisu")
}

pub(super) fn collect_extension_diagnostics(
    extensions_used: &[String],
) -> Vec<GltfExtensionDiagnostic> {
    extensions_used
        .iter()
        .filter(|extension| {
            !is_v1_required_gltf_extension(extension) || extension.as_str() == "KHR_texture_basisu"
        })
        .map(|extension| GltfExtensionDiagnostic {
            extension: extension.clone(),
            status: optional_extension_status(extension),
            help: optional_extension_help(extension),
            decoder_policy: optional_extension_decoder_policy(extension),
        })
        .collect()
}

fn optional_extension_status(extension: &str) -> GltfExtensionStatus {
    if cfg!(feature = "ktx2") && extension == "KHR_texture_basisu" {
        GltfExtensionStatus::Supported
    } else {
        GltfExtensionStatus::Degraded
    }
}

fn optional_extension_help(extension: &str) -> &'static str {
    match extension {
        "KHR_materials_clearcoat"
        | "KHR_materials_transmission"
        | "KHR_materials_ior"
        | "KHR_materials_volume"
        | "KHR_materials_sheen"
        | "KHR_materials_specular"
        | "KHR_materials_iridescence" => {
            "material extension is optional in this glTF and currently uses structured degradation; required usage fails during asset load"
        }
        "KHR_materials_variants" => {
            "material variants are v1.x-deferred and currently use structured degradation; metadata remains visible through extensions_used"
        }
        "EXT_texture_webp" => {
            "WebP texture extension is v1.x-deferred; plain .webp image paths are accepted but EXT_texture_webp texture-source rebinding is not implemented"
        }
        "KHR_texture_basisu" => {
            if cfg!(feature = "ktx2") {
                "KTX2/Basis texture descriptor loading is enabled by the ktx2 feature"
            } else {
                "KTX2/Basis texture loading requires the ktx2 feature and currently uses structured degradation; required usage fails during asset load"
            }
        }
        "KHR_draco_mesh_compression" => {
            "Draco mesh compression requires a future decoder feature and currently uses structured degradation; required usage fails during asset load"
        }
        "EXT_meshopt_compression" => {
            "meshopt compression requires a future decoder feature and currently uses structured degradation; required usage fails during asset load"
        }
        _ => {
            "optional glTF extension is not implemented and currently uses structured degradation; required usage fails during asset load"
        }
    }
}

fn optional_extension_decoder_policy(extension: &str) -> GltfDecoderPolicy {
    match extension {
        "KHR_texture_basisu" => GltfDecoderPolicy::FeatureFlag {
            feature: "ktx2",
            crate_name: "basis-universal",
            license: "Apache-2.0 OR MIT-compatible decoder required",
        },
        "KHR_draco_mesh_compression" => GltfDecoderPolicy::External {
            feature: "draco",
            crate_name: "draco",
            license: "Apache-2.0-compatible decoder required",
        },
        "EXT_meshopt_compression" => GltfDecoderPolicy::External {
            feature: "meshopt",
            crate_name: "meshopt",
            license: "MIT-compatible decoder required",
        },
        "KHR_materials_clearcoat"
        | "KHR_materials_transmission"
        | "KHR_materials_ior"
        | "KHR_materials_volume"
        | "KHR_materials_sheen"
        | "KHR_materials_specular"
        | "KHR_materials_iridescence"
        | "KHR_materials_variants"
        | "EXT_texture_webp" => GltfDecoderPolicy::V1xDeferred,
        _ => GltfDecoderPolicy::V1xDeferred,
    }
}
