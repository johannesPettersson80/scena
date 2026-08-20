use super::{EnvironmentDesc, EnvironmentSidecarProfile, PreparedEnvironmentCubemap};

#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
pub(super) fn environment_now_ms() -> f64 {
    js_sys::Date::now()
}

#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
pub(super) fn log_environment_step(label: &str, start_ms: f64) -> f64 {
    let now = environment_now_ms();
    if crate::diagnostics::browser_timing_enabled() {
        web_sys::console::log_1(
            &format!("[scena-demo] environment {label}: {:.1}ms", now - start_ms).into(),
        );
    }
    now
}

pub(super) fn warn_environment_sidecar_profile_mismatch(
    environment: &EnvironmentDesc,
    requested: EnvironmentSidecarProfile,
    actual: EnvironmentSidecarProfile,
) {
    let message = format!(
        "scena environment warning: sidecar '{}' has profile {}, but this backend requested {}; ignoring the sidecar and baking IBL from the HDR source instead",
        environment.source_path().as_str(),
        actual.name(),
        requested.name()
    );
    #[cfg(target_arch = "wasm32")]
    web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(&message));
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("{message}");
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn debug_report_environment(prepared: &PreparedEnvironmentCubemap) {
    if std::env::var("SCENA_DEBUG_LOG_ENVIRONMENT").as_deref() != Ok("1") {
        return;
    }
    eprintln!(
        "[env] resolution={} mip_count={} brdf_lut={}x{}",
        prepared.resolution, prepared.mip_count, prepared.brdf_lut_size, prepared.brdf_lut_size
    );
    for (level, faces) in prepared.mips.iter().enumerate() {
        let size = (prepared.resolution >> level).max(1) as usize;
        let mut total_step = 0.0_f64;
        let mut steps = 0_u64;
        let mut peak = 0.0_f32;
        let mut mean = 0.0_f64;
        let mut samples = 0_u64;
        for face in faces {
            for y in 0..size {
                for x in 0..size.saturating_sub(1) {
                    let a = face.get((y * size + x) * 4).copied().unwrap_or(0.0);
                    let b = face.get((y * size + x + 1) * 4).copied().unwrap_or(0.0);
                    total_step += f64::from((b - a).abs());
                    steps += 1;
                }
            }
            for texel in face.chunks_exact(4) {
                mean += f64::from(texel[0]);
                peak = peak.max(texel[0]);
                samples += 1;
            }
        }
        let mean = if samples > 0 {
            mean / samples as f64
        } else {
            0.0
        };
        let step = if steps > 0 {
            total_step / steps as f64
        } else {
            0.0
        };
        eprintln!(
            "[env] mip {level}: {size}x{size}x6  mean_R={mean:.4} peak_R={peak:.4} mean|neighbour delta|={step:.5}  relative={:.4}",
            if mean > 1.0e-6 { step / mean } else { 0.0 }
        );
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn debug_report_environment(_prepared: &PreparedEnvironmentCubemap) {}
