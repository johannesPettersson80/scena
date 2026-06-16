use std::collections::HashSet;

use crate::assets::{Assets, MaterialHandle, TextureDesc, TextureHandle};
use crate::diagnostics::{Diagnostic, DiagnosticCode};
use crate::material::{MaterialDesc, MaterialKind, TextureTransform};
use crate::scene::Scene;

#[cfg(test)]
#[derive(Debug, Clone)]
pub(in crate::render) struct PreparedBaseColorTexture {
    pub(in crate::render) handle: TextureHandle,
    pub(in crate::render) desc: TextureDesc,
    pub(in crate::render) transform: Option<TextureTransform>,
}

#[derive(Debug, Clone)]
pub(in crate::render) struct PreparedMaterialTexture {
    pub(in crate::render) handle: TextureHandle,
    pub(in crate::render) desc: TextureDesc,
    pub(in crate::render) transform: Option<TextureTransform>,
}

#[derive(Debug, Clone)]
pub(in crate::render) struct PreparedMaterialSlot {
    pub(in crate::render) handle: MaterialHandle,
    pub(in crate::render) material: MaterialDesc,
    pub(in crate::render) base_color: Option<PreparedMaterialTexture>,
    pub(in crate::render) normal: Option<PreparedMaterialTexture>,
    pub(in crate::render) metallic_roughness: Option<PreparedMaterialTexture>,
    pub(in crate::render) occlusion: Option<PreparedMaterialTexture>,
    pub(in crate::render) emissive: Option<PreparedMaterialTexture>,
    pub(in crate::render) clearcoat: Option<PreparedMaterialTexture>,
    pub(in crate::render) clearcoat_roughness: Option<PreparedMaterialTexture>,
    pub(in crate::render) clearcoat_normal: Option<PreparedMaterialTexture>,
    pub(in crate::render) sheen_color: Option<PreparedMaterialTexture>,
    pub(in crate::render) sheen_roughness: Option<PreparedMaterialTexture>,
    pub(in crate::render) anisotropy: Option<PreparedMaterialTexture>,
    pub(in crate::render) iridescence: Option<PreparedMaterialTexture>,
    pub(in crate::render) iridescence_thickness: Option<PreparedMaterialTexture>,
    pub(in crate::render) transmission: Option<PreparedMaterialTexture>,
    pub(in crate::render) thickness: Option<PreparedMaterialTexture>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::render) struct PreparedLogicalResourceStats {
    pub(in crate::render) materials: u64,
    pub(in crate::render) textures: u64,
    pub(in crate::render) material_bindings: u64,
    pub(in crate::render) material_texture_bindings: u64,
    pub(in crate::render) material_sampler_bindings: u64,
    pub(in crate::render) material_textures_missing_decoded_pixels: u64,
    pub(in crate::render) environments: u64,
    pub(in crate::render) live_logical_handles: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct MaterialTextureStats {
    bindings: usize,
    missing_decoded_pixels: usize,
}

pub(in crate::render) fn collect_logical_resource_stats<F>(
    scene: &Scene,
    assets: Option<&Assets<F>>,
    environment_count: u64,
) -> PreparedLogicalResourceStats {
    let mut geometries = HashSet::new();
    let mut materials = HashSet::new();
    let mut textures = HashSet::new();
    let mut material_texture_bindings = 0;
    let mut material_textures_missing_decoded_pixels = 0;

    for (_node, mesh, _transform) in scene.mesh_nodes() {
        geometries.insert(mesh.geometry());
        if materials.insert(mesh.material()) {
            let counts = collect_material_textures(assets, mesh.material(), &mut textures);
            material_texture_bindings += counts.bindings;
            material_textures_missing_decoded_pixels += counts.missing_decoded_pixels;
        }
    }

    for (_node, instance_set, _transform) in scene.instance_set_nodes() {
        geometries.insert(instance_set.geometry());
        if materials.insert(instance_set.material()) {
            let counts = collect_material_textures(assets, instance_set.material(), &mut textures);
            material_texture_bindings += counts.bindings;
            material_textures_missing_decoded_pixels += counts.missing_decoded_pixels;
        }
    }

    let materials = materials.len() as u64;
    let textures = textures.len() as u64;
    let material_texture_bindings = material_texture_bindings as u64;
    let environments = environment_count;
    let live_logical_handles = geometries.len() as u64 + materials + textures + environments;

    PreparedLogicalResourceStats {
        materials,
        textures,
        material_bindings: materials,
        material_texture_bindings,
        material_sampler_bindings: material_texture_bindings,
        material_textures_missing_decoded_pixels: material_textures_missing_decoded_pixels as u64,
        environments,
        live_logical_handles,
    }
}

pub(in crate::render) fn collect_material_texture_diagnostics<F>(
    scene: &Scene,
    assets: &Assets<F>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut materials = HashSet::new();

    for (_node, mesh, _transform) in scene.mesh_nodes() {
        if materials.insert(mesh.material()) {
            collect_material_texture_diagnostics_from_material(
                assets,
                mesh.material(),
                &mut diagnostics,
            );
        }
    }

    for (_node, instance_set, _transform) in scene.instance_set_nodes() {
        if materials.insert(instance_set.material()) {
            collect_material_texture_diagnostics_from_material(
                assets,
                instance_set.material(),
                &mut diagnostics,
            );
        }
    }

    diagnostics
}

#[cfg(test)]
fn collect_primary_base_color_texture<F>(
    scene: &Scene,
    assets: Option<&Assets<F>>,
) -> Option<PreparedBaseColorTexture> {
    let assets = assets?;
    let mut materials = HashSet::new();
    let mut selected_texture: Option<TextureHandle> = None;

    for (_node, mesh, _transform) in scene.mesh_nodes() {
        if materials.insert(mesh.material()) {
            selected_texture = collect_primary_base_color_texture_from_material(
                assets,
                mesh.material(),
                selected_texture,
            )
            .ok()?;
        }
    }

    for (_node, instance_set, _transform) in scene.instance_set_nodes() {
        if materials.insert(instance_set.material()) {
            selected_texture = collect_primary_base_color_texture_from_material(
                assets,
                instance_set.material(),
                selected_texture,
            )
            .ok()?;
        }
    }

    selected_texture.and_then(|handle| {
        assets.texture(handle).map(|desc| PreparedBaseColorTexture {
            handle,
            desc,
            transform: None,
        })
    })
}

#[cfg(test)]
pub(in crate::render) fn collect_backend_base_color_textures<F>(
    scene: &Scene,
    assets: Option<&Assets<F>>,
) -> Vec<PreparedBaseColorTexture> {
    let Some(assets) = assets else {
        return Vec::new();
    };
    let mut materials = HashSet::new();
    let mut textures = HashSet::new();
    let mut selected = Vec::new();

    for (_node, mesh, _transform) in scene.mesh_nodes() {
        if materials.insert(mesh.material()) {
            collect_backend_base_color_texture_from_material(
                assets,
                mesh.material(),
                &mut textures,
                &mut selected,
            );
        }
    }

    for (_node, instance_set, _transform) in scene.instance_set_nodes() {
        if materials.insert(instance_set.material()) {
            collect_backend_base_color_texture_from_material(
                assets,
                instance_set.material(),
                &mut textures,
                &mut selected,
            );
        }
    }

    selected
}

pub(in crate::render) fn collect_backend_material_slots<F>(
    scene: &Scene,
    assets: Option<&Assets<F>>,
) -> Vec<PreparedMaterialSlot> {
    let Some(assets) = assets else {
        return Vec::new();
    };
    let mut materials = HashSet::new();
    let mut selected = Vec::new();

    for (_node, mesh, _transform) in scene.mesh_nodes() {
        if materials.insert(mesh.material())
            && let Some(slot) = collect_backend_material_slot(assets, mesh.material())
        {
            selected.push(slot);
        }
    }

    for (_node, instance_set, _transform) in scene.instance_set_nodes() {
        if materials.insert(instance_set.material())
            && let Some(slot) = collect_backend_material_slot(assets, instance_set.material())
        {
            selected.push(slot);
        }
    }

    selected
}

#[cfg(test)]
fn collect_primary_base_color_texture_from_material<F>(
    assets: &Assets<F>,
    material: crate::assets::MaterialHandle,
    selected_texture: Option<TextureHandle>,
) -> Result<Option<TextureHandle>, ()> {
    let material = assets.material(material).ok_or(())?;
    let Some(texture) = material.base_color_texture() else {
        return Ok(selected_texture);
    };
    if material.base_color_texture_transform().is_some() {
        return Ok(selected_texture);
    }
    let texture_desc = assets.texture(texture).ok_or(())?;
    if !texture_desc.has_decoded_pixels() {
        return Ok(selected_texture);
    }

    match selected_texture {
        Some(selected) if selected != texture => Err(()),
        Some(selected) => Ok(Some(selected)),
        None => Ok(Some(texture)),
    }
}

#[cfg(test)]
fn collect_backend_base_color_texture_from_material<F>(
    assets: &Assets<F>,
    material: crate::assets::MaterialHandle,
    textures: &mut HashSet<TextureHandle>,
    selected: &mut Vec<PreparedBaseColorTexture>,
) {
    let Some(material) = assets.material(material) else {
        return;
    };
    let Some(texture) = material.base_color_texture() else {
        return;
    };
    if !textures.insert(texture) {
        return;
    }
    let Some(desc) = assets.texture(texture) else {
        return;
    };
    if desc.has_decoded_pixels() {
        selected.push(PreparedBaseColorTexture {
            handle: texture,
            desc,
            transform: material.base_color_texture_transform(),
        });
    }
}

fn collect_backend_material_slot<F>(
    assets: &Assets<F>,
    handle: MaterialHandle,
) -> Option<PreparedMaterialSlot> {
    let material = assets.material(handle)?;
    let base_color = collect_backend_material_texture(
        assets,
        material.base_color_texture(),
        material.base_color_texture_transform(),
    );
    let normal = collect_backend_material_texture(
        assets,
        material.normal_texture(),
        material.normal_texture_transform(),
    );
    let metallic_roughness = collect_backend_material_texture(
        assets,
        material.metallic_roughness_texture(),
        material.metallic_roughness_texture_transform(),
    );
    let occlusion = collect_backend_material_texture(
        assets,
        material.occlusion_texture(),
        material.occlusion_texture_transform(),
    );
    let emissive = collect_backend_material_texture(
        assets,
        material.emissive_texture(),
        material.emissive_texture_transform(),
    );
    let clearcoat = collect_backend_material_texture(
        assets,
        material.clearcoat_texture(),
        material.clearcoat_texture_transform(),
    );
    let clearcoat_roughness = collect_backend_material_texture(
        assets,
        material.clearcoat_roughness_texture(),
        material.clearcoat_roughness_texture_transform(),
    );
    let clearcoat_normal = collect_backend_material_texture(
        assets,
        material.clearcoat_normal_texture(),
        material.clearcoat_normal_texture_transform(),
    );
    let sheen_color = collect_backend_material_texture(
        assets,
        material.sheen_color_texture(),
        material.sheen_color_texture_transform(),
    );
    let sheen_roughness = collect_backend_material_texture(
        assets,
        material.sheen_roughness_texture(),
        material.sheen_roughness_texture_transform(),
    );
    let anisotropy = collect_backend_material_texture(
        assets,
        material.anisotropy_texture(),
        material.anisotropy_texture_transform(),
    );
    let iridescence = collect_backend_material_texture(
        assets,
        material.iridescence_texture(),
        material.iridescence_texture_transform(),
    );
    let iridescence_thickness = collect_backend_material_texture(
        assets,
        material.iridescence_thickness_texture(),
        material.iridescence_thickness_texture_transform(),
    );
    let transmission = collect_backend_material_texture(
        assets,
        material.transmission_texture(),
        material.transmission_texture_transform(),
    );
    let thickness = collect_backend_material_texture(
        assets,
        material.thickness_texture(),
        material.thickness_texture_transform(),
    );
    if matches!(material.kind(), MaterialKind::Unlit)
        && base_color.is_none()
        && normal.is_none()
        && metallic_roughness.is_none()
        && occlusion.is_none()
        && emissive.is_none()
        && clearcoat.is_none()
        && clearcoat_roughness.is_none()
        && clearcoat_normal.is_none()
        && sheen_color.is_none()
        && sheen_roughness.is_none()
        && anisotropy.is_none()
        && iridescence.is_none()
        && iridescence_thickness.is_none()
        && transmission.is_none()
        && thickness.is_none()
    {
        return None;
    }
    Some(PreparedMaterialSlot {
        handle,
        base_color,
        normal,
        metallic_roughness,
        occlusion,
        emissive,
        clearcoat,
        clearcoat_roughness,
        clearcoat_normal,
        sheen_color,
        sheen_roughness,
        anisotropy,
        iridescence,
        iridescence_thickness,
        transmission,
        thickness,
        material,
    })
}

fn collect_backend_material_texture<F>(
    assets: &Assets<F>,
    handle: Option<TextureHandle>,
    transform: Option<TextureTransform>,
) -> Option<PreparedMaterialTexture> {
    let handle = handle?;
    let desc = assets.texture(handle)?;
    desc.has_decoded_pixels()
        .then_some(PreparedMaterialTexture {
            handle,
            desc,
            transform,
        })
}

fn collect_material_textures<F>(
    assets: Option<&Assets<F>>,
    material: crate::assets::MaterialHandle,
    textures: &mut HashSet<crate::assets::TextureHandle>,
) -> MaterialTextureStats {
    let Some(assets) = assets else {
        return MaterialTextureStats::default();
    };
    let Some(material) = assets.material(material) else {
        return MaterialTextureStats::default();
    };
    let mut stats = MaterialTextureStats::default();
    for texture in [
        material.base_color_texture(),
        material.normal_texture(),
        material.metallic_roughness_texture(),
        material.occlusion_texture(),
        material.emissive_texture(),
        material.clearcoat_texture(),
        material.clearcoat_roughness_texture(),
        material.clearcoat_normal_texture(),
        material.sheen_color_texture(),
        material.sheen_roughness_texture(),
        material.anisotropy_texture(),
        material.iridescence_texture(),
        material.iridescence_thickness_texture(),
        material.transmission_texture(),
        material.thickness_texture(),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(desc) = assets.texture(texture) {
            textures.insert(texture);
            stats.bindings += 1;
            if !desc.has_decoded_pixels() {
                stats.missing_decoded_pixels += 1;
            }
        }
    }
    stats
}

fn collect_material_texture_diagnostics_from_material<F>(
    assets: &Assets<F>,
    handle: MaterialHandle,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(material) = assets.material(handle) else {
        return;
    };

    for (slot, texture) in material_texture_slots(&material) {
        let Some(texture) = texture else {
            continue;
        };
        let Some(desc) = assets.texture(texture) else {
            continue;
        };
        if desc.has_decoded_pixels() {
            continue;
        }

        diagnostics.push(Diagnostic::warning(
            DiagnosticCode::MaterialTextureMissingDecodedPixels,
            format!(
                "material texture slot {slot} references '{}' but has no decoded pixels",
                desc.path().as_str()
            ),
            "check external glTF image URI rewriting/CSP, inspect \
             AssetLoadWarning::ExternalImageMissing from the asset load report, or use \
             AssetLoadOptions::with_strict_textures(true)",
        ));
    }
}

fn material_texture_slots(material: &MaterialDesc) -> [(&'static str, Option<TextureHandle>); 15] {
    [
        ("base_color", material.base_color_texture()),
        ("normal", material.normal_texture()),
        ("metallic_roughness", material.metallic_roughness_texture()),
        ("occlusion", material.occlusion_texture()),
        ("emissive", material.emissive_texture()),
        ("clearcoat", material.clearcoat_texture()),
        (
            "clearcoat_roughness",
            material.clearcoat_roughness_texture(),
        ),
        ("clearcoat_normal", material.clearcoat_normal_texture()),
        ("sheen_color", material.sheen_color_texture()),
        ("sheen_roughness", material.sheen_roughness_texture()),
        ("anisotropy", material.anisotropy_texture()),
        ("iridescence", material.iridescence_texture()),
        (
            "iridescence_thickness",
            material.iridescence_thickness_texture(),
        ),
        ("transmission", material.transmission_texture()),
        ("thickness", material.thickness_texture()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{AssetPath, Assets, TextureSourceFormat};
    use crate::geometry::GeometryDesc;
    use crate::material::{Color, MaterialDesc, TextureColorSpace, TextureTransform};
    use crate::scene::Scene;

    #[test]
    fn primary_base_color_texture_selection_defers_texture_transforms_to_cpu_bake() {
        let assets = Assets::new();
        let texture = decoded_test_texture(&assets);
        let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.25, 0.25, 0.25));
        let material = assets.create_material(
            MaterialDesc::unlit(Color::WHITE)
                .with_base_color_texture(texture)
                .with_base_color_texture_transform(TextureTransform::new(
                    [0.25, 0.0],
                    0.0,
                    [1.0, 1.0],
                )),
        );
        let mut scene = Scene::new();
        scene.mesh(geometry, material).add().expect("mesh inserts");

        assert!(
            collect_primary_base_color_texture(&scene, Some(&assets)).is_none(),
            "backend upload must not bypass CPU baking for transformed texture coordinates"
        );
    }

    #[test]
    fn primary_base_color_texture_selection_keeps_simple_decoded_texture() {
        let assets = Assets::new();
        let texture = decoded_test_texture(&assets);
        let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.25, 0.25, 0.25));
        let material = assets
            .create_material(MaterialDesc::unlit(Color::WHITE).with_base_color_texture(texture));
        let mut scene = Scene::new();
        scene.mesh(geometry, material).add().expect("mesh inserts");

        let selected = collect_primary_base_color_texture(&scene, Some(&assets))
            .expect("simple decoded texture is backend eligible");
        assert_eq!(selected.handle, texture);
        assert_eq!(selected.desc.decoded_dimensions(), Some((1, 1)));
    }

    #[test]
    fn backend_base_color_texture_selection_keeps_multiple_decoded_textures() {
        let assets = Assets::new();
        let first_texture = decoded_test_texture(&assets);
        let second_texture = decoded_test_texture(&assets);
        let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.25, 0.25, 0.25));
        let first_material = assets.create_material(
            MaterialDesc::unlit(Color::WHITE).with_base_color_texture(first_texture),
        );
        let second_material = assets.create_material(
            MaterialDesc::unlit(Color::WHITE).with_base_color_texture(second_texture),
        );
        let mut scene = Scene::new();
        scene
            .mesh(geometry, first_material)
            .add()
            .expect("first mesh inserts");
        scene
            .mesh(geometry, second_material)
            .add()
            .expect("second mesh inserts");

        let textures = collect_backend_base_color_textures(&scene, Some(&assets));

        assert_eq!(
            textures
                .iter()
                .map(|texture| texture.handle)
                .collect::<Vec<_>>(),
            vec![first_texture, second_texture],
            "backend material slots must preserve all eligible decoded base-color textures in \
             discovery order instead of silently falling back to CPU baking after the first one"
        );
    }

    #[test]
    fn backend_base_color_texture_selection_preserves_texture_transform_uniforms() {
        let assets = Assets::new();
        let texture = decoded_test_texture(&assets);
        let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.25, 0.25, 0.25));
        let transform = TextureTransform::new([0.25, 0.5], 0.5, [0.75, 0.5]);
        let material = assets.create_material(
            MaterialDesc::unlit(Color::WHITE)
                .with_base_color_texture(texture)
                .with_base_color_texture_transform(transform),
        );
        let mut scene = Scene::new();
        scene.mesh(geometry, material).add().expect("mesh inserts");

        let textures = collect_backend_base_color_textures(&scene, Some(&assets));

        assert_eq!(textures.len(), 1);
        assert_eq!(textures[0].handle, texture);
        assert_eq!(
            textures[0].transform,
            Some(transform),
            "backend material slots must preserve KHR_texture_transform metadata so GPU/WebGL2 \
            sampling does not silently use untransformed UVs"
        );
    }

    #[test]
    fn backend_material_slots_skip_unlit_materials_without_decoded_texture_pixels() {
        let assets = Assets::new();
        let missing_texture = assets
            .create_texture_for_test(
                AssetPath::from("textures/missing-albedo.png"),
                TextureColorSpace::Srgb,
                TextureSourceFormat::Png,
                None,
            )
            .expect("missing external texture descriptor inserts without decoded pixels");
        let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.25, 0.25, 0.25));
        let material = assets.create_material(
            MaterialDesc::unlit(Color::from_linear_rgba(0.9, 0.4, 0.1, 1.0))
                .with_base_color_texture(missing_texture),
        );
        let mut scene = Scene::new();
        scene.mesh(geometry, material).add().expect("mesh inserts");

        let slots = collect_backend_material_slots(&scene, Some(&assets));

        assert!(
            slots.is_empty(),
            "unlit materials that have no decoded texture pixels should stay on the CPU-baked \
             path; sending them through a backend material slot adds browser WebGPU risk without \
             adding any material sampling capability"
        );
    }

    #[test]
    fn backend_material_slots_preserve_all_texture_roles_and_material_only_slots() {
        let assets = Assets::new();
        let base_color = decoded_test_texture(&assets);
        let normal = decoded_test_texture(&assets);
        let metallic_roughness = decoded_test_texture(&assets);
        let occlusion = decoded_test_texture(&assets);
        let emissive = decoded_test_texture(&assets);
        let clearcoat = decoded_test_texture(&assets);
        let clearcoat_roughness = decoded_test_texture(&assets);
        let clearcoat_normal = decoded_test_texture(&assets);
        let sheen_color = decoded_test_texture(&assets);
        let sheen_roughness = decoded_test_texture(&assets);
        let anisotropy = decoded_test_texture(&assets);
        let iridescence = decoded_test_texture(&assets);
        let iridescence_thickness = decoded_test_texture(&assets);
        let transmission = decoded_test_texture(&assets);
        let thickness = decoded_test_texture(&assets);
        let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.25, 0.25, 0.25));
        let material_with_textures = assets.create_material(
            MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.25, 0.75)
                .with_base_color_texture(base_color)
                .with_normal_texture(normal)
                .with_metallic_roughness_texture(metallic_roughness)
                .with_occlusion_texture(occlusion)
                .with_emissive_texture(emissive)
                .with_clearcoat_factor(1.0)
                .with_clearcoat_texture(clearcoat)
                .with_clearcoat_roughness_factor(1.0)
                .with_clearcoat_roughness_texture(clearcoat_roughness)
                .with_clearcoat_normal_texture(clearcoat_normal)
                .with_sheen_color_factor(Color::WHITE)
                .with_sheen_color_texture(sheen_color)
                .with_sheen_roughness_factor(1.0)
                .with_sheen_roughness_texture(sheen_roughness)
                .with_anisotropy_strength_factor(1.0)
                .with_anisotropy_texture(anisotropy)
                .with_iridescence_factor(1.0)
                .with_iridescence_texture(iridescence)
                .with_iridescence_thickness_texture(iridescence_thickness)
                .with_transmission_factor(1.0)
                .with_transmission_texture(transmission)
                .with_thickness_factor(1.0)
                .with_thickness_texture(thickness),
        );
        let material_without_textures =
            assets.create_material(MaterialDesc::pbr_metallic_roughness(
                Color::from_linear_rgba(0.2, 0.4, 0.6, 1.0),
                0.5,
                0.25,
            ));
        let mut scene = Scene::new();
        scene
            .mesh(geometry, material_with_textures)
            .add()
            .expect("textured mesh inserts");
        scene
            .mesh(geometry, material_without_textures)
            .add()
            .expect("factor-only mesh inserts");

        let slots = collect_backend_material_slots(&scene, Some(&assets));

        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].handle, material_with_textures);
        assert_eq!(
            slots[0].base_color.as_ref().map(|slot| slot.handle),
            Some(base_color)
        );
        assert_eq!(
            slots[0].normal.as_ref().map(|slot| slot.handle),
            Some(normal)
        );
        assert_eq!(
            slots[0].metallic_roughness.as_ref().map(|slot| slot.handle),
            Some(metallic_roughness)
        );
        assert_eq!(
            slots[0].occlusion.as_ref().map(|slot| slot.handle),
            Some(occlusion)
        );
        assert_eq!(
            slots[0].emissive.as_ref().map(|slot| slot.handle),
            Some(emissive)
        );
        assert_eq!(
            slots[0].clearcoat.as_ref().map(|slot| slot.handle),
            Some(clearcoat)
        );
        assert_eq!(
            slots[0]
                .clearcoat_roughness
                .as_ref()
                .map(|slot| slot.handle),
            Some(clearcoat_roughness)
        );
        assert_eq!(
            slots[0].clearcoat_normal.as_ref().map(|slot| slot.handle),
            Some(clearcoat_normal)
        );
        assert_eq!(
            slots[0].sheen_color.as_ref().map(|slot| slot.handle),
            Some(sheen_color)
        );
        assert_eq!(
            slots[0].sheen_roughness.as_ref().map(|slot| slot.handle),
            Some(sheen_roughness)
        );
        assert_eq!(
            slots[0].anisotropy.as_ref().map(|slot| slot.handle),
            Some(anisotropy)
        );
        assert_eq!(
            slots[0].iridescence.as_ref().map(|slot| slot.handle),
            Some(iridescence)
        );
        assert_eq!(
            slots[0]
                .iridescence_thickness
                .as_ref()
                .map(|slot| slot.handle),
            Some(iridescence_thickness)
        );
        assert_eq!(
            slots[0].transmission.as_ref().map(|slot| slot.handle),
            Some(transmission)
        );
        assert_eq!(
            slots[0].thickness.as_ref().map(|slot| slot.handle),
            Some(thickness)
        );
        assert_eq!(
            slots[1].handle, material_without_textures,
            "backend material slots must include factor-only materials so per-draw uniforms do \
             not collapse to the fallback material slot"
        );
        assert!(slots[1].base_color.is_none());
    }

    fn decoded_test_texture(assets: &Assets) -> crate::assets::TextureHandle {
        assets
            .create_texture_for_test(
                AssetPath::from(
                    "data:image/png;base64,\
                     iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==",
                ),
                TextureColorSpace::Srgb,
                TextureSourceFormat::Png,
                None,
            )
            .expect("inline texture decodes")
    }
}
