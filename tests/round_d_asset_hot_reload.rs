#![cfg(all(feature = "hot-reload", not(target_arch = "wasm32")))]

use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use scena::{Assets, Renderer, RetainPolicy, Scene};

const WIDTH: u32 = 64;
const HEIGHT: u32 = 64;

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
