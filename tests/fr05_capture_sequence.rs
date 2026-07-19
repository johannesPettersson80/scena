#![cfg(feature = "scene-host")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;

#[test]
fn fr05_recipe_capture_emits_canonical_turntable_and_clip_frames() {
    let root = fixture_dir();
    let recipe = root.join("capture.recipe.json");
    let out_dir = root.join("capture-output");
    fs::write(
        &recipe,
        serde_json::to_vec_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "geometries": [{
                "id": "box_geo",
                "primitive": { "kind": "box", "size": [0.24, 0.12, 0.08] }
            }],
            "materials": [{
                "id": "box_mat",
                "kind": "unlit",
                "base_color": "#50A8E8"
            }],
            "nodes": [{
                "id": "box",
                "geometry": "box_geo",
                "material": "box_mat"
            }],
            "animations": [{
                "id": "move",
                "duration": 1.0,
                "channels": [{
                    "target": { "kind": "node", "id": "box" },
                    "path": "translation",
                    "times": [0.0, 1.0],
                    "values": [[-0.04, 0.0, 0.0], [0.04, 0.0, 0.0]],
                    "interpolation": "linear"
                }]
            }],
            "capture": { "width": 160, "height": 120 }
        }))
        .expect("capture recipe serializes"),
    )
    .expect("capture recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "capture",
            path(&recipe),
            "--out-dir",
            path(&out_dir),
            "--views",
            "front,top,right,isometric",
            "--turntable",
            "4",
            "--clip",
            "move",
            "--frames",
            "3",
        ])
        .output()
        .expect("capture command runs");

    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("capture report is JSON");
    assert_eq!(report["schema"], "scena.capture_sequence_result.v1");
    assert_eq!(report["ok"], true);
    assert_eq!(report["coordinate_convention"]["world_up"], "+Y");
    assert_eq!(
        report["canonical_view_order"],
        json!(["front", "top", "right", "isometric"])
    );
    assert_eq!(report["sequence_encoding"], "png_frames_and_contact_sheet");

    let frames = report["frames"].as_array().expect("capture frames array");
    assert_eq!(frames.len(), 11, "{report:#}");
    for (index, expected) in ["front", "top", "right", "isometric"]
        .into_iter()
        .enumerate()
    {
        assert_eq!(frames[index]["kind"], "canonical_view");
        assert_eq!(frames[index]["label"], expected);
        assert!(frames[index]["camera"]["target"].is_array());
    }
    let turntable = &frames[4..8];
    assert!(turntable.iter().all(|frame| frame["kind"] == "turntable"));
    assert_eq!(turntable[0]["turntable"]["sample_index"], 0);
    assert_eq!(turntable[3]["turntable"]["sample_count"], 4);
    let clip = &frames[8..11];
    assert!(clip.iter().all(|frame| frame["kind"] == "clip"));
    assert_eq!(clip[0]["clip"]["name"], "move");
    assert_eq!(clip[0]["clip"]["time_seconds"], 0.0);
    assert_eq!(clip[1]["clip"]["time_seconds"], 0.5);
    assert_eq!(clip[2]["clip"]["time_seconds"], 1.0);

    let hashes = frames
        .iter()
        .map(|frame| {
            let png = PathBuf::from(frame["png"].as_str().expect("frame PNG path"));
            assert!(fs::metadata(&png).expect("frame PNG exists").len() > 0);
            frame["capture"]["payload"]["fnv1a64"]
                .as_str()
                .expect("frame hash")
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_ne!(hashes[4], hashes[5], "turntable samples must change pixels");
    assert_ne!(hashes[8], hashes[10], "clip endpoints must change pixels");

    let sheet = PathBuf::from(
        report["contact_sheet"]["png"]
            .as_str()
            .expect("contact sheet path"),
    );
    assert!(fs::metadata(sheet).expect("contact sheet exists").len() > 0);
    assert_eq!(
        report["contact_sheet"]["tiles"].as_array().unwrap().len(),
        11
    );
}

fn fixture_dir() -> PathBuf {
    let root = PathBuf::from("target/gate-artifacts/fr05-capture-sequence");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("FR05 fixture directory creates");
    root
}

fn path(path: &Path) -> &str {
    path.to_str().expect("fixture path is UTF-8")
}
