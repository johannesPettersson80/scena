use std::path::Path;

use serde_json::{Value, json};

pub(super) use crate::scena_recipe::capture_sequence::view::subject_bounds;
use crate::scena_recipe::capture_sequence::view::{CanonicalView, SubjectBounds};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ViewKind {
    BroadFace,
    TopFeatures,
    Overview,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CameraSpec {
    pub(super) eye: scena::Vec3,
    pub(super) target: scena::Vec3,
    pub(super) up: scena::Vec3,
    pub(super) fov_degrees: f64,
}

impl ViewKind {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::BroadFace => "broad_face",
            Self::TopFeatures => "top_features",
            Self::Overview => "overview",
        }
    }

    pub(super) const fn purpose(self) -> &'static str {
        match self {
            Self::BroadFace => "principal broad face with the thin dimension facing the camera",
            Self::TopFeatures => "top or recess-revealing face for shallow CAD details",
            Self::Overview => "diagonal overview for silhouette and depth context",
        }
    }
}

pub(super) fn camera_for(kind: ViewKind, bounds: SubjectBounds) -> CameraSpec {
    let center = bounds.center();
    let extent = bounds.extent();
    let radius = bounds.radius().max(0.001);
    match kind {
        ViewKind::BroadFace => {
            let thin_axis = smallest_axis(extent);
            let other_axis = strongest_non_parallel_axis(extent, thin_axis);
            let dir = axis_vec(thin_axis);
            let fov_degrees: f64 = 24.0;
            let distance = camera_distance(radius, fov_degrees, 1.2);
            let up = up_for(dir);
            let side = axis_vec(other_axis);
            CameraSpec {
                eye: center + dir * distance + up * (radius * 0.12) + side * (radius * 0.08),
                target: center,
                up,
                fov_degrees,
            }
        }
        ViewKind::TopFeatures => {
            let canonical = CanonicalView::Top;
            let dir = canonical.ideal_eye_direction();
            let fov_degrees: f64 = 22.0;
            let distance = camera_distance(radius, fov_degrees, 1.25);
            CameraSpec {
                eye: center + dir * distance + scena::Vec3::Z * (radius * 0.04),
                target: center,
                up: canonical.screen_up(),
                fov_degrees,
            }
        }
        ViewKind::Overview => {
            let dir = scena::Vec3::new(1.0, 0.75, 1.0).normalize();
            let fov_degrees: f64 = 32.0;
            let distance = camera_distance(radius, fov_degrees, 1.4);
            CameraSpec {
                eye: center + dir * distance,
                target: center,
                up: scena::Vec3::Y,
                fov_degrees,
            }
        }
    }
}

pub(super) fn inspection_recipe(
    mut recipe: Value,
    kind: ViewKind,
    camera: CameraSpec,
    width: u32,
    height: u32,
    bounds: SubjectBounds,
) -> Value {
    apply_import_presentation_defaults(&mut recipe);
    recipe["cameras"] = if kind == ViewKind::BroadFace {
        json!([{
            "id": format!("cad_inspection_{}", kind.id()),
            "kind": "perspective",
            "fov_degrees": camera.fov_degrees,
            "active": true,
            "framing": {
                "mode": "principal_face",
                "fill": 0.76,
                "margin_px": 18.0
            }
        }])
    } else {
        json!([{
            "id": format!("cad_inspection_{}", kind.id()),
            "kind": "perspective",
            "fov_degrees": camera.fov_degrees,
            "active": true,
            "transform": {
                "kind": "look_at",
                "eye": vec3_json(camera.eye),
                "target": vec3_json(camera.target),
                "up": vec3_json(camera.up)
            }
        }])
    };
    recipe["lights"] = json!([{
        "id": "cad_studio",
        "kind": "studio_rig",
        "preset": "studio_rig"
    }]);
    recipe["scene"] = json!({
        "background": { "kind": "custom", "color": "#EEF1F4" },
        "grid": { "enabled": false }
    });
    recipe["render"] = json!({
        "profile": "industrial",
        "quality": "high",
        "anti_aliasing": "fxaa",
        "supersample": 2,
        "reconstruction": "gaussian",
        "ssao": { "radius_px": 12, "intensity": 0.55, "depth_threshold": 0.03 },
        "tonemapper": "aces",
        "exposure_ev": 0.25
    });
    recipe["expect"] = json!({
        "expect_bbox_fit": {
            "min": 0.08,
            "max": 0.96
        }
    });
    recipe["capture"] = json!({ "width": width, "height": height });
    recipe["metadata"]["cad_inspection"] = json!({
        "view": kind.id(),
        "source": "scena recipe inspect-cad",
        "presentation_only": true,
        "subject_bounds": {
            "min": vec3_json(bounds.min),
            "max": vec3_json(bounds.max),
            "extent": vec3_json(bounds.extent())
        }
    });
    recipe
}

fn apply_import_presentation_defaults(recipe: &mut Value) {
    let Some(imports) = recipe.get_mut("imports").and_then(Value::as_array_mut) else {
        return;
    };
    for import in imports {
        let Some(import) = import.as_object_mut() else {
            continue;
        };
        if !import.contains_key("material") {
            import.insert(
                "material".to_owned(),
                json!({
                    "base_color": "#9AA4AE",
                    "roughness": 0.78,
                    "metallic": 0.0
                }),
            );
        }
        if !import.contains_key("edge_emphasis") {
            import.insert(
                "edge_emphasis".to_owned(),
                json!({
                    "enabled": true,
                    "base_color": "#1F252C",
                    "stroke_width_px": 2.0,
                    "edge_angle_threshold_degrees": 18.0
                }),
            );
        }
    }
}

pub(super) fn rewrite_relative_imports(
    recipe: &mut Value,
    recipe_path: &Path,
) -> Result<(), String> {
    let recipe_dir = recipe_path.parent().unwrap_or_else(|| Path::new("."));
    let Some(imports) = recipe.get_mut("imports").and_then(Value::as_array_mut) else {
        return Ok(());
    };
    let cwd = std::env::current_dir().map_err(|error| format!("failed to read cwd: {error}"))?;
    for import in imports {
        let Some(uri) = import.get("uri").and_then(Value::as_str) else {
            continue;
        };
        if uri.contains("://") || Path::new(uri).is_absolute() {
            continue;
        }
        let recipe_relative = recipe_dir.join(uri);
        let path = if recipe_relative.exists() {
            recipe_relative
        } else {
            cwd.join(uri)
        };
        import["uri"] = Value::String(path.display().to_string());
    }
    Ok(())
}

pub(super) fn vec3_json(value: scena::Vec3) -> Value {
    json!([value.x, value.y, value.z])
}

fn camera_distance(radius: f32, fov_degrees: f64, scale: f32) -> f32 {
    let half_fov = (fov_degrees.to_radians() * 0.5) as f32;
    radius / half_fov.tan() * scale
}

fn smallest_axis(extent: scena::Vec3) -> usize {
    let mut axes = [(0, extent.x), (1, extent.y), (2, extent.z)];
    axes.sort_by(|left, right| left.1.total_cmp(&right.1));
    axes[0].0
}

fn strongest_non_parallel_axis(extent: scena::Vec3, excluded: usize) -> usize {
    let mut axes = [(0, extent.x), (1, extent.y), (2, extent.z)];
    axes.sort_by(|left, right| right.1.total_cmp(&left.1));
    axes.into_iter()
        .map(|(axis, _)| axis)
        .find(|axis| *axis != excluded)
        .unwrap_or(2)
}

fn axis_vec(axis: usize) -> scena::Vec3 {
    match axis {
        0 => scena::Vec3::X,
        1 => scena::Vec3::Y,
        _ => scena::Vec3::Z,
    }
}

fn up_for(direction: scena::Vec3) -> scena::Vec3 {
    if direction.normalize_or_zero().dot(scena::Vec3::Y).abs() > 0.9 {
        scena::Vec3::Z
    } else {
        scena::Vec3::Y
    }
}
