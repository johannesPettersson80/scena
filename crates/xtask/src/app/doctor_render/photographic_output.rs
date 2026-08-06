//! Behavioural gate: judge the renderer on the pixels it produced.
//!
//! Every other rule in `doctor_render` pins source text. That catches deletion
//! and drift, but a source-substring rule cannot tell a renderer that works from
//! one that merely still compiles - the photorealism work passed all of them
//! while the flagship command produced clay-looking output. This rule reads the
//! metrics the camera-behavior proof measured off the rendered PNG and holds
//! them to the same fixture bands the test used, so the quality claim is
//! re-checked against evidence rather than against the code that made it.

use crate::app::prelude::*;

/// Written by `photo_render_camera_behavior_is_easy_path_for_imported_asset`
/// in `tests/photo_render_cli.rs`, which is the test that measures these
/// metrics off the rendered PNG.
const METRICS_ARTIFACT: &str =
    "target/gate-artifacts/photographic-output/camera_behavior_metrics.json";

/// The fixture is the single source of truth for the acceptable bands; this rule
/// deliberately does not carry its own copy of the numbers.
const FIXTURE: &str = "tests/assets/photo/camera_behavior_cad_terminal_block.fixture.json";

/// Which side of a fixture band is authoritative for a metric.
///
/// The fixture declares only the bounds that mean something: a specular
/// headroom has a floor and no ceiling, a clip fraction has a ceiling and no
/// floor. Demanding both would fail every honest band.
#[derive(Clone, Copy)]
enum Bound {
    Floor,
    Ceiling,
    Range,
}

/// Metrics that must be present, and the bound each is held to.
const REQUIRED_METRICS: &[(&str, Bound)] = &[
    ("subject_mean_luminance_srgb8", Bound::Range),
    ("subject_specular_headroom_srgb8", Bound::Floor),
    ("subject_low_clip_fraction", Bound::Ceiling),
    ("subject_color_frame_boundary_separation", Bound::Floor),
];

pub(crate) fn check_photographic_output_metrics(root: &Path, findings: &mut Vec<Finding>) {
    let artifact = root.join(METRICS_ARTIFACT);
    let Ok(raw) = fs::read(&artifact) else {
        // A fresh checkout has not rendered anything yet. Absence is only a
        // failure on a lane that promised to produce the evidence.
        if env::var("SCENA_DOCTOR_REQUIRE_GENERATED_ARTIFACTS").as_deref() == Ok("1") {
            findings.push(Finding::new(
                "ARCH-RENDER-QUALITY",
                format!(
                    "{METRICS_ARTIFACT} is missing, so no rendered-output evidence backs the \
                     photographic quality claim; run \
                     photo_render_camera_behavior_is_easy_path_for_imported_asset in \
                     tests/photo_render_cli.rs"
                ),
            ));
        }
        return;
    };

    let Ok(metrics) = serde_json::from_slice::<serde_json::Value>(&raw) else {
        findings.push(Finding::new(
            "ARCH-RENDER-QUALITY",
            format!("{METRICS_ARTIFACT} is not valid JSON"),
        ));
        return;
    };
    if metrics["schema"] != "scena.photographic_output_metrics.v1" {
        findings.push(Finding::new(
            "ARCH-RENDER-QUALITY",
            format!(
                "{METRICS_ARTIFACT} declares schema {}, expected \
                 scena.photographic_output_metrics.v1",
                metrics["schema"]
            ),
        ));
        return;
    }

    let fixture_path = root.join(FIXTURE);
    let Ok(fixture_raw) = fs::read(&fixture_path) else {
        findings.push(Finding::new(
            "ARCH-RENDER-QUALITY",
            format!("{FIXTURE} is missing, so the measured metrics cannot be judged"),
        ));
        return;
    };
    let Ok(fixture) = serde_json::from_slice::<serde_json::Value>(&fixture_raw) else {
        findings.push(Finding::new(
            "ARCH-RENDER-QUALITY",
            format!("{FIXTURE} is not valid JSON"),
        ));
        return;
    };

    for (metric, bound) in REQUIRED_METRICS {
        let Some(value) = metrics["measured"][metric].as_f64() else {
            findings.push(Finding::new(
                "ARCH-RENDER-QUALITY",
                format!("{METRICS_ARTIFACT} is missing the measured value for {metric}"),
            ));
            continue;
        };
        let band = &fixture["quality_bands"][metric];
        let floor = matches!(bound, Bound::Floor | Bound::Range)
            .then(|| band["min"].as_f64())
            .map(|min| min.ok_or("min"));
        let ceiling = matches!(bound, Bound::Ceiling | Bound::Range)
            .then(|| band["max"].as_f64())
            .map(|max| max.ok_or("max"));
        for missing in [floor, ceiling].into_iter().flatten() {
            if let Err(side) = missing {
                findings.push(Finding::new(
                    "ARCH-RENDER-QUALITY",
                    format!("{FIXTURE} does not bound {metric} with a numeric {side}"),
                ));
            }
        }
        if let Some(Ok(min)) = floor
            && value < min
        {
            findings.push(Finding::new(
                "ARCH-RENDER-QUALITY",
                format!(
                    "rendered output regressed: {metric} measured {value}, below the fixture \
                     floor {min}"
                ),
            ));
        }
        if let Some(Ok(max)) = ceiling
            && value > max
        {
            findings.push(Finding::new(
                "ARCH-RENDER-QUALITY",
                format!(
                    "rendered output regressed: {metric} measured {value}, above the fixture \
                     ceiling {max}"
                ),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FIXTURE, METRICS_ARTIFACT, check_photographic_output_metrics};
    use crate::app::prelude::{Path, PathBuf, env, fs};

    fn scratch_root(name: &str) -> PathBuf {
        let root = env::temp_dir().join(format!(
            "scena-photographic-output-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("tests/assets/photo")).expect("fixture directory creates");
        fs::write(
            root.join(FIXTURE),
            // Mirrors the shipped fixture exactly: a floor-only headroom band
            // and a ceiling-only clip band. Inventing min+max on every band here
            // is what let a rule that demands both pass its own tests and then
            // fail on the real file.
            br#"{"quality_bands":{
                "subject_mean_luminance_srgb8":{"min":80.0,"max":100.0},
                "subject_specular_headroom_srgb8":{"min":24.0},
                "subject_low_clip_fraction":{"max":0.2},
                "subject_color_frame_boundary_separation":{"min":0.01}}}"#,
        )
        .expect("fixture writes");
        root
    }

    fn write_metrics(root: &Path, measured: &str) {
        let path = root.join(METRICS_ARTIFACT);
        fs::create_dir_all(path.parent().expect("artifact has a parent"))
            .expect("artifact directory creates");
        fs::write(
            &path,
            format!(r#"{{"schema":"scena.photographic_output_metrics.v1","measured":{measured}}}"#),
        )
        .expect("artifact writes");
    }

    #[test]
    fn in_band_rendered_metrics_pass() {
        let root = scratch_root("in-band");
        write_metrics(
            &root,
            r#"{"subject_mean_luminance_srgb8":88.0,
                "subject_specular_headroom_srgb8":74.0,
                "subject_low_clip_fraction":0.03,
                "subject_color_frame_boundary_separation":0.24}"#,
        );
        let mut findings = Vec::new();
        check_photographic_output_metrics(&root, &mut findings);
        assert!(
            findings.is_empty(),
            "in-band output must pass: {findings:?}"
        );
    }

    /// The whole point of the rule: a renderer that still compiles but produces
    /// flat, low-headroom output must fail, which no source-substring pin does.
    #[test]
    fn flat_specular_output_fails_even_though_the_source_is_unchanged() {
        let root = scratch_root("flat");
        write_metrics(
            &root,
            r#"{"subject_mean_luminance_srgb8":88.0,
                "subject_specular_headroom_srgb8":11.0,
                "subject_low_clip_fraction":0.03,
                "subject_color_frame_boundary_separation":0.24}"#,
        );
        let mut findings = Vec::new();
        check_photographic_output_metrics(&root, &mut findings);
        assert_eq!(findings.len(), 1, "expected one finding, got {findings:?}");
        assert!(
            findings[0]
                .message
                .contains("subject_specular_headroom_srgb8"),
            "the finding must name the metric that regressed: {}",
            findings[0].message
        );
    }

    #[test]
    fn missing_color_frame_agreement_metric_fails() {
        let root = scratch_root("missing-color-frame-agreement");
        write_metrics(
            &root,
            r#"{"subject_mean_luminance_srgb8":88.0,
                "subject_specular_headroom_srgb8":74.0,
                "subject_low_clip_fraction":0.03}"#,
        );
        let mut findings = Vec::new();
        check_photographic_output_metrics(&root, &mut findings);
        assert!(
            findings.iter().any(|finding| {
                finding
                    .message
                    .contains("subject_color_frame_boundary_separation")
            }),
            "the behavioral gate must reject evidence that cannot prove the beauty frame agrees with its semantic mask: {findings:?}"
        );
    }

    #[test]
    fn missing_evidence_is_advisory_locally_and_blocking_on_a_lane_that_promised_it() {
        let root = scratch_root("missing");
        let mut findings = Vec::new();
        check_photographic_output_metrics(&root, &mut findings);
        assert!(
            findings.is_empty(),
            "a fresh checkout has rendered nothing yet: {findings:?}"
        );

        // SAFETY: single-threaded assertion over one process-wide flag, and the
        // value is restored before the test returns.
        unsafe { env::set_var("SCENA_DOCTOR_REQUIRE_GENERATED_ARTIFACTS", "1") };
        let mut required = Vec::new();
        check_photographic_output_metrics(&root, &mut required);
        unsafe { env::remove_var("SCENA_DOCTOR_REQUIRE_GENERATED_ARTIFACTS") };
        assert_eq!(
            required.len(),
            1,
            "a lane that promised the evidence must fail without it: {required:?}"
        );
    }
}
