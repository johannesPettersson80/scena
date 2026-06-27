use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value};

use crate::scene::recipe::types::SceneRecipeDiagnosticV1;
use crate::scene::recipe::validation::diagnostic;

/// Minimum sphere `segments` for a near-mirror material to read as a smooth
/// reflection. A coarser sphere reveals its polygon facets once the surface
/// becomes a sharp mirror.
pub(super) const MIN_MIRROR_SPHERE_SEGMENTS: u64 = 256;
/// Minimum sphere `rings` paired with [`MIN_MIRROR_SPHERE_SEGMENTS`] for a
/// smooth near-mirror sphere.
const MIN_MIRROR_SPHERE_RINGS: u64 = 192;
/// Effective roughness at or below this is treated as a near mirror.
const MIRROR_ROUGHNESS_MAX: f64 = 0.1;
/// Sphere tessellation assumed when a recipe omits it. Must match the geometry
/// construction default in `scene_host::recipe::authoring::geometry`.
const DEFAULT_SPHERE_SEGMENTS: u64 = 64;
const DEFAULT_SPHERE_RINGS: u64 = 48;

/// Warn when a renderable pairs a near-mirror material with a low-tessellation
/// sphere. A sharp reflection on a coarse sphere shows the polygon facets, so
/// this tells authors (and agents) before a chrome subject renders as a blocky,
/// faceted ball — the mistake is otherwise invisible until the material becomes
/// a mirror.
pub(super) fn validate_reflective_sphere_tessellation(
    object: &Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let sphere_tessellation = collect_sphere_tessellation(object.get("geometries"));
    if sphere_tessellation.is_empty() {
        return;
    }
    let mirror_materials = collect_mirror_materials(object.get("materials"));
    if mirror_materials.is_empty() {
        return;
    }
    for (section, label) in [("nodes", "node"), ("instance_sets", "instance set")] {
        let Some(items) = object.get(section).and_then(Value::as_array) else {
            continue;
        };
        for (index, item) in items.iter().enumerate() {
            let Some(item) = item.as_object() else {
                continue;
            };
            let (Some(geometry), Some(material)) = (
                item.get("geometry").and_then(Value::as_str),
                item.get("material").and_then(Value::as_str),
            ) else {
                continue;
            };
            let Some(tessellation) = sphere_tessellation.get(geometry) else {
                continue;
            };
            if !mirror_materials.contains(material)
                || (tessellation.segments >= MIN_MIRROR_SPHERE_SEGMENTS
                    && tessellation.rings >= MIN_MIRROR_SPHERE_RINGS)
            {
                continue;
            }
            diagnostics.push(diagnostic(
                "reflective_sphere_low_tessellation",
                "warning",
                format!("$.{section}[{index}]"),
                format!(
                    "{label} pairs near-mirror material '{material}' with sphere '{geometry}' at \
                     {} segments and {} rings; a sharp reflection will show the sphere's polygon facets",
                    tessellation.segments, tessellation.rings
                ),
                format!(
                    "raise the sphere to at least segments:{MIN_MIRROR_SPHERE_SEGMENTS}, \
                     rings:{MIN_MIRROR_SPHERE_RINGS} for smooth mirror reflections"
                ),
                None,
                false,
            ));
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SphereTessellation {
    segments: u64,
    rings: u64,
}

fn collect_sphere_tessellation(value: Option<&Value>) -> HashMap<String, SphereTessellation> {
    let mut map = HashMap::new();
    let Some(items) = value.and_then(Value::as_array) else {
        return map;
    };
    for item in items {
        let Some(item) = item.as_object() else {
            continue;
        };
        let Some(id) = item.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(primitive) = item.get("primitive").and_then(Value::as_object) else {
            continue;
        };
        if primitive.get("kind").and_then(Value::as_str) != Some("sphere") {
            continue;
        }
        let segments = primitive
            .get("segments")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_SPHERE_SEGMENTS);
        let rings = primitive
            .get("rings")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_SPHERE_RINGS);
        map.insert(id.to_owned(), SphereTessellation { segments, rings });
    }
    map
}

fn collect_mirror_materials(value: Option<&Value>) -> HashSet<String> {
    let mut set = HashSet::new();
    let Some(items) = value.and_then(Value::as_array) else {
        return set;
    };
    for item in items {
        let Some(item) = item.as_object() else {
            continue;
        };
        let Some(id) = item.get("id").and_then(Value::as_str) else {
            continue;
        };
        let is_mirror = item.get("preset").and_then(Value::as_str) == Some("chrome")
            || item
                .get("roughness")
                .and_then(Value::as_f64)
                .is_some_and(|roughness| roughness <= MIRROR_ROUGHNESS_MAX);
        if is_mirror {
            set.insert(id.to_owned());
        }
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn diagnostics_for(value: Value) -> Vec<SceneRecipeDiagnosticV1> {
        let mut diagnostics = Vec::new();
        validate_reflective_sphere_tessellation(value.as_object().unwrap(), &mut diagnostics);
        diagnostics
    }

    #[test]
    fn warns_when_chrome_material_uses_low_tessellation_sphere() {
        let diagnostics = diagnostics_for(json!({
            "geometries": [{ "id": "ball", "primitive": { "kind": "sphere", "radius": 0.5, "segments": 96 } }],
            "materials": [{ "id": "m", "preset": "chrome" }],
            "nodes": [{ "id": "n", "geometry": "ball", "material": "m" }],
        }));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "reflective_sphere_low_tessellation");
        assert_eq!(diagnostics[0].severity, "warning");
    }

    #[test]
    fn warns_when_default_segments_sphere_is_a_mirror() {
        // Omitting segments falls back to the default, which is below the mirror
        // threshold — the most likely agent mistake.
        let diagnostics = diagnostics_for(json!({
            "geometries": [{ "id": "ball", "primitive": { "kind": "sphere", "radius": 0.5 } }],
            "materials": [{ "id": "m", "kind": "pbr_metallic_roughness", "metallic": 1.0, "roughness": 0.02 }],
            "nodes": [{ "id": "n", "geometry": "ball", "material": "m" }],
        }));
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn warns_when_chrome_preset_has_roughness_override() {
        let diagnostics = diagnostics_for(json!({
            "geometries": [{ "id": "ball", "primitive": { "kind": "sphere", "radius": 0.5, "segments": 96, "rings": 64 } }],
            "materials": [{ "id": "m", "preset": "chrome", "roughness": 0.42 }],
            "nodes": [{ "id": "n", "geometry": "ball", "material": "m" }],
        }));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "reflective_sphere_low_tessellation");
    }

    #[test]
    fn no_warning_for_dense_sphere() {
        let diagnostics = diagnostics_for(json!({
            "geometries": [{ "id": "ball", "primitive": { "kind": "sphere", "radius": 0.5, "segments": 256, "rings": 192 } }],
            "materials": [{ "id": "m", "preset": "chrome" }],
            "nodes": [{ "id": "n", "geometry": "ball", "material": "m" }],
        }));
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn warns_when_segments_are_dense_but_rings_are_low() {
        let diagnostics = diagnostics_for(json!({
            "geometries": [{ "id": "ball", "primitive": { "kind": "sphere", "radius": 0.5, "segments": 256 } }],
            "materials": [{ "id": "m", "preset": "chrome" }],
            "nodes": [{ "id": "n", "geometry": "ball", "material": "m" }],
        }));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "reflective_sphere_low_tessellation");
        assert!(
            diagnostics[0].message.contains("256 segments and 48 rings"),
            "{diagnostics:#?}"
        );
    }

    #[test]
    fn no_warning_for_rough_material_on_coarse_sphere() {
        // Matte/rough materials hide facets, so a coarse sphere is fine for them.
        let diagnostics = diagnostics_for(json!({
            "geometries": [{ "id": "ball", "primitive": { "kind": "sphere", "radius": 0.5, "segments": 32 } }],
            "materials": [{ "id": "m", "preset": "metal", "roughness": 0.42 }],
            "nodes": [{ "id": "n", "geometry": "ball", "material": "m" }],
        }));
        assert!(diagnostics.is_empty());
    }
}
