use super::scena_cli_error::CliFailure;
use std::time::{SystemTime, UNIX_EPOCH};

use scena::{
    Backend, BuildError, Capabilities, CapabilityConstraintProbeV1, CapabilityConstraintStatusV1,
    CapabilityProbeModeV1, CapabilityProbeStatusV1, CapabilityProbeUnavailableV1,
    CapabilityProbeV1, CapabilityReport, CapabilityTargetProbeV1, Renderer,
};

use super::scena_output::{CliOutcome, json_outcome, json_success};

pub(crate) fn run_capabilities_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let live = parse_args(args)?;
    if !live {
        let capabilities = Capabilities::headless();
        let mut report = CapabilityReport::new(capabilities, None).to_schema_report();
        report.probe = Some(static_probe(capabilities));
        return json_success(&report, "failed to serialize static capability report");
    }

    let probed_at_unix_ms = probe_timestamp_unix_ms();
    match Renderer::headless_gpu(1, 1) {
        Ok(renderer) => {
            let mut report = renderer.capability_report().to_schema_report();
            report.probe = renderer.live_capability_probe(probed_at_unix_ms);
            json_success(&report, "failed to serialize live capability report")
        }
        Err(error) => {
            let capabilities = Capabilities::for_backend(Backend::HeadlessGpu);
            let mut report = CapabilityReport::new(capabilities, None).to_schema_report();
            report.probe = Some(unavailable_probe(probed_at_unix_ms, &error));
            json_outcome(
                &report,
                1,
                "failed to serialize unavailable capability report",
            )
        }
    }
}

fn parse_args(args: &[String]) -> Result<bool, String> {
    let mut live = false;
    let mut json = false;
    for arg in args {
        match arg.as_str() {
            "--live" if !live => live = true,
            "--json" if !json => json = true,
            "--live" | "--json" => {
                return Err(format!("duplicate capabilities argument '{arg}'; {USAGE}"));
            }
            unknown => {
                return Err(format!(
                    "unknown capabilities argument '{unknown}'; {USAGE}"
                ));
            }
        }
    }
    Ok(live)
}

fn static_probe(capabilities: Capabilities) -> CapabilityProbeV1 {
    CapabilityProbeV1 {
        mode: CapabilityProbeModeV1::Static,
        status: CapabilityProbeStatusV1::StaticNoDevice,
        source: "compiled_backend_table".to_owned(),
        probed_at_unix_ms: None,
        requested_backend: Backend::Headless,
        selected_backend: None,
        device: None,
        color_target: static_target(
            capabilities.color_target_format,
            capabilities.render_sample_counts,
        ),
        depth_target: static_target("Depth32Float", capabilities.depth_sample_counts),
        readback: CapabilityConstraintProbeV1 {
            status: CapabilityConstraintStatusV1::NotProbed,
            detail: "no device or target was created".to_owned(),
        },
        presentation: CapabilityConstraintProbeV1 {
            status: CapabilityConstraintStatusV1::NotApplicable,
            detail: "static headless report has no presentation surface".to_owned(),
        },
        unavailable: None,
    }
}

fn static_target(format: &str, counts: [u32; 3]) -> CapabilityTargetProbeV1 {
    CapabilityTargetProbeV1 {
        format: format.to_owned(),
        source: "renderer_contract".to_owned(),
        measured: false,
        allowed_usages: None,
        sample_counts: counts.into_iter().filter(|count| *count != 0).collect(),
    }
}

fn unavailable_probe(probed_at_unix_ms: u64, error: &BuildError) -> CapabilityProbeV1 {
    CapabilityProbeV1 {
        mode: CapabilityProbeModeV1::LiveAdapter,
        status: CapabilityProbeStatusV1::Unavailable,
        source: "live_wgpu_adapter_request".to_owned(),
        probed_at_unix_ms: Some(probed_at_unix_ms),
        requested_backend: Backend::HeadlessGpu,
        selected_backend: None,
        device: None,
        color_target: unavailable_target("unknown"),
        depth_target: unavailable_target("Depth32Float"),
        readback: CapabilityConstraintProbeV1 {
            status: CapabilityConstraintStatusV1::Unavailable,
            detail: "readback cannot be probed without a GPU device".to_owned(),
        },
        presentation: CapabilityConstraintProbeV1 {
            status: CapabilityConstraintStatusV1::NotProbed,
            detail: "headless capability probing does not create a presentation surface".to_owned(),
        },
        unavailable: Some(CapabilityProbeUnavailableV1 {
            code: match error {
                BuildError::NoAdapter { .. } => "no_adapter",
                BuildError::RequestDevice { .. } => "request_device",
                _ => "gpu_initialization",
            }
            .to_owned(),
            message: error.to_string(),
        }),
    }
}

fn unavailable_target(format: &str) -> CapabilityTargetProbeV1 {
    CapabilityTargetProbeV1 {
        format: format.to_owned(),
        source: "unavailable".to_owned(),
        measured: false,
        allowed_usages: None,
        sample_counts: Vec::new(),
    }
}

fn probe_timestamp_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

const USAGE: &str = "usage: scena capabilities [--live] [--json]";
