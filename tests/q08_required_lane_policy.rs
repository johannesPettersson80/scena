#![cfg(not(target_arch = "wasm32"))]

//! Q08 required physical-lane execution policy; this is not an image parity sweep.

#[allow(dead_code)]
mod support;

use support::parity::{ParityExecutionPolicy, parity_execution_policy};

#[test]
fn ordinary_platform_test_without_forcing_or_adapter_returns_diagnostic_skip_policy() {
    assert_eq!(
        parity_execution_policy(false, false, false),
        ParityExecutionPolicy::SkipDiagnostic,
        "ordinary macOS/Windows all-target tests do not execute GPU parity without a required lane"
    );
}

#[test]
fn strict_parity_policy_cannot_be_downgraded_by_missing_lavapipe() {
    assert_eq!(
        parity_execution_policy(true, false, false),
        ParityExecutionPolicy::RequiredPhysicalHardware,
    );
    assert_eq!(
        parity_execution_policy(false, true, false),
        ParityExecutionPolicy::DiagnosticGpuConformance,
    );
    assert_eq!(
        parity_execution_policy(false, false, true),
        ParityExecutionPolicy::DiagnosticGpuConformance,
    );
}
