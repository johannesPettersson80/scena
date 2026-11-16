//! An instanced scene import must draw an asset exactly where the regular
//! import draws it, on the CPU rasterizer and, when a headless adapter is
//! available, on the GPU path.
//!
//! The asset is a rotated bar under a non-uniformly scaled node, so the bar is
//! sheared inside the asset. The regular import keeps that hierarchy and
//! resolves it through world matrices. The instanced import flattens it into
//! one instance per drawable, and has to carry the same shear.
#![cfg(feature = "scene-host")]
#![cfg(not(target_arch = "wasm32"))]

#[allow(dead_code)]
mod support;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use scena::{AssetPath, DefaultAssetFetcher, SceneHostCore, Transform, Vec3};

const SIZE: u32 = 160;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Cpu,
    Gpu,
}

#[derive(Debug, Clone, Copy)]
enum Import {
    Regular,
    Instanced,
}

#[test]
fn cpu_instanced_import_draws_a_sheared_asset_like_the_regular_import() {
    assert_instanced_matches_regular(Backend::Cpu, Transform::IDENTITY);
}

#[test]
fn gpu_instanced_import_draws_a_sheared_asset_like_the_regular_import() {
    if host(Backend::Gpu).is_none() {
        return;
    }
    assert_instanced_matches_regular(Backend::Gpu, Transform::IDENTITY);
}

#[test]
fn cpu_instance_root_with_non_uniform_scale_shears_like_the_regular_import() {
    assert_instanced_matches_regular(Backend::Cpu, stretched_root());
}

#[test]
fn gpu_instance_root_with_non_uniform_scale_shears_like_the_regular_import() {
    if host(Backend::Gpu).is_none() {
        return;
    }
    assert_instanced_matches_regular(Backend::Gpu, stretched_root());
}

/// A root transform whose non-uniform scale shears the rotated bar again.
fn stretched_root() -> Transform {
    Transform::at(Vec3::new(0.1, -0.05, 0.0))
        .rotate_z_deg(20.0)
        .with_scale(Vec3::new(0.5, 1.6, 1.0))
}

fn assert_instanced_matches_regular(backend: Backend, root: Transform) {
    let asset = AssetFile::write(&sheared_bar_gltf());
    let regular = render(backend, &asset, Import::Regular, root);
    let instanced = render(backend, &asset, Import::Instanced, root);
    drop(asset);

    let lit = regular
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 128)
        .count();
    assert!(
        lit > 100,
        "{backend:?}: the regular import must draw the bar; lit pixels={lit}"
    );
    // Both paths place the same quad with the same matrix, so only the edge
    // rule of the rasterizer can tip a boundary pixel.
    let differing = regular
        .chunks_exact(4)
        .zip(instanced.chunks_exact(4))
        .filter(|(left, right)| left[0].abs_diff(right[0]) > 96)
        .count();
    assert!(
        differing * 50 <= lit,
        "{backend:?}: the instanced import must cover the pixels the regular import covers; \
         {differing} of {lit} lit pixels differ"
    );
}

fn render(backend: Backend, asset: &AssetFile, import: Import, root: Transform) -> Vec<u8> {
    let mut host = host(backend).expect("the caller checked that the backend is available");
    let path = AssetPath::from(asset.path.to_str().expect("asset path is UTF-8"));
    match import {
        // An instance root's transform sits above the whole asset, including
        // the asset's own root node, so the regular twin gets a parent node
        // with the same transform.
        Import::Regular => {
            let parent = host
                .add_empty(None, root, None)
                .expect("parent node inserts");
            pollster::block_on(host.instantiate_url_under(parent, path))
                .expect("asset instantiates");
        }
        Import::Instanced => {
            let roots = pollster::block_on(host.instantiate_url_instanced(path, 1))
                .expect("asset instantiates as instances");
            assert_eq!(roots.len(), 1, "one instance root was requested");
            host.set_transform(roots[0], root)
                .expect("instance root transform applies");
        }
    }
    host.prepare().expect("host prepares");
    host.render().expect("host renders");
    host.renderer().frame_rgba8().to_vec()
}

fn host(backend: Backend) -> Option<SceneHostCore<DefaultAssetFetcher>> {
    match backend {
        Backend::Cpu => Some(SceneHostCore::headless(SIZE, SIZE).expect("CPU host builds")),
        Backend::Gpu => {
            support::parity::configure_lavapipe_adapter();
            match SceneHostCore::headless_gpu(SIZE, SIZE) {
                Ok(host) => Some(host),
                Err(error) => {
                    eprintln!("skipping: no headless GPU adapter is available ({error:?})");
                    None
                }
            }
        }
    }
}

/// A glTF file in the system temp directory, removed when dropped.
struct AssetFile {
    path: PathBuf,
}

impl AssetFile {
    fn write(gltf: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "scena-exact-instances-{}-{nonce}-{}.gltf",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, gltf).expect("asset writes");
        Self { path }
    }
}

impl Drop for AssetFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// A two-node glTF: a parent scaled (3, 1, 1) and a child mesh node turned 35
/// degrees about Z. The mesh is an unlit white, double-sided 0.5 x 0.08 quad
/// facing +Z, so it is a sheared bar in the asset.
fn sheared_bar_gltf() -> String {
    let positions: [[f32; 3]; 4] = [
        [-0.25, -0.04, 0.0],
        [0.25, -0.04, 0.0],
        [0.25, 0.04, 0.0],
        [-0.25, 0.04, 0.0],
    ];
    let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
    let mut buffer = Vec::new();
    for position in positions {
        for component in position {
            buffer.extend_from_slice(&component.to_le_bytes());
        }
    }
    for _ in positions {
        for component in [0.0_f32, 0.0, 1.0] {
            buffer.extend_from_slice(&component.to_le_bytes());
        }
    }
    let index_offset = buffer.len();
    for index in indices {
        buffer.extend_from_slice(&index.to_le_bytes());
    }
    let (sin, cos) = (35.0_f32.to_radians() * 0.5).sin_cos();
    serde_json::json!({
        "asset": { "version": "2.0" },
        "extensionsUsed": ["KHR_materials_unlit"],
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [
            { "name": "ScaledParent", "scale": [3.0, 1.0, 1.0], "children": [1] },
            { "name": "RotatedBar", "rotation": [0.0, 0.0, sin, cos], "mesh": 0 }
        ],
        "meshes": [{
            "primitives": [{
                "attributes": { "POSITION": 0, "NORMAL": 1 },
                "indices": 2,
                "material": 0
            }]
        }],
        "materials": [{
            "doubleSided": true,
            "pbrMetallicRoughness": { "baseColorFactor": [1.0, 1.0, 1.0, 1.0] },
            "extensions": { "KHR_materials_unlit": {} }
        }],
        "accessors": [
            {
                "bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3",
                "min": [-0.25, -0.04, 0.0], "max": [0.25, 0.04, 0.0]
            },
            { "bufferView": 1, "componentType": 5126, "count": 4, "type": "VEC3" },
            { "bufferView": 2, "componentType": 5123, "count": 6, "type": "SCALAR" }
        ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 48, "target": 34962 },
            { "buffer": 0, "byteOffset": 48, "byteLength": 48, "target": 34962 },
            { "buffer": 0, "byteOffset": index_offset, "byteLength": 12, "target": 34963 }
        ],
        "buffers": [{
            "byteLength": buffer.len(),
            "uri": format!(
                "data:application/octet-stream;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(&buffer)
            )
        }]
    })
    .to_string()
}
