use crate::app::prelude::*;

const Q02_MATERIAL_PRESETS: &[&str] = &[
    "matte",
    "plastic",
    "metal",
    "rough_metal",
    "chrome",
    "brushed_steel",
    "clearcoat_plastic",
    "satin",
    "leather",
    "clear_glass",
    "frosted_glass",
    "rubber",
];

const Q02_NEIGHBOR_PAIRS: &[(&str, &str)] = &[
    ("metal", "rough_metal"),
    ("metal", "chrome"),
    ("chrome", "plastic"),
    ("clearcoat_plastic", "plastic"),
    ("clear_glass", "frosted_glass"),
    ("rubber", "plastic"),
];

pub(crate) fn q02_material_result_passes(
    root: &Path,
    relative: &str,
    schema: &str,
    surface: &str,
    proof_class: &str,
    expected_live_frame: &str,
) -> Result<bool, String> {
    let path = root.join(relative);
    if !path.is_file() {
        return Ok(false);
    }
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value = serde_json::from_str::<Value>(&text)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    let evaluator = value.get("threshold_evaluator").unwrap_or(&Value::Null);
    let live_frame = value.get("live_frame").unwrap_or(&Value::Null);
    let errors_empty = value
        .get("errors")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty);
    let presets_complete = value
        .get("per_material")
        .and_then(Value::as_object)
        .is_some_and(|materials| {
            Q02_MATERIAL_PRESETS
                .iter()
                .all(|preset| materials.contains_key(*preset))
        });
    let neighbors_pass = value
        .get("neighbor_pairs")
        .and_then(Value::as_array)
        .is_some_and(|neighbors| {
            Q02_NEIGHBOR_PAIRS.iter().all(|(left, right)| {
                neighbors.iter().any(|neighbor| {
                    neighbor.get("passed").and_then(Value::as_bool) == Some(true)
                        && q02_neighbor_pair_matches(neighbor, left, right)
                })
            })
        });
    let commit_valid = value
        .get("commit_sha")
        .and_then(Value::as_str)
        .is_some_and(|commit| {
            commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        });
    let timestamp_valid = value
        .get("timestamp_unix_seconds")
        .and_then(Value::as_u64)
        .is_some_and(|timestamp| timestamp > 0);
    let source_checksums_valid = value
        .get("source_checksums")
        .and_then(Value::as_array)
        .is_some_and(|entries| {
            !entries.is_empty()
                && entries.iter().all(|entry| {
                    let Some(relative) = entry.get("path").and_then(Value::as_str) else {
                        return false;
                    };
                    let Some(expected) = entry.get("sha256").and_then(Value::as_str) else {
                        return false;
                    };
                    relative.starts_with(|character: char| character.is_ascii_alphanumeric())
                        && sha256_hex(&root.join(relative)).is_ok_and(|actual| actual == expected)
                })
        });
    let live_frame_path = live_frame.get("path").and_then(Value::as_str);
    let live_frame_hash = live_frame.get("sha256").and_then(Value::as_str);
    let live_frame_valid = live_frame_path == Some(expected_live_frame)
        && live_frame_hash.is_some_and(|expected| {
            sha256_hex(&root.join(expected_live_frame)).is_ok_and(|actual| actual == expected)
        });
    let surface_specific = match surface {
        "live-cpu-headless" => value
            .get("live_renderer")
            .and_then(Value::as_str)
            .is_some_and(|renderer| renderer.contains("Renderer::headless")),
        "live-webgl2-chromium" => value
            .get("per_material")
            .and_then(Value::as_object)
            .is_some_and(|materials| {
                Q02_MATERIAL_PRESETS.iter().all(|preset| {
                    let Some(material) = materials.get(*preset) else {
                        return false;
                    };
                    material
                        .get("crop_path")
                        .and_then(Value::as_str)
                        .is_some_and(|path| {
                            path.starts_with(
                                "target/gate-artifacts/round-e-cloudflare-material-proof/",
                            )
                        })
                        && material
                            .get("reference_path")
                            .and_then(Value::as_str)
                            .is_some_and(|path| path.starts_with("tests/visual/references/"))
                        && material.get("reference_delta_gate").and_then(Value::as_str)
                            == Some("hard")
                        && material
                            .get("passed_reference_delta")
                            .and_then(Value::as_bool)
                            == Some(true)
                })
            }),
        "live-webgpu-chromium" => {
            value.get("backend").and_then(Value::as_str) == Some("WebGpu")
                && value
                    .get("renderer_readback")
                    .and_then(|readback| readback.get("source"))
                    .and_then(Value::as_str)
                    == Some("renderer-owned-gpu-copy")
        }
        _ => false,
    };
    Ok(value.get("schema").and_then(Value::as_str) == Some(schema)
        && value.get("status").and_then(Value::as_str) == Some("passed")
        && value.get("proof_class").and_then(Value::as_str) == Some(proof_class)
        && evaluator.get("proof_class").and_then(Value::as_str)
            == Some("round-e-shared-material-threshold-evaluator")
        && evaluator.get("surface").and_then(Value::as_str) == Some(surface)
        && errors_empty
        && presets_complete
        && neighbors_pass
        && commit_valid
        && timestamp_valid
        && source_checksums_valid
        && live_frame_valid
        && surface_specific)
}

fn q02_neighbor_pair_matches(entry: &Value, left: &str, right: &str) -> bool {
    let Some(pair) = entry.get("pair").and_then(Value::as_array) else {
        return false;
    };
    if pair.len() != 2 {
        return false;
    }
    let first = pair[0].as_str();
    let second = pair[1].as_str();
    (first == Some(left) && second == Some(right)) || (first == Some(right) && second == Some(left))
}
