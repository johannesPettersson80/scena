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
            let bytes = tiny_basisu_ktx2_solid_rgba(*rgba, *color_space);
            (AssetPath::from(format!("memory://ktx2-{slot}.ktx2")), bytes)
        })
        .collect::<Vec<_>>();
    let assets = Assets::with_fetcher(MemoryFetcher::new(files.clone()));
    assert_ktx2_normal_texture_affects_cpu_preview_pixels();
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
        assert_material_role_frame(&frame, slot);
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
fn m8_ext_mesh_gpu_instancing_visual_row_writes_release_artifacts() {
    let root = artifact_root();
    fs::create_dir_all(&root).expect("artifact dir");
    let gltf = ext_mesh_gpu_instancing_triangle_gltf();
    let source_sha = sha256_bytes(gltf.as_bytes());
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://ext-mesh-gpu-instancing.gltf"),
        gltf.into_bytes(),
    )]));
    let scene_asset =
        pollster::block_on(assets.load_scene("memory://ext-mesh-gpu-instancing.gltf"))
            .expect("EXT_mesh_gpu_instancing scene loads");
    let mut scene = Scene::new();
    scene
        .instantiate(&scene_asset)
        .expect("instanced scene instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("instanced bounds frame");
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("instanced scene prepares");
    renderer.render(&scene, camera).expect("scene renders");
    let frame = renderer.frame_rgba8().to_vec();
    assert_non_degenerate_frame(&frame, "EXT_mesh_gpu_instancing");
    let ppm_path = root.join("ext-mesh-gpu-instancing.ppm");
    write_ppm(&ppm_path, 64, 64, &frame);
    let ppm_bytes = fs::read(&ppm_path).expect("ppm readable");

    write_json(
        &root.join("ext-mesh-gpu-instancing-visual-proof.json"),
        json!({
            "schema": "scena.compressed_asset_visual_proof.v1",
            "status": "passed",
            "commit_sha": commit_label(),
            "extension": "EXT_mesh_gpu_instancing",
            "source_sha256": source_sha,
            "instance_count": scene_asset.nodes()[0].instance_transforms().len(),
            "artifact": path_string(&ppm_path),
            "artifact_sha256": sha256_bytes(&ppm_bytes),
            "backend": "Headless",
            "evidence_class": "local-instanced-render-proof"
        }),
    );
}

#[test]
fn m8_compressed_native_gpu_lane_records_fail_closed_unavailable_artifact() {
    let root = artifact_root();
    fs::create_dir_all(&root).expect("artifact dir");

    if approved_unstable_headless_gpu_release_lane_enabled() {
        match render_native_gpu_compressed_asset_lane(&root) {
            Ok(artifact) => write_json(&root.join("native-gpu-compressed-lane.json"), artifact),
            Err(reason) => {
                write_compressed_backend_lane_artifact(
                    &root,
                    "native-gpu",
                    "unavailable",
                    false,
                    &format!("approved native GPU compressed-asset run failed: {reason}"),
                );
                write_browser_compressed_lane_placeholders(&root);
                panic!(
                    "native compressed KTX2/meshopt GPU proof cannot be approved on this host: {reason}"
                );
            }
        }
        write_browser_compressed_lane_placeholders(&root);
        return;
    }

    write_compressed_backend_lane_artifact(
        &root,
        "native-gpu",
        match Renderer::headless_gpu(64, 64) {
            Ok(_) => "available-needs-approved-rendered-output-lane",
            Err(_) => "unavailable",
        },
        false,
        "local unit tests must not promote backend availability into release proof; set SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS=1 on an approved native GPU lane",
    );
    write_browser_compressed_lane_placeholders(&root);
}

fn approved_unstable_headless_gpu_release_lane_enabled() -> bool {
    std::env::var_os("SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS").is_some()
}

fn write_browser_compressed_lane_placeholders(root: &Path) {
    for lane in ["browser-webgpu", "browser-webgl2"] {
        write_compressed_backend_lane_artifact(
            root,
            lane,
            "unavailable-local-rust-unit-test",
            false,
            "browser compressed-asset proof must come from a production-assets Playwright lane, not from a native Rust unit test",
        );
    }
}

fn write_compressed_backend_lane_artifact(
    root: &Path,
    lane: &str,
    status: &str,
    release_evidence: bool,
    reason: &str,
) {
    write_json(
        &root.join(format!("{lane}-compressed-lane.json")),
        json!({
            "schema": "scena.compressed_asset_backend_lane.v1",
            "lane": lane,
            "status": status,
            "commit_sha": commit_label(),
            "release_evidence": release_evidence,
            "reason": reason,
        }),
    );
}

fn render_native_gpu_compressed_asset_lane(root: &Path) -> Result<serde_json::Value, String> {
    let ktx2_source = tiny_basisu_ktx2_solid_rgba([230, 48, 32, 255], TextureColorSpace::Srgb);
    let ktx2_path = AssetPath::from("memory://native-gpu-ktx2-base-color.ktx2");
    let ktx2_assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        ktx2_path.clone(),
        ktx2_source.clone(),
    )]));
    let texture =
        pollster::block_on(ktx2_assets.load_texture(ktx2_path.as_str(), TextureColorSpace::Srgb))
            .map_err(|error| format!("KTX2 texture load failed before GPU upload: {error:?}"))?;
    let (ktx2_frame, ktx2_gpu) = render_material_native_gpu(
        &ktx2_assets,
        material_for_slot("base-color", texture),
        "native-gpu-ktx2-base-color",
    )?;
    let ktx2_artifact = root.join("native-gpu-ktx2-base-color.ppm");
    write_ppm(&ktx2_artifact, 64, 64, &ktx2_frame);

    let meshopt_source = meshopt_triangle_gltf("TRIANGLES");
    let meshopt_assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://native-gpu-meshopt-triangles.gltf"),
        meshopt_source.clone().into_bytes(),
    )]));
    let scene_asset =
        pollster::block_on(meshopt_assets.load_scene("memory://native-gpu-meshopt-triangles.gltf"))
            .map_err(|error| format!("meshopt glTF load failed before GPU upload: {error:?}"))?;
    let mut meshopt_scene = Scene::new();
    meshopt_scene
        .instantiate(&scene_asset)
        .map_err(|error| format!("meshopt scene instantiate failed: {error:?}"))?;
    let meshopt_camera = meshopt_scene
        .add_default_camera()
        .map_err(|error| format!("meshopt default camera insert failed: {error:?}"))?;
    let (meshopt_frame, meshopt_gpu) = render_scene_native_gpu(
        &meshopt_assets,
        &mut meshopt_scene,
        meshopt_camera,
        "native-gpu-meshopt-triangles",
    )?;
    let meshopt_artifact = root.join("native-gpu-meshopt-triangles.ppm");
    write_ppm(&meshopt_artifact, 64, 64, &meshopt_frame);

    Ok(json!({
        "schema": "scena.compressed_asset_backend_lane.v1",
        "lane": "native-gpu",
        "status": "passed",
        "commit_sha": commit_label(),
        "release_evidence": true,
        "approved_env": "SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS=1",
        "proof_class": "native-headless-gpu-rendered-output",
        "decoders": {
            "ktx2": "basisu_c_sys KTX2/Basis -> RGBA8 before GPU upload",
            "meshopt": "EXT_meshopt_compression bufferView expansion before GPU upload"
        },
        "rows": [
            {
                "name": "native-gpu-ktx2-base-color",
                "source_path": ktx2_path.as_str(),
                "source_sha256": sha256_bytes(&ktx2_source),
                "artifact": path_string(&ktx2_artifact),
                "artifact_sha256": sha256_bytes(&fs::read(&ktx2_artifact).expect("KTX2 GPU artifact readable")),
                "gpu": ktx2_gpu,
            },
            {
                "name": "native-gpu-meshopt-triangles",
                "source_path": "memory://native-gpu-meshopt-triangles.gltf",
                "source_sha256": sha256_bytes(meshopt_source.as_bytes()),
                "artifact": path_string(&meshopt_artifact),
                "artifact_sha256": sha256_bytes(&fs::read(&meshopt_artifact).expect("meshopt GPU artifact readable")),
                "gpu": meshopt_gpu,
            }
        ],
    }))
}

fn render_material_native_gpu<F: AssetFetcher>(
    assets: &Assets<F>,
    material: MaterialDesc,
    label: &str,
) -> Result<(Vec<u8>, serde_json::Value), String> {
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.72, 0.72, 0.08));
    let material = assets.create_material(material);
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::ZERO))
        .add()
        .map_err(|error| format!("{label} mesh insert failed: {error:?}"))?;
    scene
        .directional_light(DirectionalLight::key_light().with_illuminance_lux(12_000.0))
        .add()
        .map_err(|error| format!("{label} light insert failed: {error:?}"))?;
    let camera = scene
        .add_default_camera()
        .map_err(|error| format!("{label} default camera insert failed: {error:?}"))?;
    render_scene_native_gpu(assets, &mut scene, camera, label)
}

fn render_scene_native_gpu<F: AssetFetcher>(
    assets: &Assets<F>,
    scene: &mut Scene,
    camera: scena::CameraKey,
    label: &str,
) -> Result<(Vec<u8>, serde_json::Value), String> {
    let mut renderer = Renderer::headless_gpu(64, 64)
        .map_err(|error| format!("Renderer::headless_gpu unavailable: {error:?}"))?;
    renderer
        .prepare_with_assets(scene, assets)
        .map_err(|error| format!("{label} native GPU prepare failed: {error:?}"))?;
    let outcome = renderer
        .render(scene, camera)
        .map_err(|error| format!("{label} native GPU render failed: {error:?}"))?;
    let frame = renderer.frame_rgba8().to_vec();
    assert_non_degenerate_frame(&frame, label);
    let capabilities = *renderer.capabilities();
    let stats = renderer.stats();
    Ok((
        frame,
        json!({
            "backend": format!("{:?}", capabilities.backend),
            "gpu_device": capabilities.gpu_device,
            "surface_attached": capabilities.surface_attached,
            "forward_pbr": format!("{:?}", capabilities.forward_pbr),
            "texture_compression_basisu": format!("{:?}", capabilities.texture_compression_basisu),
            "hardware_instancing": format!("{:?}", capabilities.hardware_instancing),
            "readback_headless_screenshots": format!("{:?}", capabilities.readback_headless_screenshots),
            "draw_calls": outcome.draw_calls,
            "primitives": outcome.primitives,
            "material_texture_bindings": stats.material_texture_bindings,
            "material_textures_missing_decoded_pixels": stats.material_textures_missing_decoded_pixels,
            "gpu_submissions": stats.gpu_submissions,
        }),
    ))
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
        .directional_light(DirectionalLight::default().with_illuminance_lux(12_000.0))
        .add()
        .expect("light inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .frame_all_with_assets(camera, assets)
        .expect("material proof geometry frames");
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
    renderer.set_environment(assets.default_environment());
    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("scene prepares");
    renderer.render(&scene, camera).expect("scene renders");
    renderer.frame_rgba8().to_vec()
}

fn assert_material_role_frame(frame: &[u8], label: &str) {
    assert_non_degenerate_frame(frame, label);
}

fn assert_non_degenerate_frame(frame: &[u8], label: &str) {
    assert_visible_frame(frame, label);
    let first = &frame[0..4];
    let distinct = frame
        .chunks_exact(4)
        .filter(|pixel| *pixel != first)
        .take(9)
        .count();
    assert!(distinct > 0, "{label} frame must not be constant");
}

fn assert_visible_frame(frame: &[u8], label: &str) {
    assert_eq!(frame.len(), 64 * 64 * 4);
    let bright = frame
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 16 || pixel[1] > 16 || pixel[2] > 16)
        .count();
    assert!(
        bright > 16,
        "{label} frame must contain visible foreground pixels"
    );
}

fn assert_ktx2_normal_texture_affects_cpu_preview_pixels() {
    let (flat, flat_decoded) = render_frame_for_ktx2_normal_texture([128, 128, 255, 255]);
    let (inverted, inverted_decoded) = render_frame_for_ktx2_normal_texture([128, 128, 0, 255]);
    assert_ne!(
        flat_decoded, inverted_decoded,
        "KTX2 normal fixtures must decode to distinct tangent-space normals"
    );
    let differing_pixels = flat
        .chunks_exact(4)
        .zip(inverted.chunks_exact(4))
        .filter(|(flat, inverted)| flat[..3] != inverted[..3])
        .count();
    let max_channel_delta = flat
        .iter()
        .zip(&inverted)
        .map(|(flat, inverted)| flat.abs_diff(*inverted))
        .max()
        .unwrap_or(0);
    assert!(
        differing_pixels > 16 && max_channel_delta > 8,
        "KTX2 normal texture pixels must materially affect CPU preview lighting instead of being silently ignored; differing_pixels={differing_pixels}, max_channel_delta={max_channel_delta}, decoded flat={flat_decoded:?}, decoded inverted={inverted_decoded:?}"
    );
    let flat_luma = frame_rgb_sum(&flat);
    let inverted_luma = frame_rgb_sum(&inverted);
    assert!(
        flat_luma > inverted_luma,
        "front-facing KTX2 normal map should receive more total directional light than an inverted normal, flat_luma={flat_luma}, inverted_luma={inverted_luma}"
    );
}

fn render_frame_for_ktx2_normal_texture(pixel: [u8; 4]) -> (Vec<u8>, [u8; 3]) {
    let path = format!(
        "memory://ktx2-normal-{}-{}-{}.ktx2",
        pixel[0], pixel[1], pixel[2]
    );
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from(path.clone()),
        tiny_basisu_ktx2_solid_rgba(pixel, TextureColorSpace::Linear),
    )]));
    let normal = pollster::block_on(assets.load_texture(path, TextureColorSpace::Linear))
        .expect("KTX2 normal texture loads");
    let texture = assets
        .texture(normal)
        .expect("KTX2 texture descriptor exists");
    let (_, _, decoded) = texture
        .decoded_rgba8()
        .expect("KTX2 texture has decoded RGBA8 pixels");
    let decoded_rgb = [decoded[0], decoded[1], decoded[2]];
    let frame = render_material(&assets, material_for_slot("normal", normal));
    (frame, decoded_rgb)
}

fn frame_rgb_sum(frame: &[u8]) -> u64 {
    frame
        .chunks_exact(4)
        .map(|pixel| u64::from(pixel[0]) + u64::from(pixel[1]) + u64::from(pixel[2]))
        .sum()
}

fn ext_mesh_gpu_instancing_triangle_gltf() -> String {
    let mut buffer = Vec::with_capacity(60);
    push_vec3_f32(
        &mut buffer,
        [[-0.25, -0.25, 0.0], [0.25, -0.25, 0.0], [0.0, 0.25, 0.0]],
    );
    push_vec3_f32(&mut buffer, [[-0.7, 0.0, 0.0], [0.7, 0.0, 0.0]]);
    let bytes = base64::engine::general_purpose::STANDARD.encode(&buffer);
    let byte_length = buffer.len();

    format!(
        r#"{{
        "asset": {{ "version": "2.0" }},
        "extensionsUsed": ["EXT_mesh_gpu_instancing", "KHR_materials_unlit"],
        "extensionsRequired": ["EXT_mesh_gpu_instancing", "KHR_materials_unlit"],
        "materials": [{{
            "pbrMetallicRoughness": {{ "baseColorFactor": [0.95, 0.2, 0.1, 1.0] }},
            "extensions": {{ "KHR_materials_unlit": {{}} }}
        }}],
        "meshes": [{{
            "primitives": [{{
                "attributes": {{ "POSITION": 0 }},
                "material": 0
            }}]
        }}],
        "nodes": [{{
            "name": "InstancedTriangle",
            "mesh": 0,
            "extensions": {{
                "EXT_mesh_gpu_instancing": {{
                    "attributes": {{ "TRANSLATION": 1 }}
                }}
            }}
        }}],
        "buffers": [{{ "byteLength": {byte_length}, "uri": "data:application/octet-stream;base64,{bytes}" }}],
        "bufferViews": [
            {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
            {{ "buffer": 0, "byteOffset": 36, "byteLength": 24 }}
        ],
        "accessors": [
            {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-0.25,-0.25,0.0], "max": [0.25,0.25,0.0] }},
            {{ "bufferView": 1, "componentType": 5126, "count": 2, "type": "VEC3" }}
        ]
    }}"#
    )
}

fn push_vec3_f32<const N: usize>(buffer: &mut Vec<u8>, values: [[f32; 3]; N]) {
    for vector in values {
        for value in vector {
            buffer.extend_from_slice(&value.to_le_bytes());
        }
    }
}

fn tiny_basisu_ktx2_solid_rgba(pixel: [u8; 4], color_space: TextureColorSpace) -> Vec<u8> {
    tiny_basisu_ktx2_rgba8(&[pixel; 16], color_space)
}

fn tiny_basisu_ktx2_rgba8(
    source_pixels: &[[u8; 4]; 16],
    color_space: TextureColorSpace,
) -> Vec<u8> {
    use basisu_c_sys::BasisTextureFormat;
    use basisu_c_sys::common;
    use basisu_c_sys::extra::{
        BasisuEncoder, BasisuEncoderParams, SourceImage, SourceImageData, basisu_encoder_init,
    };

    pollster::block_on(basisu_encoder_init());
    let mut encoder = BasisuEncoder::new();
    let mut encoded_pixels = Vec::with_capacity(4 * 4 * 4);
    for pixel in source_pixels {
        encoded_pixels.extend_from_slice(pixel);
    }
    encoder
        .set_image(SourceImage {
            data: SourceImageData::Rgba8(&encoded_pixels),
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
        let decoded_stride = if view.semantic == "INDICES" {
            String::new()
        } else {
            format!(",\n                \"byteStride\": {}", view.stride)
        };
        buffer_views.push_str(&format!(
            r#"{{
                "buffer": 0,
                "byteOffset": {decoded_offset},
                "byteLength": {decoded_len}{decoded_stride},
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
