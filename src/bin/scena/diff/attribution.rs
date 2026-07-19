use std::collections::BTreeMap;

use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Classification {
    Attributed,
    Ambiguous,
    Unattributed,
}

impl Classification {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Attributed => "attributed",
            Self::Ambiguous => "ambiguous",
            Self::Unattributed => "unattributed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RegionKey {
    classification: Classification,
    reason: &'static str,
    before: String,
    after: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RegionBounds {
    pixel_count: usize,
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

pub(super) fn attributed_visual_diff(
    before_capture: &scena::CaptureRgba8,
    after_capture: &scena::CaptureRgba8,
    before_aov: &scena::SceneHostSemanticAovCaptureV1,
    after_aov: &scena::SceneHostSemanticAovCaptureV1,
    before_manifest: &scena::SceneRecipeBuildV1,
    after_manifest: &scena::SceneRecipeBuildV1,
    max_abs_diff: u8,
) -> Result<(Value, Vec<u8>), String> {
    let width = before_capture.descriptor.width;
    let height = before_capture.descriptor.height;
    ensure_matching_dimensions(width, height, after_capture, before_aov, after_aov)?;

    let before_legend = persistent_legend(before_manifest, before_aov)?;
    let after_legend = persistent_legend(after_manifest, after_aov)?;
    let unresolved_exclusions =
        has_unresolved_exclusions(before_aov) || has_unresolved_exclusions(after_aov);
    let mut regions = BTreeMap::<RegionKey, RegionBounds>::new();
    let mut diff_rgba8 = vec![0_u8; before_capture.rgba8.len()];
    let mut attributed_pixels = 0_usize;
    let mut ambiguous_pixels = 0_usize;
    let mut unattributed_pixels = 0_usize;

    for pixel_index in 0..(width as usize).saturating_mul(height as usize) {
        let byte_index = pixel_index.saturating_mul(4);
        let before_pixel = &before_capture.rgba8[byte_index..byte_index + 4];
        let after_pixel = &after_capture.rgba8[byte_index..byte_index + 4];
        if !pixel_changed(before_pixel, after_pixel, max_abs_diff) {
            continue;
        }
        diff_rgba8[byte_index..byte_index + 4].copy_from_slice(&[255, 0, 255, 255]);
        let x = (pixel_index % width as usize) as u32;
        let y = (pixel_index / width as usize) as u32;
        let before_id = before_aov.id_indices[pixel_index];
        let after_id = after_aov.id_indices[pixel_index];
        let before_identity = before_legend.get(&before_id).cloned();
        let after_identity = after_legend.get(&after_id).cloned();
        let edge =
            is_identity_edge(before_aov, pixel_index) || is_identity_edge(after_aov, pixel_index);
        let (classification, reason) = classify(
            before_id,
            after_id,
            before_identity.as_ref(),
            after_identity.as_ref(),
            edge,
            unresolved_exclusions,
        );
        match classification {
            Classification::Attributed => attributed_pixels += 1,
            Classification::Ambiguous => ambiguous_pixels += 1,
            Classification::Unattributed => unattributed_pixels += 1,
        }
        let key = RegionKey {
            classification,
            reason,
            before: canonical_identity(before_identity.as_ref())?,
            after: canonical_identity(after_identity.as_ref())?,
        };
        regions
            .entry(key)
            .and_modify(|bounds| {
                bounds.pixel_count += 1;
                bounds.min_x = bounds.min_x.min(x);
                bounds.min_y = bounds.min_y.min(y);
                bounds.max_x = bounds.max_x.max(x);
                bounds.max_y = bounds.max_y.max(y);
            })
            .or_insert(RegionBounds {
                pixel_count: 1,
                min_x: x,
                min_y: y,
                max_x: x,
                max_y: y,
            });
    }

    let changed_pixels = attributed_pixels + ambiguous_pixels + unattributed_pixels;
    let regions = regions
        .into_iter()
        .map(|(key, bounds)| {
            Ok(json!({
                "classification": key.classification.as_str(),
                "reason": key.reason,
                "pixel_count": bounds.pixel_count,
                "bounds": {
                    "min_x": bounds.min_x,
                    "min_y": bounds.min_y,
                    "max_x": bounds.max_x,
                    "max_y": bounds.max_y,
                },
                "before": identity_report(&key.before, before_aov, before_manifest)?,
                "after": identity_report(&key.after, after_aov, after_manifest)?,
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok((
        json!({
            "summary": {
                "changed_pixels": changed_pixels,
                "attributed_pixels": attributed_pixels,
                "ambiguous_pixels": ambiguous_pixels,
                "unattributed_pixels": unattributed_pixels,
            },
            "semantics": {
                "identity": "persistent_recipe_node_or_instance_candidate",
                "anti_aliased_edges": "ambiguous",
                "transparent_and_excluded_surfaces": "unattributed_or_ambiguous",
                "zero_id": "background_or_excluded_surface",
                "attribution_scope": "changed_color_pixels_sampled_against_semantic_id_aovs",
                "not_claimed": "causal_attribution_for_transparency_post_processing_or_subpixel_edges",
                "global_exclusions_present": unresolved_exclusions,
            },
            "before_exclusions": before_aov.exclusions,
            "after_exclusions": after_aov.exclusions,
            "regions": regions,
        }),
        diff_rgba8,
    ))
}

fn ensure_matching_dimensions(
    width: u32,
    height: u32,
    after_capture: &scena::CaptureRgba8,
    before_aov: &scena::SceneHostSemanticAovCaptureV1,
    after_aov: &scena::SceneHostSemanticAovCaptureV1,
) -> Result<(), String> {
    let dimensions = [
        (
            "after capture",
            after_capture.descriptor.width,
            after_capture.descriptor.height,
        ),
        ("before semantic AOV", before_aov.width, before_aov.height),
        ("after semantic AOV", after_aov.width, after_aov.height),
    ];
    for (label, actual_width, actual_height) in dimensions {
        if (actual_width, actual_height) != (width, height) {
            return Err(format!(
                "rendered diff requires matching dimensions; before capture is {width}x{height}, {label} is {actual_width}x{actual_height}"
            ));
        }
    }
    Ok(())
}

fn persistent_legend(
    manifest: &scena::SceneRecipeBuildV1,
    aov: &scena::SceneHostSemanticAovCaptureV1,
) -> Result<BTreeMap<u32, Value>, String> {
    let mut nodes = manifest
        .nodes
        .iter()
        .map(|node| {
            (
                node.handle,
                json!({ "kind": "recipe_node", "node_id": node.id }),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for import in &manifest.imports {
        for (node_path, handle) in &import.nodes_by_path {
            nodes.insert(
                *handle,
                json!({
                    "kind": "import_node",
                    "import_id": import.id,
                    "node_path": node_path,
                }),
            );
        }
    }
    let instances = manifest
        .instances
        .iter()
        .map(|instance| {
            (
                (instance.set_handle, instance.instance_id),
                json!({
                    "kind": "recipe_instance",
                    "set_id": instance.set_id,
                    "instance_id": instance.id,
                }),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut legend = BTreeMap::new();
    for entry in &aov.legend {
        let identity = entry
            .instance_id
            .and_then(|instance| instances.get(&(entry.node_handle, instance)))
            .or_else(|| nodes.get(&entry.node_handle))
            .cloned()
            .ok_or_else(|| {
                format!(
                    "semantic AOV palette {} has no persistent recipe identity",
                    entry.palette_index
                )
            })?;
        legend.insert(entry.palette_index, identity);
    }
    Ok(legend)
}

fn classify(
    before_id: u32,
    after_id: u32,
    before: Option<&Value>,
    after: Option<&Value>,
    edge: bool,
    unresolved_exclusions: bool,
) -> (Classification, &'static str) {
    if before_id == 0 && after_id == 0 {
        return (
            Classification::Unattributed,
            "background_or_excluded_surface",
        );
    }
    if unresolved_exclusions {
        return (
            Classification::Ambiguous,
            "excluded_surface_present_without_pixel_mask",
        );
    }
    if edge {
        return (Classification::Ambiguous, "semantic_identity_edge");
    }
    match (before_id, after_id, before, after) {
        (_, _, Some(left), Some(right)) if left == right => {
            (Classification::Attributed, "same_persistent_identity")
        }
        (0, _, None, Some(_)) => (Classification::Attributed, "persistent_identity_added"),
        (_, 0, Some(_), None) => (Classification::Attributed, "persistent_identity_removed"),
        (_, _, Some(_), Some(_)) => (
            Classification::Ambiguous,
            "different_persistent_identity_candidates",
        ),
        _ => (Classification::Unattributed, "missing_persistent_identity"),
    }
}

fn has_unresolved_exclusions(aov: &scena::SceneHostSemanticAovCaptureV1) -> bool {
    let exclusions = aov.exclusions;
    exclusions.transparent_triangle_count > 0
        || exclusions.overlay_triangle_count > 0
        || exclusions.unattributed_triangle_count > 0
        || exclusions.stroke_segment_count > 0
        || exclusions.label_quad_count > 0
        || exclusions.gpu_instance_record_count > 0
}

fn is_identity_edge(aov: &scena::SceneHostSemanticAovCaptureV1, index: usize) -> bool {
    let width = aov.width as usize;
    let height = aov.height as usize;
    let x = index % width;
    let y = index / width;
    let id = aov.id_indices[index];
    let neighbors = [
        x.checked_sub(1).map(|x| y * width + x),
        (x + 1 < width).then_some(y * width + x + 1),
        y.checked_sub(1).map(|y| y * width + x),
        (y + 1 < height).then_some((y + 1) * width + x),
    ];
    neighbors
        .into_iter()
        .flatten()
        .any(|neighbor| aov.id_indices[neighbor] != id)
}

fn pixel_changed(before: &[u8], after: &[u8], tolerance: u8) -> bool {
    before
        .iter()
        .zip(after)
        .any(|(before, after)| before.abs_diff(*after) > tolerance)
}

fn canonical_identity(identity: Option<&Value>) -> Result<String, String> {
    match identity {
        Some(identity) => serde_json::to_string(identity)
            .map_err(|error| format!("failed to encode persistent identity: {error}")),
        None => Ok("null".to_owned()),
    }
}

fn identity_report(
    encoded: &str,
    aov: &scena::SceneHostSemanticAovCaptureV1,
    manifest: &scena::SceneRecipeBuildV1,
) -> Result<Value, String> {
    let persistent_identity: Value = serde_json::from_str(encoded)
        .map_err(|error| format!("failed to decode persistent identity: {error}"))?;
    Ok(json!({
        "persistent_identity": persistent_identity,
        "identity_scope": aov.identity_scope,
        "manifest_schema": manifest.schema,
    }))
}
