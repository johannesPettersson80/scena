use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::Path;

pub(super) fn verify_reference_expectations(
    references: &[scena::SceneRecipeReferenceExpectationV1],
    capture: &scena::CaptureRgba8,
    recipe_dir: &Path,
) -> Result<Vec<scena::RenderQualityCheckV1>, String> {
    let mut checks = Vec::new();
    for reference in references {
        let reference_path = recipe_dir.join(&reference.image);
        let expected = decode_png_rgba8(&reference_path).map_err(|error| {
            format!(
                "failed to load reference image '{}': {error}",
                reference_path.display()
            )
        })?;
        let Some(metrics) = scena::reference_quality_metrics(
            &capture.rgba8,
            &expected.rgba8,
            capture.descriptor.width,
            capture.descriptor.height,
        ) else {
            checks.push(reference_check(ReferenceCheckInput {
                reference,
                code: "reference_dimensions_mismatch",
                severity: "error",
                observed_key: "width",
                observed: capture.descriptor.width as f32,
                threshold_key: "expected_width",
                threshold: expected.width as f32,
                width: capture.descriptor.width,
                height: capture.descriptor.height,
                fix_hint: "use a reference image with the exact capture dimensions",
            }));
            checks.push(reference_check(ReferenceCheckInput {
                reference,
                code: "reference_dimensions_mismatch",
                severity: "error",
                observed_key: "height",
                observed: capture.descriptor.height as f32,
                threshold_key: "expected_height",
                threshold: expected.height as f32,
                width: capture.descriptor.width,
                height: capture.descriptor.height,
                fix_hint: "use a reference image with the exact capture dimensions",
            }));
            continue;
        };
        match reference.metric.as_str() {
            "rgba_abs_diff" => {
                let threshold = reference.mean_max.unwrap_or(0.0) as f32;
                if metrics.mean_abs_diff > threshold {
                    checks.push(reference_check(ReferenceCheckInput {
                        reference,
                        code: "reference_rgba_abs_diff_exceeded",
                        severity: "error",
                        observed_key: "mean_abs_diff",
                        observed: metrics.mean_abs_diff,
                        threshold_key: "mean_max",
                        threshold,
                        width: capture.descriptor.width,
                        height: capture.descriptor.height,
                        fix_hint: "update the render or intentionally refresh the committed reference image after review",
                    }));
                }
            }
            "delta_e2000" => {
                let threshold = reference.mean_max.unwrap_or(1.0) as f32;
                if metrics.mean_delta_e2000 > threshold {
                    checks.push(reference_check(ReferenceCheckInput {
                        reference,
                        code: "reference_delta_e2000_exceeded",
                        severity: "error",
                        observed_key: "mean_delta_e2000",
                        observed: metrics.mean_delta_e2000,
                        threshold_key: "mean_max",
                        threshold,
                        width: capture.descriptor.width,
                        height: capture.descriptor.height,
                        fix_hint: "fix the material/color regression or refresh the reference after review",
                    }));
                }
            }
            "ssim" => {
                let threshold = reference.min_ssim.unwrap_or(0.99) as f32;
                if metrics.ssim < threshold {
                    checks.push(reference_check(ReferenceCheckInput {
                        reference,
                        code: "reference_ssim_too_low",
                        severity: "error",
                        observed_key: "ssim",
                        observed: metrics.ssim,
                        threshold_key: "min_ssim",
                        threshold,
                        width: capture.descriptor.width,
                        height: capture.descriptor.height,
                        fix_hint: "fix the structural render difference or refresh the reference after review",
                    }));
                }
            }
            _ => {}
        }
    }
    Ok(checks)
}

pub(super) fn refresh_quality_summary(report: &mut scena::RenderQualityReportV1) {
    let errors = report
        .checks
        .iter()
        .filter(|check| check.severity == "error")
        .count();
    let warnings = report
        .checks
        .iter()
        .filter(|check| check.severity == "warning")
        .count();
    report.ok = errors == 0;
    report.summary = scena::RenderQualitySummaryV1 {
        checks: report.checks.len(),
        errors,
        warnings,
    };
}

struct ReferenceCheckInput<'a> {
    reference: &'a scena::SceneRecipeReferenceExpectationV1,
    code: &'a str,
    severity: &'a str,
    observed_key: &'a str,
    observed: f32,
    threshold_key: &'a str,
    threshold: f32,
    width: u32,
    height: u32,
    fix_hint: &'a str,
}

fn reference_check(check: ReferenceCheckInput<'_>) -> scena::RenderQualityCheckV1 {
    scena::RenderQualityCheckV1 {
        id: check.reference.id.clone(),
        code: check.code.to_owned(),
        severity: check.severity.to_owned(),
        region: scena::RenderQualityRegionV1 {
            kind: "frame".to_owned(),
            handle: None,
            rect_css_px: Some(scena::RenderIntrospectionRectV1 {
                min_x: 0.0,
                min_y: 0.0,
                max_x: check.width as f32,
                max_y: check.height as f32,
                width: check.width as f32,
                height: check.height as f32,
            }),
        },
        observed: BTreeMap::from([(check.observed_key.to_owned(), round3(check.observed))]),
        threshold: BTreeMap::from([(check.threshold_key.to_owned(), round3(check.threshold))]),
        fix_hint: check.fix_hint.to_owned(),
    }
}

struct DecodedPng {
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
}

fn decode_png_rgba8(path: &Path) -> Result<DecodedPng, String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().map_err(|error| error.to_string())?;
    let Some(buffer_size) = reader.output_buffer_size() else {
        return Err("PNG output buffer size overflowed".to_owned());
    };
    let mut buffer = vec![0; buffer_size];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|error| error.to_string())?;
    let bytes = &buffer[..info.buffer_size()];
    let rgba8 = match info.color_type {
        png::ColorType::Rgba => bytes.to_vec(),
        png::ColorType::Rgb => {
            let mut rgba = Vec::with_capacity((info.width * info.height * 4) as usize);
            for pixel in bytes.chunks_exact(3) {
                rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
            }
            rgba
        }
        png::ColorType::Grayscale => {
            let mut rgba = Vec::with_capacity((info.width * info.height * 4) as usize);
            for value in bytes {
                rgba.extend_from_slice(&[*value, *value, *value, 255]);
            }
            rgba
        }
        _ => {
            return Err(format!(
                "unsupported reference PNG color type {:?}; use RGBA, RGB, or grayscale",
                info.color_type
            ));
        }
    };
    Ok(DecodedPng {
        width: info.width,
        height: info.height,
        rgba8,
    })
}

fn round3(value: f32) -> f32 {
    if value.is_finite() {
        (value * 1000.0).round() / 1000.0
    } else {
        0.0
    }
}
