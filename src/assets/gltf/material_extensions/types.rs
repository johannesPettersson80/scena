#[derive(Debug, Clone, Copy)]
pub(in crate::assets::gltf) struct DispersionExtension {
    pub(in crate::assets::gltf) factor: f32,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::assets::gltf) struct IorExtension {
    pub(in crate::assets::gltf) ior: f32,
}
