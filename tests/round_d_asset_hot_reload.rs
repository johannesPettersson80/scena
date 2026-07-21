#![cfg(all(feature = "hot-reload", not(target_arch = "wasm32")))]

use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use base64::Engine as _;
use scena::{Assets, Renderer, RetainPolicy, Scene};

const WIDTH: u32 = 64;
const HEIGHT: u32 = 64;

#[test]
fn reload_scene_replaces_changed_external_texture_at_the_same_path() {
    let dir = std::env::temp_dir().join(format!(
        "scena-c09-external-texture-reload-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("C09 temp directory creates");
    let scene_path = dir.join("scene.gltf");
    let texture_path = dir.join("mutable.png");
    write_external_texture_scene(&scene_path, "mutable.png");
    write_solid_png(&texture_path, [230, 20, 30, 255]);

    let mut assets = Assets::new();
    assets.set_retain_policy(RetainPolicy::Always);
    let first = pollster::block_on(assets.load_scene(path_string(&scene_path)))
        .expect("initial external-texture scene loads");
    let first_material = assets
        .material(
            first.nodes()[0]
                .mesh()
                .expect("initial mesh exists")
                .material(),
        )
        .expect("initial material exists");
    let texture = first_material
        .base_color_texture()
        .expect("initial texture handle exists");
    assert_eq!(
        assets
            .texture(texture)
            .expect("initial texture descriptor exists")
            .decoded_rgba8()
            .expect("initial texture pixels decode")
            .2,
        &[230, 20, 30, 255]
    );

    write_solid_png(&texture_path, [20, 220, 40, 255]);
    let report = pollster::block_on(assets.reload_scene_with_report(&first))
        .expect("changed external texture reloads transactionally");
    assert!(!report.cache_hit());
    assert_eq!(report.external_images(), 1);
    let reloaded = report.asset();
    let reloaded_material = assets
        .material(
            reloaded.nodes()[0]
                .mesh()
                .expect("reloaded mesh exists")
                .material(),
        )
        .expect("reloaded material exists");
    let reloaded_texture = reloaded_material
        .base_color_texture()
        .expect("reloaded texture handle exists");
    assert_eq!(
        reloaded_texture, texture,
        "explicit reload preserves the cache handle while updating its source version"
    );
    assert_eq!(
        assets
            .texture(reloaded_texture)
            .expect("reloaded texture descriptor exists")
            .decoded_rgba8()
            .expect("reloaded texture pixels decode")
            .2,
        &[20, 220, 40, 255],
        "same-path reload must expose the new decoded pixels"
    );

    let unchanged_report = pollster::block_on(assets.reload_scene_with_report(reloaded))
        .expect("same-byte external texture reload is a successful no-op");
    let unchanged_texture = assets
        .material(
            unchanged_report.asset().nodes()[0]
                .mesh()
                .expect("same-byte mesh exists")
                .material(),
        )
        .expect("same-byte material exists")
        .base_color_texture()
        .expect("same-byte texture handle exists");
    assert_eq!(unchanged_texture, texture);

    std::fs::write(&texture_path, b"not a PNG").expect("invalid replacement bytes write");
    let failed_report =
        pollster::block_on(assets.reload_scene_with_report(unchanged_report.asset()))
            .expect_err("invalid replacement texture must fail the reload transaction");
    assert_eq!(failed_report.path().as_str(), path_string(&scene_path));
    assert!(failed_report.previous_asset_preserved());
    assert!(matches!(
        failed_report.error(),
        scena::AssetError::Parse { .. }
    ));
    assert_eq!(
        assets
            .texture(texture)
            .expect("last complete texture survives failed decode")
            .decoded_rgba8()
            .expect("last complete texture pixels survive")
            .2,
        &[20, 220, 40, 255]
    );

    std::fs::remove_file(&texture_path).expect("external texture removes");
    let deleted_error = pollster::block_on(assets.reload_scene(unchanged_report.asset()))
        .expect_err("deleted external texture must fail closed");
    assert_asset_error_path(&deleted_error, &texture_path);
    let cached = pollster::block_on(assets.load_scene(path_string(&scene_path)))
        .expect("last complete scene remains cached after failed reload");
    let cached_texture = assets
        .material(
            cached.nodes()[0]
                .mesh()
                .expect("cached complete mesh exists")
                .material(),
        )
        .expect("cached complete material exists")
        .base_color_texture()
        .expect("cached complete texture exists");
    assert_eq!(cached_texture, texture);

    std::fs::remove_dir_all(&dir).expect("C09 temp directory removes");
}

#[test]
fn reload_scene_updates_every_shared_texture_consumer_once() {
    let dir = std::env::temp_dir().join(format!(
        "scena-c09-shared-texture-reload-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("shared-texture temp directory creates");
    let scene_path = dir.join("scene.gltf");
    let texture_path = dir.join("shared.png");
    write_shared_external_texture_scene(&scene_path, "shared.png");
    write_solid_png(&texture_path, [190, 30, 40, 255]);

    let mut assets = Assets::new();
    assets.set_retain_policy(RetainPolicy::Always);
    let first = pollster::block_on(assets.load_scene(path_string(&scene_path)))
        .expect("shared-texture scene loads");
    let first_textures = scene_base_color_textures(&assets, &first);
    assert!(first_textures.len() >= 2, "both material consumers load");
    assert!(
        first_textures
            .iter()
            .all(|handle| *handle == first_textures[0]),
        "same path/configuration must deduplicate to one shared handle: {first_textures:?}"
    );

    write_solid_png(&texture_path, [25, 205, 55, 255]);
    let reloaded = pollster::block_on(assets.reload_scene(&first))
        .expect("shared external texture reload succeeds");
    let reloaded_textures = scene_base_color_textures(&assets, &reloaded);
    assert_eq!(reloaded_textures, first_textures);
    assert_eq!(
        assets
            .texture(first_textures[0])
            .expect("shared descriptor exists after reload")
            .decoded_rgba8()
            .expect("shared replacement pixels decode")
            .2,
        &[25, 205, 55, 255]
    );

    std::fs::remove_dir_all(&dir).expect("shared-texture temp directory removes");
}

#[test]
fn ordinary_load_keeps_texture_provenance_immutable_until_explicit_reload() {
    let dir = std::env::temp_dir().join(format!(
        "scena-c09-explicit-reload-boundary-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("explicit-boundary temp directory creates");
    let first_scene_path = dir.join("first.gltf");
    let second_scene_path = dir.join("second.gltf");
    let texture_path = dir.join("shared.png");
    write_external_texture_scene(&first_scene_path, "shared.png");
    write_external_texture_scene(&second_scene_path, "shared.png");
    write_solid_png(&texture_path, [200, 35, 45, 255]);

    let mut assets = Assets::new();
    assets.set_retain_policy(RetainPolicy::Always);
    let first = pollster::block_on(assets.load_scene(path_string(&first_scene_path)))
        .expect("first scene revision loads");
    let texture = scene_base_color_textures(&assets, &first)[0];

    write_solid_png(&texture_path, [35, 200, 55, 255]);
    let ordinary_error = pollster::block_on(assets.load_scene(path_string(&second_scene_path)))
        .expect_err("ordinary load must not mutate an existing cache identity");
    assert!(
        matches!(
            ordinary_error,
            scena::AssetError::Parse { ref reason, .. }
                if reason.contains("texture cache identity collision")
        ),
        "ordinary load must retain the immutable provenance guard: {ordinary_error:?}"
    );
    assert_eq!(
        assets
            .texture(texture)
            .expect("original descriptor survives ordinary-load collision")
            .decoded_rgba8()
            .expect("original pixels survive ordinary-load collision")
            .2,
        &[200, 35, 45, 255]
    );

    let reloaded = pollster::block_on(assets.reload_scene(&first))
        .expect("explicit reload is the source-replacement boundary");
    assert_eq!(scene_base_color_textures(&assets, &reloaded)[0], texture);
    assert_eq!(
        assets
            .texture(texture)
            .expect("explicitly reloaded descriptor exists")
            .decoded_rgba8()
            .expect("explicitly reloaded pixels decode")
            .2,
        &[35, 200, 55, 255]
    );

    std::fs::remove_dir_all(&dir).expect("explicit-boundary temp directory removes");
}

#[test]
fn reload_scene_replaces_external_buffer_and_keeps_last_complete_version_on_failure() {
    let dir = std::env::temp_dir().join(format!(
        "scena-c09-external-buffer-reload-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("external-buffer temp directory creates");
    let scene_path = dir.join("scene.gltf");
    let buffer_path = dir.join("geometry.bin");
    write_external_buffer_scene(&scene_path);
    write_triangle_buffer(&buffer_path, -0.4);

    let mut assets = Assets::new();
    assets.set_retain_policy(RetainPolicy::Always);
    let first = pollster::block_on(assets.load_scene(path_string(&scene_path)))
        .expect("external-buffer scene loads");
    assert!((first_vertex_x(&assets, &first) + 0.4).abs() <= 0.0001);

    write_triangle_buffer(&buffer_path, -0.85);
    let reloaded =
        pollster::block_on(assets.reload_scene(&first)).expect("changed external buffer reloads");
    assert!((first_vertex_x(&assets, &reloaded) + 0.85).abs() <= 0.0001);

    std::fs::remove_file(&buffer_path).expect("external buffer removes");
    let error = pollster::block_on(assets.reload_scene(&reloaded))
        .expect_err("deleted external buffer must fail closed");
    assert_asset_error_path(&error, &buffer_path);
    let cached = pollster::block_on(assets.load_scene(path_string(&scene_path)))
        .expect("last complete external-buffer scene remains cached");
    assert!((first_vertex_x(&assets, &cached) + 0.85).abs() <= 0.0001);

    std::fs::remove_dir_all(&dir).expect("external-buffer temp directory removes");
}

#[test]
fn reload_scene_uses_content_addressed_identity_for_changed_embedded_texture() {
    let dir = std::env::temp_dir().join(format!(
        "scena-c09-embedded-texture-reload-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("embedded-texture temp directory creates");
    let scene_path = dir.join("scene.gltf");
    let red_uri = png_data_uri([210, 25, 35, 255]);
    write_external_texture_scene(&scene_path, &red_uri);

    let mut assets = Assets::new();
    assets.set_retain_policy(RetainPolicy::Always);
    let first = pollster::block_on(assets.load_scene(path_string(&scene_path)))
        .expect("embedded-texture scene loads");
    let first_texture = scene_base_color_textures(&assets, &first)[0];

    let green_uri = png_data_uri([30, 215, 50, 255]);
    write_external_texture_scene(&scene_path, &green_uri);
    let reloaded =
        pollster::block_on(assets.reload_scene(&first)).expect("changed embedded texture reloads");
    let reloaded_texture = scene_base_color_textures(&assets, &reloaded)[0];
    assert_ne!(
        reloaded_texture, first_texture,
        "embedded content changes mint a new content-addressed identity"
    );
    assert_eq!(
        assets
            .texture(first_texture)
            .expect("old embedded snapshot remains valid")
            .decoded_rgba8()
            .expect("old embedded pixels remain")
            .2,
        &[210, 25, 35, 255]
    );
    assert_eq!(
        assets
            .texture(reloaded_texture)
            .expect("new embedded descriptor exists")
            .decoded_rgba8()
            .expect("new embedded pixels decode")
            .2,
        &[30, 215, 50, 255]
    );

    std::fs::remove_dir_all(&dir).expect("embedded-texture temp directory removes");
}

#[test]
fn asset_hot_reload_watcher_reports_debounced_file_change_and_reload_updates_retained_asset() {
    let scene_path = artifact_path();
    write_triangle_scene(&scene_path, [0.25, 0.5, 0.75]);

    let mut assets = Assets::new();
    assets.set_retain_policy(RetainPolicy::Always);
    let first = pollster::block_on(assets.load_scene(path_string(&scene_path)))
        .expect("initial scene loads from disk");
    assert_eq!(first.node_count(), 1);

    let mut scene = Scene::new();
    let import = scene
        .instantiate(&first)
        .expect("initial import instantiates");
    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .frame_import(camera, &import)
        .expect("initial import frames");
    let mut renderer = Renderer::headless(WIDTH, HEIGHT).expect("headless renderer builds");
    let before = render_frame(&mut renderer, &mut scene, &assets, camera);
    let mut watcher = assets
        .watch_scene_for_hot_reload(&first, Duration::from_millis(40))
        .expect("native hot reload watcher starts");

    thread::sleep(Duration::from_millis(80));
    write_triangle_scene(&scene_path, [0.85, 0.2, 0.2]);

    let changed = wait_for_change(&mut watcher, first.path().as_str());
    assert_eq!(changed, vec![first.path().clone()]);

    let reloaded = pollster::block_on(assets.reload_scene(&first))
        .expect("retained scene reloads after file change");
    assert_eq!(reloaded.node_count(), 1);
    let replacement = scene
        .replace_import(&import, &reloaded)
        .expect("reloaded scene replaces the existing import");
    scene
        .frame_import(camera, &replacement)
        .expect("reloaded import frames");
    let after = render_frame(&mut renderer, &mut scene, &assets, camera);

    assert_ne!(
        before, after,
        "hot reload visual proof must show the changed asset render"
    );
    write_frame_strip(&[before, after]);
}

fn wait_for_change(
    watcher: &mut scena::AssetHotReloadWatcher,
    expected_path: &str,
) -> Vec<scena::AssetPath> {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        let changed = watcher
            .drain_changed_scenes()
            .expect("hot reload watcher drains");
        if changed.iter().any(|path| path.as_str() == expected_path) {
            return changed;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!("timed out waiting for debounced hot-reload event for {expected_path}");
}

fn artifact_path() -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/asset-hot-reload");
    std::fs::create_dir_all(&dir).expect("asset-hot-reload artifact directory");
    dir.join("hot_reload_scene.gltf")
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn write_triangle_scene(path: &Path, base_color: [f32; 3]) {
    let scene = format!(
        r#"{{
  "asset": {{"version": "2.0", "generator": "scena-hot-reload-test"}},
  "extensionsUsed": ["KHR_materials_unlit"],
  "scene": 0,
  "scenes": [{{"name": "Default", "nodes": [0]}}],
  "nodes": [{{"name": "HotReloadTriangle", "mesh": 0}}],
  "meshes": [
    {{
      "name": "HotReloadMesh",
      "primitives": [
        {{
          "attributes": {{"POSITION": 0, "NORMAL": 1, "COLOR_0": 2}},
          "indices": 3,
          "material": 0
        }}
      ]
    }}
  ],
  "materials": [
    {{
      "name": "HotReloadMaterial",
      "pbrMetallicRoughness": {{
        "baseColorFactor": [{}, {}, {}, 1.0],
        "metallicFactor": 0.0,
        "roughnessFactor": 1.0
      }},
      "extensions": {{"KHR_materials_unlit": {{}}}}
    }}
  ],
  "buffers": [
    {{
      "byteLength": 126,
      "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAAAAAAAAAAAAIA/AAAAAAAAAAAAAIA/AAAAAAAAAAAAAIA/AACAPwAAAAAAAAAAAACAPwAAAAAAAIA/AAAAAAAAgD8AAAAAAAAAAAAAgD8AAIA/AAABAAIA"
    }}
  ],
  "bufferViews": [
    {{"buffer": 0, "byteOffset": 0, "byteLength": 36}},
    {{"buffer": 0, "byteOffset": 36, "byteLength": 36}},
    {{"buffer": 0, "byteOffset": 72, "byteLength": 48}},
    {{"buffer": 0, "byteOffset": 120, "byteLength": 6}}
  ],
  "accessors": [
    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-0.5, -0.5, 0.0], "max": [0.5, 0.5, 0.0]}},
    {{"bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3"}},
    {{"bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC4"}},
    {{"bufferView": 3, "componentType": 5123, "count": 3, "type": "SCALAR"}}
  ]
}}"#,
        base_color[0], base_color[1], base_color[2]
    );
    std::fs::write(path, scene).expect("hot-reload scene fixture writes");
}

fn write_external_texture_scene(path: &Path, image_uri: &str) {
    let scene = format!(
        r#"{{
  "asset": {{"version": "2.0", "generator": "scena-c09-reload-test"}},
  "extensionsUsed": ["KHR_materials_unlit"],
  "extensionsRequired": ["KHR_materials_unlit"],
  "scene": 0,
  "scenes": [{{"nodes": [0]}}],
  "nodes": [{{"name": "MutableTextureTriangle", "mesh": 0}}],
  "images": [{{"uri": "{image_uri}"}}],
  "textures": [{{"source": 0}}],
  "materials": [{{
    "pbrMetallicRoughness": {{"baseColorTexture": {{"index": 0}}}},
    "extensions": {{"KHR_materials_unlit": {{}}}}
  }}],
  "meshes": [{{"primitives": [{{
    "attributes": {{"POSITION": 0, "TEXCOORD_0": 1}},
    "indices": 2,
    "material": 0
  }}]}}],
  "buffers": [{{
    "byteLength": 66,
    "uri": "data:application/octet-stream;base64,mpkZv5qZGb8AAAAAmpkZP5qZGb8AAAAAAAAAAJqZGT8AAAAAAAAAAAAAAAAAAIA/AAAAAAAAAD8AAIA/AAABAAIA"
  }}],
  "bufferViews": [
    {{"buffer": 0, "byteOffset": 0, "byteLength": 36}},
    {{"buffer": 0, "byteOffset": 36, "byteLength": 24}},
    {{"buffer": 0, "byteOffset": 60, "byteLength": 6}}
  ],
  "accessors": [
    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-0.6, -0.6, 0.0], "max": [0.6, 0.6, 0.0]}},
    {{"bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2"}},
    {{"bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR"}}
  ]
}}"#
    );
    std::fs::write(path, scene).expect("external-texture scene fixture writes");
}

fn write_shared_external_texture_scene(path: &Path, image_uri: &str) {
    let scene = format!(
        r#"{{
  "asset": {{"version": "2.0", "generator": "scena-c09-shared-reload-test"}},
  "extensionsUsed": ["KHR_materials_unlit"],
  "extensionsRequired": ["KHR_materials_unlit"],
  "scene": 0,
  "scenes": [{{"nodes": [0]}}],
  "nodes": [{{"name": "SharedTextureConsumers", "mesh": 0}}],
  "images": [{{"uri": "{image_uri}"}}],
  "textures": [{{"source": 0}}, {{"source": 0}}],
  "materials": [
    {{"pbrMetallicRoughness": {{"baseColorTexture": {{"index": 0}}}}, "extensions": {{"KHR_materials_unlit": {{}}}}}},
    {{"pbrMetallicRoughness": {{"baseColorTexture": {{"index": 1}}}}, "extensions": {{"KHR_materials_unlit": {{}}}}}}
  ],
  "meshes": [{{"primitives": [
    {{"attributes": {{"POSITION": 0, "TEXCOORD_0": 1}}, "indices": 2, "material": 0}},
    {{"attributes": {{"POSITION": 0, "TEXCOORD_0": 1}}, "indices": 2, "material": 1}}
  ]}}],
  "buffers": [{{
    "byteLength": 66,
    "uri": "data:application/octet-stream;base64,mpkZv5qZGb8AAAAAmpkZP5qZGb8AAAAAAAAAAJqZGT8AAAAAAAAAAAAAAAAAAIA/AAAAAAAAAD8AAIA/AAABAAIA"
  }}],
  "bufferViews": [
    {{"buffer": 0, "byteOffset": 0, "byteLength": 36}},
    {{"buffer": 0, "byteOffset": 36, "byteLength": 24}},
    {{"buffer": 0, "byteOffset": 60, "byteLength": 6}}
  ],
  "accessors": [
    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-0.6, -0.6, 0.0], "max": [0.6, 0.6, 0.0]}},
    {{"bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2"}},
    {{"bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR"}}
  ]
}}"#
    );
    std::fs::write(path, scene).expect("shared external-texture scene fixture writes");
}

fn write_external_buffer_scene(path: &Path) {
    std::fs::write(
        path,
        r#"{
  "asset": {"version": "2.0", "generator": "scena-c09-buffer-reload-test"},
  "extensionsUsed": ["KHR_materials_unlit"],
  "extensionsRequired": ["KHR_materials_unlit"],
  "scene": 0,
  "scenes": [{"nodes": [0]}],
  "nodes": [{"name": "ExternalBufferTriangle", "mesh": 0}],
  "materials": [{"pbrMetallicRoughness": {"baseColorFactor": [0.3, 0.8, 0.4, 1.0]}, "extensions": {"KHR_materials_unlit": {}}}],
  "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1, "material": 0}]}],
  "buffers": [{"byteLength": 42, "uri": "geometry.bin"}],
  "bufferViews": [
    {"buffer": 0, "byteOffset": 0, "byteLength": 36},
    {"buffer": 0, "byteOffset": 36, "byteLength": 6}
  ],
  "accessors": [
    {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1.0, -0.5, 0.0], "max": [0.6, 0.6, 0.0]},
    {"bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR"}
  ]
}"#,
    )
    .expect("external-buffer scene fixture writes");
}

fn write_triangle_buffer(path: &Path, left_x: f32) {
    let mut bytes = Vec::with_capacity(42);
    for value in [left_x, -0.5, 0.0, 0.6, -0.5, 0.0, 0.0, 0.6, 0.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for index in [0_u16, 1, 2] {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    std::fs::write(path, bytes).expect("external geometry buffer writes");
}

fn scene_base_color_textures(
    assets: &Assets,
    scene: &scena::SceneAsset,
) -> Vec<scena::TextureHandle> {
    scene
        .nodes()
        .iter()
        .flat_map(|node| node.meshes())
        .map(|mesh| {
            assets
                .material(mesh.material())
                .expect("scene material exists")
                .base_color_texture()
                .expect("scene material has base-color texture")
        })
        .collect()
}

fn assert_asset_error_path(error: &scena::AssetError, expected: &Path) {
    let actual = match error {
        scena::AssetError::NotFound { path } | scena::AssetError::Io { path, .. } => path,
        other => panic!("expected filesystem dependency error, got {other:?}"),
    };
    assert_eq!(actual, &path_string(expected));
}

fn first_vertex_x(assets: &Assets, scene: &scena::SceneAsset) -> f32 {
    let mesh = scene
        .nodes()
        .iter()
        .find_map(|node| node.mesh())
        .expect("scene mesh exists");
    assets
        .geometry(mesh.geometry())
        .expect("scene geometry exists")
        .vertices()[0]
        .position
        .x
}

fn write_solid_png(path: &Path, rgba: [u8; 4]) {
    std::fs::write(path, solid_png_bytes(rgba)).expect("texture fixture writes");
}

fn png_data_uri(rgba: [u8; 4]) -> String {
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(solid_png_bytes(rgba))
    )
}

fn solid_png_bytes(rgba: [u8; 4]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    {
        let mut writer = encoder.write_header().expect("texture PNG header writes");
        writer
            .write_image_data(&rgba)
            .expect("texture PNG pixels write");
    }
    bytes
}

fn render_frame(
    renderer: &mut Renderer,
    scene: &mut Scene,
    assets: &Assets,
    camera: scena::CameraKey,
) -> Vec<u8> {
    renderer
        .prepare_with_assets(scene, assets)
        .expect("hot-reload scene prepares");
    renderer
        .render(scene, camera)
        .expect("hot-reload scene renders");
    renderer.frame_rgba8().to_vec()
}

fn write_frame_strip(frames: &[Vec<u8>]) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/gate-artifacts/asset-hot-reload/asset-hot-reload-animated-proof.ppm");
    let mut bytes = format!("P6\n{} {}\n255\n", WIDTH * frames.len() as u32, HEIGHT).into_bytes();
    for y in 0..HEIGHT {
        for frame in frames {
            for x in 0..WIDTH {
                let index = ((y * WIDTH + x) * 4) as usize;
                bytes.extend_from_slice(&frame[index..index + 3]);
            }
        }
    }
    std::fs::write(path, bytes).expect("hot-reload animated proof writes");
}
