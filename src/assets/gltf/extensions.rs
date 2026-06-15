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
    suggested_fix: &'static str,
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

    pub const fn suggested_fix(&self) -> &'static str {
        self.suggested_fix
    }

    pub const fn decoder_policy(&self) -> GltfDecoderPolicy {
        self.decoder_policy
    }
}

pub(super) fn is_v1_required_gltf_extension(extension: &str) -> bool {
    let built_in = matches!(
        extension,
        "KHR_lights_punctual"
            | "KHR_materials_unlit"
            | "KHR_materials_emissive_strength"
            | "KHR_texture_transform"
            | "KHR_mesh_quantization"
            | "KHR_materials_variants"
            | "EXT_mesh_gpu_instancing"
    );
    built_in
        || (extension == "KHR_texture_basisu" && cfg!(feature = "ktx2"))
        || (extension == "EXT_meshopt_compression" && cfg!(feature = "meshopt"))
}

pub(super) fn collect_extension_diagnostics(
    extensions_used: &[String],
) -> Vec<GltfExtensionDiagnostic> {
    extensions_used
        .iter()
        .filter(|extension| {
            !is_v1_required_gltf_extension(extension)
                || matches!(
                    extension.as_str(),
                    "KHR_texture_basisu"
                        | "KHR_materials_variants"
                        | "EXT_meshopt_compression"
                        | "EXT_mesh_gpu_instancing"
                )
        })
        .map(|extension| GltfExtensionDiagnostic {
            extension: extension.clone(),
            status: optional_extension_status(extension),
            help: optional_extension_help(extension),
            suggested_fix: optional_extension_suggested_fix(extension),
            decoder_policy: optional_extension_decoder_policy(extension),
        })
        .collect()
}

fn optional_extension_status(extension: &str) -> GltfExtensionStatus {
    match extension {
        "KHR_materials_variants" => GltfExtensionStatus::Supported,
        "EXT_mesh_gpu_instancing" => GltfExtensionStatus::Supported,
        "KHR_texture_basisu" if cfg!(feature = "ktx2") => GltfExtensionStatus::Supported,
        "EXT_meshopt_compression" if cfg!(feature = "meshopt") => GltfExtensionStatus::Supported,
        _ => GltfExtensionStatus::Degraded,
    }
}

fn optional_extension_help(extension: &str) -> &'static str {
    match extension {
        "KHR_materials_clearcoat" => {
            "clearcoat factors and texture slots are parsed, and clearcoat/roughness/normal texture channels are wired through CPU/reference and GPU shader paths; structured degradation remains for required usage until approved backend screenshot or readback proof covers the target lane"
        }
        "KHR_materials_sheen" => {
            "sheen material extension factors and color/roughness texture slots are parsed, and CPU/reference plus GPU shader paths sample the same roles; structured degradation remains for required usage until approved backend screenshot or readback proof covers the target lane"
        }
        "KHR_materials_anisotropy" => {
            "anisotropy material extension strength/rotation factors and direction/strength texture slots are parsed, and CPU/reference plus GPU shader paths sample the same roles; structured degradation remains for required usage until approved backend screenshot or readback proof covers the target lane"
        }
        "KHR_materials_iridescence" => {
            "iridescence material extension factor, IOR, thickness range, and factor/thickness texture slots are parsed, and CPU/reference plus GPU shader paths sample the same roles; structured degradation remains for required usage until approved backend screenshot or readback proof covers the target lane"
        }
        "KHR_materials_dispersion" => {
            "dispersion material extension factor is parsed, and CPU/reference plus GPU shader paths apply a channel-spread specular approximation; structured degradation remains for required usage until approved backend screenshot or readback proof covers the target lane"
        }
        "KHR_materials_transmission" | "KHR_materials_ior" | "KHR_materials_volume" => {
            "transmission, IOR, and volume material extension factors and texture slots are parsed, CPU/reference shading uses transmission, thickness, and attenuation, and attached GPU lanes may claim physical glass only when the target capability report has physical_glass_transmission=supported; structured degradation remains for required usage on CPU/reference and unattached factory lanes"
        }
        "KHR_materials_specular" => {
            "specular material extension is optional in this glTF and currently uses structured degradation; required usage fails during asset load"
        }
        "KHR_materials_variants" => {
            "material variants are supported for v1.0: top-level variants and per-primitive mappings are parsed into typed runtime variant selection"
        }
        "EXT_mesh_gpu_instancing" => {
            "EXT_mesh_gpu_instancing is parsed into scene-owned InstanceSet nodes using built-in TRS accessors"
        }
        "EXT_texture_webp" => {
            "WebP texture extension is v1.x-deferred; plain .webp image paths are accepted but EXT_texture_webp texture-source rebinding is not implemented"
        }
        "KHR_texture_basisu" => {
            if cfg!(feature = "ktx2") {
                "KTX2/Basis texture loading is decoder-backed by basisu_c_sys; decodable KTX2/Basis bytes become renderer-visible RGBA8 pixels"
            } else {
                "KTX2/Basis texture loading requires the ktx2 feature and currently uses structured degradation; required usage fails during asset load"
            }
        }
        "KHR_draco_mesh_compression" => {
            "Draco mesh compression requires a future decoder feature and currently uses structured degradation; required usage fails during asset load"
        }
        "EXT_meshopt_compression" => {
            if cfg!(feature = "meshopt") {
                "EXT_meshopt_compression is decoder-backed by the meshopt crate before mesh/material access"
            } else {
                "meshopt compression requires the meshopt feature and currently uses structured degradation; required usage fails during asset load"
            }
        }
        _ => {
            "optional glTF extension is not implemented and currently uses structured degradation; required usage fails during asset load"
        }
    }
}

fn optional_extension_suggested_fix(extension: &str) -> &'static str {
    match extension {
        "KHR_materials_variants" | "EXT_mesh_gpu_instancing" => {
            "No action needed for this extension."
        }
        "KHR_texture_basisu" => {
            if cfg!(feature = "ktx2") {
                "No action needed when the ktx2 or production-assets feature is enabled."
            } else {
                "Enable the production-assets or ktx2 feature, or export PNG/JPEG/WebP fallback textures."
            }
        }
        "EXT_meshopt_compression" => {
            if cfg!(feature = "meshopt") {
                "No action needed when the meshopt or production-assets feature is enabled."
            } else {
                "Enable the production-assets or meshopt feature, or export an uncompressed buffer fallback."
            }
        }
        "KHR_draco_mesh_compression" => {
            "Re-export the asset uncompressed or with EXT_meshopt_compression; revisit Draco when a maintained decoder is adopted."
        }
        "KHR_materials_clearcoat" => {
            "Keep KHR_materials_clearcoat optional unless CPU/reference clearcoat is enough, or export a fallback material without clearcoat for required assets that depend on unproven backend parity."
        }
        "KHR_materials_sheen" => {
            "Keep KHR_materials_sheen optional unless CPU/reference sheen is enough, or export a fallback material without sheen for required assets that depend on unproven backend parity."
        }
        "KHR_materials_anisotropy" => {
            "Keep KHR_materials_anisotropy optional unless CPU/reference anisotropy is enough, or export a fallback material without anisotropy for required assets that depend on unproven backend parity."
        }
        "KHR_materials_iridescence" => {
            "Keep KHR_materials_iridescence optional unless CPU/reference iridescence is enough, or export a fallback material without iridescence for required assets that depend on unproven backend parity."
        }
        "KHR_materials_dispersion" => {
            "Keep KHR_materials_dispersion optional unless CPU/reference dispersion is enough, or export a fallback material without dispersion for required assets that depend on unproven backend parity."
        }
        "KHR_materials_transmission" | "KHR_materials_ior" | "KHR_materials_volume" => {
            "Keep the extension optional unless CPU/reference transmission and volume shading is enough, gate required assets on a target capability report with physical_glass_transmission=supported, or export a fallback material for unsupported lanes."
        }
        "KHR_materials_specular" => {
            "Export a fallback material without the extension, or keep the extension optional until the matching renderer feature is supported."
        }
        "EXT_texture_webp" => {
            "Re-export with PNG/JPEG/WebP image URIs outside EXT_texture_webp, or use KTX2 through the ktx2 feature."
        }
        _ => {
            "Make the extension optional with a visual fallback, or add an explicit scena decoder/support policy before relying on it."
        }
    }
}

fn optional_extension_decoder_policy(extension: &str) -> GltfDecoderPolicy {
    match extension {
        "KHR_texture_basisu" => GltfDecoderPolicy::FeatureFlag {
            feature: "ktx2",
            crate_name: "basisu_c_sys",
            license: "MIT OR Apache-2.0",
        },
        "KHR_draco_mesh_compression" => GltfDecoderPolicy::External {
            feature: "draco",
            crate_name: "draco",
            license: "Apache-2.0-compatible decoder required",
        },
        "EXT_meshopt_compression" => GltfDecoderPolicy::FeatureFlag {
            feature: "meshopt",
            crate_name: "meshopt",
            license: "MIT",
        },
        "KHR_materials_variants" => GltfDecoderPolicy::BuiltIn,
        "EXT_mesh_gpu_instancing" => GltfDecoderPolicy::BuiltIn,
        "KHR_materials_clearcoat"
        | "KHR_materials_transmission"
        | "KHR_materials_ior"
        | "KHR_materials_volume"
        | "KHR_materials_sheen"
        | "KHR_materials_specular"
        | "KHR_materials_iridescence"
        | "KHR_materials_anisotropy"
        | "KHR_materials_dispersion"
        | "EXT_texture_webp" => GltfDecoderPolicy::V1xDeferred,
        _ => GltfDecoderPolicy::V1xDeferred,
    }
}
