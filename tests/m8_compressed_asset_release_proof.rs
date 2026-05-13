#![cfg(all(feature = "ktx2", feature = "meshopt", not(target_arch = "wasm32")))]

use std::collections::BTreeMap;
use std::fs;
use std::future::{Ready, ready};
use std::path::{Path, PathBuf};

use base64::Engine;
use scena::{
    AssetError, AssetFetcher, AssetPath, Assets, Color, DirectionalLight, GeometryDesc,
    MaterialDesc, Renderer, Scene, TextureColorSpace, Transform, Vec3,
};
use serde_json::json;
use sha2::{Digest, Sha256};

#[test]
fn m8_ktx2_rejects_isolated_invalid_container_and_color_space_cases() {
    let srgb = tiny_basisu_ktx2_solid_rgba([255, 0, 0, 255], TextureColorSpace::Srgb);
    let cases = [
        ("truncated-levels", srgb[..srgb.len() / 2].to_vec()),
        ("zero-level-count", mutate_u32(&srgb, 40, 0)),
        ("unsupported-supercompression", mutate_u32(&srgb, 44, 99)),
        ("unsupported-cubemap-face-count", mutate_u32(&srgb, 36, 6)),
        ("unsupported-dfd-layout", mutate_u32(&srgb, 52, 0)),
    ];

    for (name, bytes) in cases {
        let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
            AssetPath::from(format!("memory://invalid-{name}.ktx2")),
            bytes,
        )]));
        let error = pollster::block_on(assets.load_texture(
            format!("memory://invalid-{name}.ktx2"),
            TextureColorSpace::Srgb,
        ))
        .expect_err("invalid KTX2 must fail closed");
        assert!(
            matches!(error, AssetError::Parse { .. }),
            "{name} must return a structured parse error, got {error:?}"
        );
    }

    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://srgb-authored-normal.ktx2"),
        srgb,
    )]));
    let error = pollster::block_on(assets.load_texture(
        "memory://srgb-authored-normal.ktx2",
        TextureColorSpace::Linear,
    ))
    .expect_err("sRGB-authored KTX2 must not silently load as a linear texture role");
    assert!(
        matches!(error, AssetError::Parse { ref reason, .. }
            if reason.contains("color-space mismatch")),
        "KTX2 color-space mismatch must be explicit, got {error:?}"
    );
}

#[test]
fn m8_ktx2_material_role_visual_rows_write_release_artifacts() {
    let root = artifact_root();
    fs::create_dir_all(&root).expect("artifact dir");
    let textures = [
        ("base-color", [220, 32, 28, 255], TextureColorSpace::Srgb),
        ("normal", [128, 128, 255, 255], TextureColorSpace::Linear),
        (
            "metallic-roughness",
            [0, 178, 220, 255],
            TextureColorSpace::Linear,
        ),
        ("occlusion", [64, 64, 64, 255], TextureColorSpace::Linear),
        ("emissive", [20, 180, 255, 255], TextureColorSpace::Srgb),
    ];
    let files = textures
        .iter()
        .map(|(slot, rgba, color_space)| {
            (
                AssetPath::from(format!("memory://ktx2-{slot}.ktx2")),
                tiny_basisu_ktx2_solid_rgba(*rgba, *color_space),
            )
        })
        .collect::<Vec<_>>();
    let assets = Assets::with_fetcher(MemoryFetcher::new(files.clone()));
    let mut rows = Vec::new();

    for (slot, _, color_space) in textures {
        let path = format!("memory://ktx2-{slot}.ktx2");
        let texture = pollster::block_on(assets.load_texture(path.clone(), color_space))
            .expect("KTX2 role texture loads");
        let texture_desc = assets.texture(texture).expect("texture desc exists");
        let mip_metadata = texture_desc
            .decoded_mip_metadata()
            .expect("KTX2 texture has decoded mip metadata");
        assert_eq!(mip_metadata[0].0, 4);
        assert_eq!(mip_metadata[0].1, 4);

        let material = material_for_slot(slot, texture);
        let frame = render_material(&assets, material);
        assert_non_degenerate_frame(&frame, slot);
        let ppm_path = root.join(format!("ktx2-{slot}.ppm"));
        write_ppm(&ppm_path, 64, 64, &frame);
        let ppm_bytes = fs::read(&ppm_path).expect("ppm readable");
        rows.push(json!({
            "slot": slot,
            "source_path": path,
            "source_sha256": sha256_bytes(&files.iter().find(|(asset_path, _)| asset_path.as_str() == path).expect("source exists").1),
            "decoded_dimensions": [mip_metadata[0].0, mip_metadata[0].1],
            "mip_count": mip_metadata.len(),
            "artifact": path_string(&ppm_path),
            "artifact_sha256": sha256_bytes(&ppm_bytes),
            "backend": "Headless",
            "evidence_class": "local-decoded-rgba8-render-proof"
        }));
    }

    write_json(
        &root.join("ktx2-material-role-visual-proof.json"),
        json!({
            "schema": "scena.compressed_asset_visual_proof.v1",
            "status": "passed",
            "commit_sha": commit_label(),
            "decoder": { "crate": "basisu_c_sys", "mode": "KTX2/Basis -> RGBA8" },
            "rows": rows,
            "release_evidence": "local-cpu-proof-not-native-compressed-gpu-upload"
        }),
    );
}

#[test]
fn m8_meshopt_visual_rows_write_release_artifacts() {
    let root = artifact_root();
    fs::create_dir_all(&root).expect("artifact dir");
    let fixtures = [
        ("meshopt-triangles", meshopt_triangle_gltf("TRIANGLES")),
        ("meshopt-indices", meshopt_triangle_gltf("INDICES")),
        ("meshopt-normals", meshopt_normals_gltf()),
        ("meshopt-tangents", meshopt_tangents_gltf()),
        ("meshopt-quantized", meshopt_quantized_positions_gltf()),
    ];
    let mut rows = Vec::new();

    for (name, gltf) in fixtures {
        let uri = format!("memory://{name}.gltf");
        let source_sha = sha256_bytes(gltf.as_bytes());
        let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
            AssetPath::from(uri.clone()),
            gltf.into_bytes(),
        )]));
        let scene_asset =
            pollster::block_on(assets.load_scene(uri.clone())).expect("meshopt glTF loads");
        let mut scene = Scene::new();
        scene
            .instantiate(&scene_asset)
            .expect("meshopt scene instantiates");
        let camera = scene.add_default_camera().expect("camera inserts");
        let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("scene prepares");
        renderer.render(&scene, camera).expect("scene renders");
        let frame = renderer.frame_rgba8().to_vec();
        assert_non_degenerate_frame(&frame, name);
        let ppm_path = root.join(format!("{name}.ppm"));
        write_ppm(&ppm_path, 64, 64, &frame);
        let ppm_bytes = fs::read(&ppm_path).expect("ppm readable");
        rows.push(json!({
            "name": name,
            "source_sha256": source_sha,
            "artifact": path_string(&ppm_path),
            "artifact_sha256": sha256_bytes(&ppm_bytes),
            "backend": "Headless",
            "evidence_class": "local-meshopt-decoded-render-proof"
        }));
    }

    write_json(
        &root.join("meshopt-visual-proof.json"),
        json!({
            "schema": "scena.compressed_asset_visual_proof.v1",
            "status": "passed",
            "commit_sha": commit_label(),
            "decoder": { "crate": "meshopt", "mode": "EXT_meshopt_compression bufferView expansion" },
            "rows": rows,
            "release_evidence": "local-cpu-proof-not-native-backend-proof"
        }),
    );
}

#[test]
fn m8_compressed_native_gpu_lane_records_fail_closed_unavailable_artifact() {
    let root = artifact_root();
    fs::create_dir_all(&root).expect("artifact dir");
    let lanes = [
        (
            "native-gpu",
            match Renderer::headless_gpu(64, 64) {
                Ok(_) => "available-needs-dedicated-rendered-output-lane",
                Err(_) => "unavailable",
            },
        ),
        ("browser-webgpu", "unavailable-local-rust-unit-test"),
        ("browser-webgl2", "unavailable-local-rust-unit-test"),
    ];

    for (lane, status) in lanes {
        write_json(
            &root.join(format!("{lane}-compressed-lane.json")),
            json!({
                "schema": "scena.compressed_asset_backend_lane.v1",
                "lane": lane,
                "status": status,
                "commit_sha": commit_label(),
                "release_evidence": false,
                "reason": "local unit tests must not promote backend availability or ignored tests into release proof"
            }),
        );
        assert_ne!(
            status, "passed",
            "{lane} availability artifact must not masquerade as release proof"
        );
    }
}

fn material_for_slot(slot: &str, texture: scena::TextureHandle) -> MaterialDesc {
    match slot {
        "base-color" => MaterialDesc::unlit(Color::WHITE).with_base_color_texture(texture),
        "normal" => {
            MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(190, 190, 190), 0.0, 0.75)
                .with_normal_texture(texture)
        }
        "metallic-roughness" => {
            MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(190, 190, 190), 1.0, 1.0)
                .with_metallic_roughness_texture(texture)
        }
        "occlusion" => {
            MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(190, 190, 190), 0.0, 0.65)
                .with_occlusion_texture(texture)
        }
        "emissive" => MaterialDesc::unlit(Color::BLACK)
            .with_emissive(Color::WHITE)
            .with_emissive_strength(2.0)
            .with_emissive_texture(texture),
        _ => unreachable!("unknown material slot {slot}"),
    }
    .with_double_sided(true)
}

fn render_material<F: AssetFetcher>(assets: &Assets<F>, material: MaterialDesc) -> Vec<u8> {
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.72, 0.72, 0.08));
    let material = assets.create_material(material);
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::ZERO))
        .add()
        .expect("mesh inserts");
    scene
        .directional_light(DirectionalLight::default().with_illuminance_lux(1.0))
        .add()
        .expect("light inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("scene prepares");
    renderer.render(&scene, camera).expect("scene renders");
    renderer.frame_rgba8().to_vec()
}

fn assert_non_degenerate_frame(frame: &[u8], label: &str) {
    assert_eq!(frame.len(), 64 * 64 * 4);
    let first = &frame[0..4];
    let distinct = frame
        .chunks_exact(4)
        .filter(|pixel| *pixel != first)
        .take(9)
        .count();
    let bright = frame
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 16 || pixel[1] > 16 || pixel[2] > 16)
        .count();
    assert!(distinct > 0, "{label} frame must not be constant");
    assert!(
        bright > 16,
        "{label} frame must contain visible foreground pixels"
    );
}

fn tiny_basisu_ktx2_solid_rgba(pixel: [u8; 4], color_space: TextureColorSpace) -> Vec<u8> {
    use basisu_c_sys::BasisTextureFormat;
    use basisu_c_sys::common;
    use basisu_c_sys::extra::{
        BasisuEncoder, BasisuEncoderParams, SourceImage, SourceImageData, basisu_encoder_init,
    };

    pollster::block_on(basisu_encoder_init());
    let mut encoder = BasisuEncoder::new();
    let mut pixels = Vec::with_capacity(4 * 4 * 4);
    for _ in 0..16 {
        pixels.extend_from_slice(&pixel);
    }
    encoder
        .set_image(SourceImage {
            data: SourceImageData::Rgba8(&pixels),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
        })
        .expect("solid image is accepted by the Basis Universal encoder");
    let srgb_flag = if color_space == TextureColorSpace::Srgb {
        common::BU_COMP_FLAGS_SRGB
    } else {
        0
    };
    encoder
        .compress(BasisuEncoderParams {
            basis_tex_format: BasisTextureFormat::UastcLdr4x4,
            quality_level: 75,
            effort_level: 2,
            flags_and_quality: srgb_flag
                | common::BU_COMP_FLAGS_KTX2_OUTPUT
                | common::BU_COMP_FLAGS_TEXTURE_TYPE_2D,
            low_level_uastc_rdo_or_dct_quality: 0.0,
        })
        .expect("solid texture compresses to a KTX2/Basis Universal payload")
}

fn mutate_u32(bytes: &[u8], offset: usize, value: u32) -> Vec<u8> {
    let mut mutated = bytes.to_vec();
    mutated[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    mutated
}

fn meshopt_triangle_gltf(index_mode: &'static str) -> String {
    let positions = [[-0.5_f32, -0.5, 0.0], [0.5, -0.5, 0.0], [-0.5, 0.5, 0.0]];
    let indices = [0_u32, 1, 2];
    let compressed_positions =
        meshopt::encode_vertex_buffer(&positions).expect("positions meshopt-encode");
    let compressed_indices = if index_mode == "INDICES" {
        meshopt_encode_index_sequence(&indices, positions.len())
    } else {
        meshopt::encode_index_buffer(&indices, positions.len()).expect("indices meshopt-encode")
    };
    meshopt_gltf_from_views(vec![
        MeshoptView::new(
            "POSITION",
            5126,
            "VEC3",
            12,
            36,
            compressed_positions,
            "ATTRIBUTES",
        ),
        MeshoptView::new(
            "INDICES",
            5123,
            "SCALAR",
            2,
            6,
            compressed_indices,
            index_mode,
        ),
    ])
}

fn meshopt_normals_gltf() -> String {
    let positions = [[-0.5_f32, -0.5, 0.0], [0.5, -0.5, 0.0], [-0.5, 0.5, 0.0]];
    let normals = [[0.0_f32, 0.0, 1.0]; 3];
    let indices = [0_u32, 1, 2];
    meshopt_gltf_from_views(vec![
        MeshoptView::new(
            "POSITION",
            5126,
            "VEC3",
            12,
            36,
            meshopt::encode_vertex_buffer(&positions).expect("positions encode"),
            "ATTRIBUTES",
        ),
        MeshoptView::new(
            "NORMAL",
            5126,
            "VEC3",
            12,
            36,
            meshopt::encode_vertex_buffer(&normals).expect("normals encode"),
            "ATTRIBUTES",
        ),
        MeshoptView::new(
            "INDICES",
            5123,
            "SCALAR",
            2,
            6,
            meshopt::encode_index_buffer(&indices, positions.len()).expect("indices encode"),
            "TRIANGLES",
        ),
    ])
}

fn meshopt_tangents_gltf() -> String {
    let positions = [[-0.5_f32, -0.5, 0.0], [0.5, -0.5, 0.0], [-0.5, 0.5, 0.0]];
    let tangents = [[1.0_f32, 0.0, 0.0, 1.0]; 3];
    let indices = [0_u32, 1, 2];
    meshopt_gltf_from_views(vec![
        MeshoptView::new(
            "POSITION",
            5126,
            "VEC3",
            12,
            36,
            meshopt::encode_vertex_buffer(&positions).expect("positions encode"),
            "ATTRIBUTES",
        ),
        MeshoptView::new(
            "TANGENT",
            5126,
            "VEC4",
            16,
            48,
            meshopt::encode_vertex_buffer(&tangents).expect("tangents encode"),
            "ATTRIBUTES",
        ),
        MeshoptView::new(
            "INDICES",
            5123,
            "SCALAR",
            2,
            6,
            meshopt::encode_index_buffer(&indices, positions.len()).expect("indices encode"),
            "TRIANGLES",
        ),
    ])
}

fn meshopt_quantized_positions_gltf() -> String {
    let positions = [
        [-16384_i16, -16384, 0, 0],
        [16384, -16384, 0, 0],
        [-16384, 16384, 0, 0],
    ];
    let indices = [0_u32, 1, 2];
    meshopt_gltf_from_views(vec![
        MeshoptView::new(
            "POSITION",
            5122,
            "VEC3",
            8,
            24,
            meshopt::encode_vertex_buffer(&positions).expect("quantized positions encode"),
            "ATTRIBUTES",
        )
        .normalized(true)
        .min_max("[-0.5,-0.5,0.0]", "[0.5,0.5,0.0]"),
        MeshoptView::new(
            "INDICES",
            5123,
            "SCALAR",
            2,
            6,
            meshopt::encode_index_buffer(&indices, positions.len()).expect("indices encode"),
            "TRIANGLES",
        ),
    ])
}

fn meshopt_encode_index_sequence(indices: &[u32], vertex_count: usize) -> Vec<u8> {
    let bound =
        unsafe { meshopt::ffi::meshopt_encodeIndexSequenceBound(indices.len(), vertex_count) };
    let mut result = vec![0; bound];
    let size = unsafe {
        meshopt::ffi::meshopt_encodeIndexSequence(
            result.as_mut_ptr(),
            result.len(),
            indices.as_ptr(),
            indices.len(),
        )
    };
    assert!(size > 0, "meshopt index-sequence encoding succeeds");
    result.truncate(size);
    result
}

#[derive(Clone)]
struct MeshoptView {
    semantic: &'static str,
    component_type: u32,
    accessor_type: &'static str,
    stride: usize,
    decoded_len: usize,
    encoded: Vec<u8>,
    mode: &'static str,
    normalized: bool,
    min: &'static str,
    max: &'static str,
}

impl MeshoptView {
    fn new(
        semantic: &'static str,
        component_type: u32,
        accessor_type: &'static str,
        stride: usize,
        decoded_len: usize,
        encoded: Vec<u8>,
        mode: &'static str,
    ) -> Self {
        Self {
            semantic,
            component_type,
            accessor_type,
            stride,
            decoded_len,
            encoded,
            mode,
            normalized: false,
            min: "[-0.5,-0.5,0.0]",
            max: "[0.5,0.5,0.0]",
        }
    }

    fn normalized(mut self, normalized: bool) -> Self {
        self.normalized = normalized;
        self
    }

    fn min_max(mut self, min: &'static str, max: &'static str) -> Self {
        self.min = min;
        self.max = max;
        self
    }
}

fn meshopt_gltf_from_views(views: Vec<MeshoptView>) -> String {
    let decoded_len = views.iter().map(|view| view.decoded_len).sum::<usize>();
    let mut encoded = Vec::new();
    let mut decoded_offset = 0usize;
    let mut encoded_offset = 0usize;
    let mut buffer_views = String::new();
    let mut accessors = String::new();
    let mut attributes = String::new();
    let mut index_accessor = 0usize;

    for (index, view) in views.iter().enumerate() {
        let encoded_len = view.encoded.len();
        encoded.extend_from_slice(&view.encoded);
        buffer_views.push_str(&format!(
            r#"{{
                "buffer": 0,
                "byteOffset": {decoded_offset},
                "byteLength": {decoded_len},
                "byteStride": {stride},
                "extensions": {{
                    "EXT_meshopt_compression": {{
                        "buffer": 1,
                        "byteOffset": {encoded_offset},
                        "byteLength": {encoded_len},
                        "byteStride": {stride},
                        "count": 3,
                        "mode": "{mode}",
                        "filter": "NONE"
                    }}
                }}
            }}"#,
            decoded_len = view.decoded_len,
            stride = view.stride,
            mode = view.mode,
        ));
        let normalized = if view.normalized {
            r#", "normalized": true"#
        } else {
            ""
        };
        accessors.push_str(&format!(
            r#"{{ "bufferView": {index}, "componentType": {component_type}, "count": 3, "type": "{accessor_type}", "min": {min}, "max": {max}{normalized} }}"#,
            component_type = view.component_type,
            accessor_type = view.accessor_type,
            min = view.min,
            max = view.max,
        ));
        if view.semantic == "INDICES" {
            index_accessor = index;
        } else {
            if !attributes.is_empty() {
                attributes.push_str(", ");
            }
            attributes.push_str(&format!(r#""{}": {index}"#, view.semantic));
        }
        decoded_offset += view.decoded_len;
        encoded_offset += encoded_len;
        if index + 1 != views.len() {
            buffer_views.push_str(",\n");
            accessors.push_str(",\n");
        }
    }

    let decoded_uri = base64::engine::general_purpose::STANDARD.encode(vec![0_u8; decoded_len]);
    let encoded_uri = base64::engine::general_purpose::STANDARD.encode(encoded);
    format!(
        r#"{{
        "asset": {{ "version": "2.0" }},
        "extensionsUsed": ["EXT_meshopt_compression", "KHR_mesh_quantization"],
        "extensionsRequired": ["EXT_meshopt_compression"],
        "materials": [{{ "pbrMetallicRoughness": {{ "baseColorFactor": [0.75, 0.75, 0.75, 1.0] }} }}],
        "meshes": [{{
            "primitives": [{{
                "attributes": {{ {attributes} }},
                "indices": {index_accessor},
                "material": 0
            }}]
        }}],
        "nodes": [{{ "name": "MeshoptRoot", "mesh": 0 }}],
        "buffers": [
            {{ "byteLength": {decoded_len}, "uri": "data:application/octet-stream;base64,{decoded_uri}" }},
            {{ "byteLength": {encoded_len}, "uri": "data:application/octet-stream;base64,{encoded_uri}" }}
        ],
        "bufferViews": [{buffer_views}],
        "accessors": [{accessors}]
    }}"#,
        encoded_len = encoded_offset,
    )
}

fn write_ppm(path: &Path, width: u32, height: u32, rgba: &[u8]) {
    let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(path, ppm).expect("ppm writes");
}

fn write_json(path: &Path, value: serde_json::Value) {
    fs::write(
        path,
        serde_json::to_vec_pretty(&value).expect("json serializes"),
    )
    .expect("json writes");
}

fn sha256_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn commit_label() -> String {
    std::env::var("GITHUB_SHA").unwrap_or_else(|_| "local-checkout".to_string())
}

fn artifact_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/m8-compressed-assets")
}

#[derive(Clone)]
struct MemoryFetcher {
    files: BTreeMap<AssetPath, Vec<u8>>,
}

impl MemoryFetcher {
    fn new(files: Vec<(AssetPath, Vec<u8>)>) -> Self {
        Self {
            files: files.into_iter().collect(),
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
