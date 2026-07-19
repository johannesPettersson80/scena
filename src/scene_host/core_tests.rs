#![cfg(not(target_arch = "wasm32"))]

use super::SceneHostCore;
use crate::{Backend, BuildError, DefaultAssetFetcher, SceneHostErrorCode};

#[test]
fn c13_strict_headless_gpu_with_fetcher_propagates_injected_gpu_failure() {
    let error = SceneHostCore::<DefaultAssetFetcher>::headless_gpu_with_fetcher_using(
        DefaultAssetFetcher::default(),
        8,
        8,
        |_width, _height| {
            Err(BuildError::NoAdapter {
                backend: Backend::HeadlessGpu,
            })
        },
    )
    .expect_err("strict GPU construction must not return a CPU host");

    assert_eq!(error.code(), SceneHostErrorCode::Build);
    assert!(error.message().contains("HeadlessGpu"));
}

#[test]
fn c13_prefer_gpu_reports_injected_cpu_fallback() {
    let (host, report) =
        SceneHostCore::<DefaultAssetFetcher>::headless_prefer_gpu_with_fetcher_using(
            DefaultAssetFetcher::default(),
            8,
            8,
            |_width, _height| {
                Err(BuildError::RequestDevice {
                    backend: Backend::HeadlessGpu,
                })
            },
        )
        .expect("preferred GPU construction may explicitly fall back to CPU");

    assert_eq!(host.backend(), Backend::Headless);
    assert_eq!(report.requested_backend(), Backend::HeadlessGpu);
    assert_eq!(report.selected_backend(), Backend::Headless);
    assert!(report.fallback_used());
    assert_eq!(
        report.gpu_error(),
        Some(&BuildError::RequestDevice {
            backend: Backend::HeadlessGpu,
        })
    );
}

#[test]
fn c13_public_strict_constructor_never_returns_cpu_backend() {
    match SceneHostCore::headless_gpu(8, 8) {
        Ok(host) => assert_eq!(host.backend(), Backend::HeadlessGpu),
        Err(error) => assert_eq!(error.code(), SceneHostErrorCode::Build),
    }
}
