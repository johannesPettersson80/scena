//! A material's surface finish, read from `extras.scena.surface_finish`.
//!
//! glTF has no extension for how a part was manufactured, so a file states it
//! in the material's `extras`, where scena keeps its other annotations:
//!
//! ```json
//! "extras": { "scena": { "surface_finish": "cast_metal" } }
//! ```
//!
//! or, to set more than the kind,
//!
//! ```json
//! "extras": { "scena": { "surface_finish": {
//!     "kind": "painted_metal", "scale": 1.5, "relief": 0.8, "wear": 0.3,
//!     "grime": 0.4, "edge_radius_m": 0.002, "axis": [1, 0, 0], "seed": 7
//! } } }
//! ```
//!
//! A finish that is present but wrong is an error, not a silently plain
//! material: the file asked for something the import cannot deliver.

use ::gltf::Material;
use serde_json::Value as JsonValue;

use crate::diagnostics::AssetError;
use crate::material::SurfaceFinish;

use super::super::{AssetPath, PhotographicSurfaceKind};

const FIELDS: [&str; 8] = [
    "kind",
    "scale",
    "relief",
    "wear",
    "grime",
    "edge_radius_m",
    "axis",
    "seed",
];

pub(super) fn parse_surface_finish(
    path: &AssetPath,
    material_index: usize,
    material: &Material<'_>,
) -> Result<Option<SurfaceFinish>, AssetError> {
    let Some(raw_extras) = material.extras().as_ref() else {
        return Ok(None);
    };
    let place = format!("materials[{material_index}].extras");
    let invalid = |reason: String| AssetError::Parse {
        path: path.as_str().to_owned(),
        reason,
    };
    let extras = serde_json::from_str::<JsonValue>(raw_extras.get())
        .map_err(|error| invalid(format!("{place} is invalid JSON: {error}")))?;
    let Some(finish) = extras
        .get("scena")
        .and_then(|scena| scena.get("surface_finish"))
    else {
        return Ok(None);
    };
    let place = format!("{place}.scena.surface_finish");
    let kind = |name: &JsonValue| {
        name.as_str()
            .and_then(PhotographicSurfaceKind::from_name)
            .ok_or_else(|| {
                invalid(format!(
                    "{place} names no known finish; the kinds are {}",
                    PhotographicSurfaceKind::NAMES.join(", ")
                ))
            })
    };
    let JsonValue::Object(fields) = finish else {
        return kind(finish).map(|kind| Some(SurfaceFinish::new(kind)));
    };
    if let Some(unknown) = fields.keys().find(|key| !FIELDS.contains(&key.as_str())) {
        return Err(invalid(format!(
            "{place}.{unknown} is not a setting of a finish; the settings are {}",
            FIELDS.join(", ")
        )));
    }
    let mut parsed = SurfaceFinish::new(kind(fields.get("kind").unwrap_or(&JsonValue::Null))?);
    let number = |name: &str, low: f32, high: f32| -> Result<Option<f32>, AssetError> {
        let Some(value) = fields.get(name) else {
            return Ok(None);
        };
        value
            .as_f64()
            .map(|value| value as f32)
            .filter(|value| (low..=high).contains(value))
            .map(Some)
            .ok_or_else(|| {
                invalid(format!(
                    "{place}.{name} must be a number from {low} to {high}"
                ))
            })
    };
    if let Some(scale) = number("scale", 0.02, SurfaceFinish::MAX_SCALE)? {
        parsed = parsed.with_scale(scale);
    }
    if let Some(relief) = number("relief", 0.0, SurfaceFinish::MAX_RELIEF)? {
        parsed = parsed.with_relief(relief);
    }
    if let Some(wear) = number("wear", 0.0, 1.0)? {
        parsed = parsed.with_wear(wear);
    }
    if let Some(grime) = number("grime", 0.0, 1.0)? {
        parsed = parsed.with_grime(grime);
    }
    if let Some(radius) = number("edge_radius_m", 0.0, SurfaceFinish::MAX_EDGE_RADIUS_M)? {
        parsed = parsed.with_edge_radius_m(radius);
    }
    if let Some(axis) = fields.get("axis") {
        let axis = axis
            .as_array()
            .filter(|axis| axis.len() == 3)
            .and_then(|axis| {
                let component = |index: usize| axis[index].as_f64().map(|value| value as f32);
                Some(glam::Vec3::new(component(0)?, component(1)?, component(2)?))
            })
            .filter(|axis| axis.is_finite() && axis.length() > 0.0)
            .ok_or_else(|| {
                invalid(format!(
                    "{place}.axis must be three numbers that are not all zero"
                ))
            })?;
        parsed = parsed.with_axis(axis);
    }
    if let Some(seed) = fields.get("seed") {
        let seed = seed
            .as_u64()
            .and_then(|seed| u32::try_from(seed).ok())
            .ok_or_else(|| invalid(format!("{place}.seed must be a whole number below 2^32")))?;
        parsed = parsed.with_seed(seed);
    }
    Ok(Some(parsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finish_of(extras: &str) -> Result<Option<SurfaceFinish>, AssetError> {
        let json = format!(
            r#"{{"asset":{{"version":"2.0"}},"materials":[{{"name":"m","extras":{extras}}}]}}"#
        );
        let gltf = ::gltf::Gltf::from_slice(json.as_bytes()).expect("the document parses");
        let material = gltf.document.materials().next().expect("one material");
        parse_surface_finish(&AssetPath::from("part.gltf"), 0, &material)
    }

    fn reason(extras: &str) -> String {
        match finish_of(extras) {
            Err(AssetError::Parse { path, reason }) => {
                assert_eq!(path, "part.gltf");
                reason
            }
            other => panic!("expected a parse error, got {other:?}"),
        }
    }

    #[test]
    fn a_material_without_the_annotation_has_no_finish() {
        assert_eq!(finish_of("{}").expect("parses"), None);
        assert_eq!(
            finish_of(r#"{"scena":{"anchors":[]}}"#).expect("parses"),
            None
        );
        assert_eq!(
            finish_of(r#"{"other":"cast_metal"}"#).expect("parses"),
            None
        );
    }

    #[test]
    fn a_name_alone_selects_the_finish_as_it_comes() {
        let finish = finish_of(r#"{"scena":{"surface_finish":"cast_metal"}}"#)
            .expect("parses")
            .expect("a finish");
        assert_eq!(
            finish,
            SurfaceFinish::new(PhotographicSurfaceKind::CastMetal)
        );
    }

    #[test]
    fn every_setting_is_read() {
        let finish = finish_of(
            r#"{"scena":{"surface_finish":{"kind":"machined_metal","scale":3,"relief":0.5,
                "wear":0.25,"grime":0.75,"edge_radius_m":0.002,"axis":[2,0,0],"seed":9}}}"#,
        )
        .expect("parses")
        .expect("a finish");
        let expected = SurfaceFinish::new(PhotographicSurfaceKind::MachinedMetal)
            .with_scale(3.0)
            .with_relief(0.5)
            .with_wear(0.25)
            .with_grime(0.75)
            .with_edge_radius_m(0.002)
            .with_axis(glam::Vec3::X)
            .with_seed(9);
        assert_eq!(finish, expected);
    }

    #[test]
    fn a_wrong_finish_is_an_error_that_says_where_and_why() {
        let unknown = reason(r#"{"scena":{"surface_finish":"shiny"}}"#);
        assert!(unknown.contains("materials[0].extras.scena.surface_finish names no known finish"));
        assert!(unknown.contains("cast_metal") && unknown.contains("fabric"));
        assert!(reason(r#"{"scena":{"surface_finish":{"scale":2}}}"#).contains("no known finish"));
        assert!(reason(r#"{"scena":{"surface_finish":7}}"#).contains("no known finish"));
        let stray = reason(r#"{"scena":{"surface_finish":{"kind":"rubber","gloss":1}}}"#);
        assert!(
            stray.contains("surface_finish.gloss is not a setting"),
            "{stray}"
        );
        for (setting, message) in [
            (r#""wear":2"#, "wear must be a number from 0 to 1"),
            (r#""grime":"lots""#, "grime must be a number from 0 to 1"),
            (r#""scale":0"#, "scale must be a number from 0.02 to 50"),
            (r#""relief":-1"#, "relief must be a number from 0 to 4"),
            (
                r#""edge_radius_m":5"#,
                "edge_radius_m must be a number from 0 to 1",
            ),
            (
                r#""axis":[0,0,0]"#,
                "axis must be three numbers that are not all zero",
            ),
            (r#""axis":[1,2]"#, "axis must be three numbers"),
            (r#""seed":-4"#, "seed must be a whole number below 2^32"),
            (r#""seed":1.5"#, "seed must be a whole number below 2^32"),
        ] {
            let extras =
                format!(r#"{{"scena":{{"surface_finish":{{"kind":"rubber",{setting}}}}}}}"#);
            let reason = reason(&extras);
            assert!(reason.contains(message), "{setting}: {reason}");
        }
    }
}
