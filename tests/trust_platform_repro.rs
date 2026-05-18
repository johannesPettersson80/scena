//! Reproduces trust-platform's reported Bug #2: building a Scene manually
//! with `scene.mesh(geometry, material)` from `scene_asset.nodes()` (instead
//! of going through `scene.instantiate(&scene_asset)`) and rendering a glTF
//! PBR asset reportedly produces a blank canvas on WebGL2 / Firefox.
//!
//! This test runs the same flow against the headless CPU rasterizer to see
//! whether the bug reproduces backend-independently. If it does, the root
//! cause is in scena's scene/material/prepare path (not browser-specific).
//! If it does not, the bug is likely confined to a specific GPU backend.

#![cfg(not(target_arch = "wasm32"))]

use scena::{Assets, Renderer, Scene, Transform};

const WATERBOTTLE_PATH: &str = "tests/assets/gltf/khronos/WaterBottle/WaterBottle.gltf";
const CRACKER_BOX_PATH: &str = "/home/johannes/projects/trust-platform/editors/vscode/media/trust-twin/components/ycb/meshes/003_cracker_box_textured.gltf";

fn build_manual_scene_from_gltf() -> (Assets, Scene, scena::CameraKey) {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene(WATERBOTTLE_PATH)).expect("WaterBottle loads");

    let mut scene = Scene::new();
    let root = scene.root();
    for node in scene_asset.nodes() {
        for mesh in node.meshes() {
            scene
                .mesh(mesh.geometry(), mesh.material())
                .parent(root)
                .transform(node.transform())
                .add()
                .expect("mesh node inserts");
        }
    }

    let camera = scene
        .add_perspective_camera(
            scene.root(),
            scena::PerspectiveCamera::default(),
            Transform::at(scena::Vec3::new(0.12, 0.05, 0.25))
                .rotate_y_deg(25.0)
                .rotate_x_deg(-10.0),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    (assets, scene, camera)
}

fn coloured_ratio(frame: &[u8]) -> f32 {
    let coloured = frame
        .chunks_exact(4)
        .filter(|p| p[0] != 0 || p[1] != 0 || p[2] != 0)
        .count();
    let total = frame.len() / 4;
    if total == 0 {
        0.0
    } else {
        coloured as f32 / total as f32
    }
}

/// Cracker-box glTF references its baseColorTexture by external URI
/// ("003_cracker_box_textured.png"). This is the trust-platform-failing
/// shape: a glTF whose textures live in adjacent files rather than as a
/// base64 data URI or a buffer view. If scena's load path silently drops
/// the texture (returning Ok(None) from decode_texture_pixels), the prepared
/// material slot ends up with no textures and renders with the 1x1 fallback.
#[test]
fn manually_built_scene_from_external_uri_gltf_renders_non_blank() {
    if !std::path::Path::new(CRACKER_BOX_PATH).exists() {
        eprintln!("cracker box fixture not available, skipping");
        return;
    }
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene(CRACKER_BOX_PATH)).expect("cracker box loads");

    let mut scene = Scene::new();
    let root = scene.root();
    let mut added = 0usize;
    for node in scene_asset.nodes() {
        for mesh in node.meshes() {
            // Diagnostics: was the material's base-color texture actually
            // decoded into Assets, or did scena silently drop it?
            if let Some(material) = assets.material(mesh.material())
                && let Some(texture_handle) = material.base_color_texture()
            {
                let decoded = assets
                    .texture(texture_handle)
                    .map(|t| t.has_decoded_pixels())
                    .unwrap_or(false);
                eprintln!(
                    "cracker box material {:?} base_color texture {:?} has_decoded_pixels={}",
                    mesh.material(),
                    texture_handle,
                    decoded,
                );
            }
            scene
                .mesh(mesh.geometry(), mesh.material())
                .parent(root)
                .transform(node.transform())
                .add()
                .expect("mesh inserts");
            added += 1;
        }
    }
    assert!(added >= 1, "cracker box has at least one mesh primitive");

    let camera = scene
        .add_perspective_camera(
            scene.root(),
            scena::PerspectiveCamera::default(),
            Transform::at(scena::Vec3::new(0.10, 0.05, 0.20))
                .rotate_y_deg(25.0)
                .rotate_x_deg(-10.0),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");

    let mut renderer = match Renderer::headless_gpu(256, 256) {
        Ok(r) => r,
        Err(error) => {
            eprintln!("Renderer::headless_gpu unavailable, skipping: {error:?}");
            return;
        }
    };
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("prepare succeeds");
    renderer.render(&scene, camera).expect("render succeeds");

    let frame = renderer.frame_rgba8();
    let ratio = coloured_ratio(frame);
    assert!(
        ratio > 0.01,
        "cracker box rendered nearly blank ({:.2}% coloured); external-URI texture path may be silently dropping the base-color texture",
        ratio * 100.0,
    );
}

#[test]
fn manually_built_scene_from_gltf_handles_renders_non_blank_on_headless_gpu() {
    let (assets, mut scene, camera) = build_manual_scene_from_gltf();
    let mut renderer = match Renderer::headless_gpu(256, 256) {
        Ok(r) => r,
        Err(error) => {
            eprintln!("Renderer::headless_gpu unavailable, skipping: {error:?}");
            return;
        }
    };
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("prepare succeeds on HeadlessGpu");
    renderer
        .render(&scene, camera)
        .expect("render succeeds on HeadlessGpu");

    let frame = renderer.frame_rgba8();
    let ratio = coloured_ratio(frame);
    assert!(
        ratio > 0.01,
        "HeadlessGpu render of manually-built scene from glTF material handles \
         is nearly blank ({:.2}% coloured). This reproduces trust-platform Bug #2 \
         in the local GPU prepare path.",
        ratio * 100.0,
    );
}

#[test]
fn manually_built_scene_from_gltf_handles_renders_non_blank() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene(WATERBOTTLE_PATH)).expect("WaterBottle loads");

    let mut scene = Scene::new();
    let root = scene.root();

    // The trust-platform pattern: iterate asset nodes and add meshes
    // by hand, passing the glTF-loaded MaterialHandle straight through
    // to scene.mesh(geom, mat).
    let nodes = scene_asset.nodes().to_vec();
    let mut added = 0usize;
    for node in &nodes {
        for mesh in node.meshes() {
            scene
                .mesh(mesh.geometry(), mesh.material())
                .parent(root)
                .transform(node.transform())
                .add()
                .expect("mesh node inserts");
            added += 1;
        }
    }
    assert!(
        added >= 1,
        "WaterBottle has at least one mesh primitive; set up assumption"
    );

    // Hand-place the camera at the same offset the m8 WaterBottle test uses.
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            scena::PerspectiveCamera::default(),
            Transform::at(scena::Vec3::new(0.12, 0.05, 0.25))
                .rotate_y_deg(25.0)
                .rotate_x_deg(-10.0),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");

    let mut renderer = Renderer::headless(256, 256).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("prepare succeeds against the manually built scene");
    renderer
        .render(&scene, camera)
        .expect("render succeeds against the manually built scene");

    let frame = renderer.frame_rgba8();
    let coloured = frame
        .chunks_exact(4)
        .filter(|p| p[0] != 0 || p[1] != 0 || p[2] != 0)
        .count();
    let total = frame.len() / 4;
    let ratio = coloured as f32 / total as f32;
    assert!(
        ratio > 0.01,
        "manually-built scene from glTF material handles rendered nearly \
         blank ({coloured}/{total} pixels coloured, {:.2}%). \
         Compare against the standard instantiate() path to localise the bug.",
        ratio * 100.0,
    );
}
