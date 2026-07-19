use std::collections::BTreeMap;
use std::future::{Ready, ready};
use std::io::Cursor;
use std::sync::Arc;

use base64::Engine;
use glam::Vec3;
use scena::{
    AssetError, AssetFetcher, AssetPath, Assets, Color, GeometryDesc, MaterialDesc, Renderer,
    Scene, SceneAsset, TextureColorSpace, TextureHandle, Transform,
};

const LOSSLESS_RED_2X2_WEBP_BASE64: &str = "UklGRhwAAABXRUJQVlA4TA8AAAAvAUAAAAcQ/Y/+ByKi/wEA";

#[test]
fn c03_structured_texture_diagnostic_keeps_asset_error_compact() {
    let size = std::mem::size_of::<AssetError>();
    assert!(
        size <= 128,
        "AssetError must stay below Clippy's large Result error threshold; got {size} bytes"
    );
}

#[test]
fn c03_distinct_glb_embedded_images_have_distinct_identity_pixels_provenance_and_output() {
    let red_png = png_rgba8(2, 2, [230, 12, 12, 255]);
    let green_png = png_rgba8(2, 2, [12, 230, 12, 255]);
    let red_path = AssetPath::from("memory://c03/red.glb");
    let green_path = AssetPath::from("memory://c03/green.glb");
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (red_path.clone(), png_texture_glb(&red_png)),
        (green_path.clone(), png_texture_glb(&green_png)),
    ]));

    let red_asset = pollster::block_on(assets.load_scene(red_path)).expect("red GLB loads");
    let green_asset = pollster::block_on(assets.load_scene(green_path)).expect("green GLB loads");
    let red_handle = base_color_texture(&assets, &red_asset);
    let green_handle = base_color_texture(&assets, &green_asset);
    assert_ne!(
        red_handle, green_handle,
        "different embedded bytes at image index 0 must not share a texture handle"
    );

    let red = assets.texture(red_handle).expect("red texture exists");
    let green = assets.texture(green_handle).expect("green texture exists");
    assert_ne!(red.path(), green.path());
    assert!(red.path().as_str().contains("sha256-"));
    assert!(green.path().as_str().contains("sha256-"));
    assert_ne!(
        red.provenance().source_sha256(),
        green.provenance().source_sha256()
    );
    assert_eq!(
        red.decoded_rgba8().expect("red pixels").2,
        red_pixel_bytes()
    );
    assert_eq!(
        green.decoded_rgba8().expect("green pixels").2,
        green_pixel_bytes()
    );

    let red_frame = render_import(&assets, &red_asset);
    let green_frame = render_import(&assets, &green_asset);
    assert_red_dominant(center_pixel(&red_frame, 64, 64));
    assert_green_dominant(center_pixel(&green_frame, 64, 64));
    assert_ne!(red_frame, green_frame);
}

#[cfg(not(feature = "ktx2"))]
#[test]
fn c03_embedded_basis_fallbacks_are_namespaced_and_same_asset_still_deduplicates() {
    let red_png = png_rgba8(2, 2, [230, 12, 12, 255]);
    let green_png = png_rgba8(2, 2, [12, 230, 12, 255]);
    let red_path = AssetPath::from("memory://c03/basis-fallback-red.glb");
    let green_path = AssetPath::from("memory://c03/basis-fallback-green.glb");
    let placeholder_basis = b"descriptor is intentionally not decoded without the ktx2 feature";
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (
            red_path.clone(),
            basis_texture_glb(placeholder_basis, &red_png, MaterialSlot::BaseColor),
        ),
        (
            green_path.clone(),
            basis_texture_glb(placeholder_basis, &green_png, MaterialSlot::BaseColor),
        ),
    ]));

    let red_asset =
        pollster::block_on(assets.load_scene(red_path.clone())).expect("red fallback GLB loads");
    let green_asset =
        pollster::block_on(assets.load_scene(green_path)).expect("green fallback GLB loads");
    let red_handle = base_color_texture(&assets, &red_asset);
    let green_handle = base_color_texture(&assets, &green_asset);
    assert_ne!(red_handle, green_handle);
    assert_eq!(
        assets
            .texture(red_handle)
            .expect("red fallback texture")
            .decoded_rgba8()
            .expect("red fallback pixels")
            .2,
        red_pixel_bytes()
    );
    assert_eq!(
        assets
            .texture(green_handle)
            .expect("green fallback texture")
            .decoded_rgba8()
            .expect("green fallback pixels")
            .2,
        green_pixel_bytes()
    );

    let red_again = pollster::block_on(assets.load_scene(red_path)).expect("cached red GLB loads");
    assert_eq!(
        red_handle,
        base_color_texture(&assets, &red_again),
        "same-asset cache lookup must still deduplicate"
    );
}

#[test]
fn c03_native_webp_decodes_real_pixels_and_renders_the_texture() {
    let path = AssetPath::from("memory://c03/red-lossless.webp");
    let webp = base64::engine::general_purpose::STANDARD
        .decode(LOSSLESS_RED_2X2_WEBP_BASE64)
        .expect("embedded WebP base64 is valid");
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(path.clone(), webp)]));
    let handle = pollster::block_on(assets.load_texture(path.clone(), TextureColorSpace::Srgb))
        .expect("native WebP load succeeds");
    let texture = assets.texture(handle).expect("WebP texture exists");
    assert_eq!(texture.decoded_dimensions(), Some((2, 2)));
    let (_, _, pixels) = texture.decoded_rgba8().expect("WebP pixels decode");
    assert_eq!(pixels, [255, 0, 0, 255].repeat(4));
    assert_eq!(
        handle,
        pollster::block_on(assets.load_texture(path, TextureColorSpace::Srgb))
            .expect("same WebP path deduplicates")
    );

    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.55, 0.55, 0.55));
    let material =
        assets.create_material(MaterialDesc::unlit(Color::WHITE).with_base_color_texture(handle));
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::ZERO))
        .add()
        .expect("WebP textured mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("WebP textured scene prepares");
    renderer.render(&scene, camera).expect("WebP scene renders");
    assert_red_dominant(center_pixel(renderer.frame_rgba8(), 64, 64));
}

#[cfg(all(feature = "ktx2", not(target_arch = "wasm32")))]
#[test]
fn c03_embedded_basis_images_are_content_namespaced_and_same_asset_deduplicates() {
    let red_ktx2 = basisu_ktx2([230, 12, 12, 255], true);
    let green_ktx2 = basisu_ktx2([12, 230, 12, 255], true);
    let fallback = png_rgba8(2, 2, [80, 80, 80, 255]);
    let red_path = AssetPath::from("memory://c03/basis-red.glb");
    let green_path = AssetPath::from("memory://c03/basis-green.glb");
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (
            red_path.clone(),
            basis_texture_glb(&red_ktx2, &fallback, MaterialSlot::BaseColor),
        ),
        (
            green_path.clone(),
            basis_texture_glb(&green_ktx2, &fallback, MaterialSlot::BaseColor),
        ),
    ]));

    let red_asset =
        pollster::block_on(assets.load_scene(red_path.clone())).expect("red Basis GLB loads");
    let green_asset =
        pollster::block_on(assets.load_scene(green_path)).expect("green Basis GLB loads");
    let red = base_color_texture(&assets, &red_asset);
    let green = base_color_texture(&assets, &green_asset);
    assert_ne!(red, green);
    assert_ne!(
        assets
            .texture(red)
            .expect("red Basis texture")
            .provenance()
            .source_sha256(),
        assets
            .texture(green)
            .expect("green Basis texture")
            .provenance()
            .source_sha256()
    );
    assert_red_dominant(center_pixel(&render_import(&assets, &red_asset), 64, 64));
    assert_green_dominant(center_pixel(&render_import(&assets, &green_asset), 64, 64));

    let red_again = pollster::block_on(assets.load_scene(red_path)).expect("cached Basis GLB");
    assert_eq!(red, base_color_texture(&assets, &red_again));
}

#[cfg(all(feature = "ktx2", not(target_arch = "wasm32")))]
#[test]
fn c03_ktx2_accepts_compliant_color_and_non_color_dfd_contracts() {
    let color = basisu_ktx2([230, 12, 12, 255], true);
    let mut non_color = basisu_ktx2([128, 128, 255, 255], false);
    set_ktx2_dfd_color_space(&mut non_color, 0, 1);
    let fallback = png_rgba8(2, 2, [80, 80, 80, 255]);
    let color_path = AssetPath::from("memory://c03/compliant-color.glb");
    let normal_path = AssetPath::from("memory://c03/compliant-normal.glb");
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (
            color_path.clone(),
            basis_texture_glb(&color, &fallback, MaterialSlot::BaseColor),
        ),
        (
            normal_path.clone(),
            basis_texture_glb(&non_color, &fallback, MaterialSlot::Normal),
        ),
    ]));

    pollster::block_on(assets.load_scene(color_path)).expect("BT709+sRGB color DFD is valid");
    pollster::block_on(assets.load_scene(normal_path))
        .expect("unspecified+linear non-color DFD is valid");
}

#[cfg(all(feature = "ktx2", not(target_arch = "wasm32")))]
#[test]
fn c03_ktx2_mismatch_diagnostic_names_slot_dfd_expected_values_and_repair() {
    let srgb = basisu_ktx2([128, 128, 255, 255], true);
    let fallback = png_rgba8(2, 2, [80, 80, 80, 255]);
    let path = AssetPath::from("memory://c03/mismatched-normal.glb");
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        path.clone(),
        basis_texture_glb(&srgb, &fallback, MaterialSlot::Normal),
    )]));

    let error = pollster::block_on(assets.load_scene(path))
        .expect_err("sRGB+BT709 DFD must be rejected for normalTexture");
    let diagnostic = error.to_string();
    for expected in [
        "normalTexture",
        "colorPrimaries=BT709",
        "transferFunction=SRGB",
        "colorPrimaries=UNSPECIFIED",
        "transferFunction=LINEAR",
        "Repair",
    ] {
        assert!(
            diagnostic.contains(expected),
            "diagnostic must contain {expected:?}: {diagnostic}"
        );
    }
}

fn base_color_texture<F: AssetFetcher>(assets: &Assets<F>, asset: &SceneAsset) -> TextureHandle {
    let mesh = asset.nodes()[0].mesh().expect("fixture mesh exists");
    assets
        .material(mesh.material())
        .expect("fixture material exists")
        .base_color_texture()
        .expect("fixture base-color texture exists")
}

fn render_import<F: AssetFetcher>(assets: &Assets<F>, asset: &SceneAsset) -> Vec<u8> {
    let mut scene = Scene::new();
    scene.instantiate(asset).expect("fixture instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("fixture prepares");
    renderer.render(&scene, camera).expect("fixture renders");
    renderer.frame_rgba8().to_vec()
}

fn center_pixel(frame: &[u8], width: u32, height: u32) -> [u8; 4] {
    let offset = (((height / 2) * width + width / 2) * 4) as usize;
    frame[offset..offset + 4]
        .try_into()
        .expect("center pixel has four channels")
}

fn assert_red_dominant(pixel: [u8; 4]) {
    assert!(
        pixel[0] > pixel[1].saturating_add(40) && pixel[0] > pixel[2].saturating_add(40),
        "expected a red-dominant rendered pixel, got {pixel:?}"
    );
}

fn assert_green_dominant(pixel: [u8; 4]) {
    assert!(
        pixel[1] > pixel[0].saturating_add(40) && pixel[1] > pixel[2].saturating_add(40),
        "expected a green-dominant rendered pixel, got {pixel:?}"
    );
}

fn red_pixel_bytes() -> Vec<u8> {
    [230, 12, 12, 255].repeat(4)
}

fn green_pixel_bytes() -> Vec<u8> {
    [12, 230, 12, 255].repeat(4)
}

fn png_rgba8(width: u32, height: u32, pixel: [u8; 4]) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut bytes), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("PNG header writes");
        writer
            .write_image_data(&pixel.repeat((width * height) as usize))
            .expect("PNG pixels write");
    }
    bytes
}

fn png_texture_glb(png: &[u8]) -> Vec<u8> {
    textured_glb(
        vec![("image/png", png)],
        r#"{ "source": 0, "sampler": 0 }"#,
        MaterialSlot::BaseColor,
        false,
    )
}

fn basis_texture_glb(ktx2: &[u8], fallback: &[u8], slot: MaterialSlot) -> Vec<u8> {
    textured_glb(
        vec![("image/ktx2", ktx2), ("image/png", fallback)],
        r#"{
            "source": 1,
            "sampler": 0,
            "extensions": { "KHR_texture_basisu": { "source": 0 } }
        }"#,
        slot,
        true,
    )
}

#[derive(Clone, Copy)]
enum MaterialSlot {
    BaseColor,
    #[cfg(feature = "ktx2")]
    Normal,
}

fn textured_glb(
    images: Vec<(&str, &[u8])>,
    texture_json: &str,
    slot: MaterialSlot,
    uses_basisu: bool,
) -> Vec<u8> {
    let mut bin = Vec::new();
    for value in [-0.65_f32, -0.65, 0.0, 0.65, -0.65, 0.0, 0.0, 0.65, 0.0] {
        bin.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0.0_f32, 0.0, 1.0, 0.0, 0.5, 1.0] {
        bin.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        bin.extend_from_slice(&value.to_le_bytes());
    }
    pad_to_four(&mut bin, 0);

    let mut image_views = Vec::new();
    for (mime, bytes) in images {
        let offset = bin.len();
        bin.extend_from_slice(bytes);
        image_views.push((mime, offset, bytes.len()));
        pad_to_four(&mut bin, 0);
    }
    let image_json = image_views
        .iter()
        .enumerate()
        .map(|(index, (mime, _, _))| {
            format!(r#"{{ "bufferView": {}, "mimeType": "{mime}" }}"#, index + 3)
        })
        .collect::<Vec<_>>()
        .join(",");
    let image_buffer_views = image_views
        .iter()
        .map(|(_, offset, len)| {
            format!(r#"{{ "buffer": 0, "byteOffset": {offset}, "byteLength": {len} }}"#)
        })
        .collect::<Vec<_>>()
        .join(",");
    let material_texture = match slot {
        MaterialSlot::BaseColor => {
            r#""pbrMetallicRoughness": {
                "baseColorTexture": { "index": 0 },
                "metallicFactor": 0.0,
                "roughnessFactor": 1.0
            }"#
        }
        #[cfg(feature = "ktx2")]
        MaterialSlot::Normal => {
            r#""pbrMetallicRoughness": {
                "baseColorFactor": [0.8, 0.8, 0.8, 1.0],
                "metallicFactor": 0.0,
                "roughnessFactor": 1.0
            },
            "normalTexture": { "index": 0 }"#
        }
    };
    let basis_extension = if uses_basisu {
        r#", "KHR_texture_basisu""#
    } else {
        ""
    };
    let buffer_len = bin.len();
    let json = format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "extensionsUsed": ["KHR_materials_unlit"{basis_extension}],
            "buffers": [{{ "byteLength": {buffer_len} }}],
            "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 36, "byteLength": 24 }},
                {{ "buffer": 0, "byteOffset": 60, "byteLength": 6 }},
                {image_buffer_views}
            ],
            "accessors": [
                {{
                    "bufferView": 0,
                    "componentType": 5126,
                    "count": 3,
                    "type": "VEC3",
                    "min": [-0.65, -0.65, 0.0],
                    "max": [0.65, 0.65, 0.0]
                }},
                {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2" }},
                {{ "bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR" }}
            ],
            "images": [{image_json}],
            "samplers": [{{ "magFilter": 9729, "minFilter": 9729 }}],
            "textures": [{texture_json}],
            "materials": [{{
                {material_texture},
                "extensions": {{ "KHR_materials_unlit": {{}} }},
                "doubleSided": true
            }}],
            "meshes": [{{
                "primitives": [{{
                    "attributes": {{ "POSITION": 0, "TEXCOORD_0": 1 }},
                    "indices": 2,
                    "material": 0
                }}]
            }}],
            "nodes": [{{ "name": "C03TexturedTriangle", "mesh": 0 }}]
        }}"#
    );
    build_glb(json, bin)
}

fn build_glb(json: String, mut bin: Vec<u8>) -> Vec<u8> {
    let mut json = json.into_bytes();
    pad_to_four(&mut json, b' ');
    pad_to_four(&mut bin, 0);
    let length = 12 + 8 + json.len() + 8 + bin.len();
    let mut glb = Vec::with_capacity(length);
    glb.extend_from_slice(&0x4654_6C67_u32.to_le_bytes());
    glb.extend_from_slice(&2_u32.to_le_bytes());
    glb.extend_from_slice(&(length as u32).to_le_bytes());
    glb.extend_from_slice(&(json.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x4E4F_534A_u32.to_le_bytes());
    glb.extend_from_slice(&json);
    glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x004E_4942_u32.to_le_bytes());
    glb.extend_from_slice(&bin);
    glb
}

fn pad_to_four(bytes: &mut Vec<u8>, pad: u8) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(pad);
    }
}

#[cfg(all(feature = "ktx2", not(target_arch = "wasm32")))]
fn basisu_ktx2(pixel: [u8; 4], srgb: bool) -> Vec<u8> {
    use basisu_c_sys::BasisTextureFormat;
    use basisu_c_sys::common;
    use basisu_c_sys::extra::{
        BasisuEncoder, BasisuEncoderParams, SourceImage, SourceImageData, basisu_encoder_init,
    };

    pollster::block_on(basisu_encoder_init());
    let mut encoder = BasisuEncoder::new();
    let pixels = pixel.repeat(16);
    encoder
        .set_image(SourceImage {
            data: SourceImageData::Rgba8(&pixels),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
        })
        .expect("C03 pixels are accepted by BasisU");
    let mut flags = common::BU_COMP_FLAGS_KTX2_OUTPUT | common::BU_COMP_FLAGS_TEXTURE_TYPE_2D;
    if srgb {
        flags |= common::BU_COMP_FLAGS_SRGB;
    }
    encoder
        .compress(BasisuEncoderParams {
            basis_tex_format: BasisTextureFormat::UastcLdr4x4,
            quality_level: 75,
            effort_level: 2,
            flags_and_quality: flags,
            low_level_uastc_rdo_or_dct_quality: 0.0,
        })
        .expect("C03 pixels encode as KTX2/BasisU")
}

#[cfg(all(feature = "ktx2", not(target_arch = "wasm32")))]
fn set_ktx2_dfd_color_space(bytes: &mut [u8], primaries: u8, transfer: u8) {
    let dfd_offset =
        u32::from_le_bytes(bytes[48..52].try_into().expect("KTX2 DFD offset")) as usize;
    let basic_dfd = dfd_offset + 4 + 8;
    bytes[basic_dfd + 1] = primaries;
    bytes[basic_dfd + 2] = transfer;
    ktx2::Reader::new(bytes).expect("mutated C03 DFD remains structurally valid");
}

#[derive(Clone)]
struct MemoryFetcher {
    files: Arc<BTreeMap<AssetPath, Vec<u8>>>,
}

impl MemoryFetcher {
    fn new(files: Vec<(AssetPath, Vec<u8>)>) -> Self {
        Self {
            files: Arc::new(files.into_iter().collect()),
        }
    }
}

impl AssetFetcher for MemoryFetcher {
    type Future<'a> = Ready<Result<Vec<u8>, AssetError>>;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        ready(
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| AssetError::NotFound {
                    path: path.as_str().to_string(),
                }),
        )
    }
}
