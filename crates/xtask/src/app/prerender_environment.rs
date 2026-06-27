use crate::app::prelude::*;

pub(crate) fn run_prerender_environment(
    input: &str,
    resolution: Option<u32>,
) -> Result<(), Vec<Finding>> {
    let input_path = PathBuf::from(input);
    let sidecar_path = PathBuf::from(format!("{input}.prefilter.bin"));
    let source_bytes = fs::read(input).map_err(|error| {
        vec![Finding::new(
            "DEMO-HDR-SIDECAR-CURRENT",
            format!("failed to read HDR environment {input}: {error}"),
        )]
    })?;
    let cubemap_resolution =
        resolution.unwrap_or(scena::DEFAULT_ENVIRONMENT_CUBEMAP_FACE_RESOLUTION);
    let environment = scena::EnvironmentDesc::from_equirectangular_hdr_bytes(input, &source_bytes)
        .map(|environment| environment.with_cubemap_resolution(cubemap_resolution))
        .map_err(|error| {
            vec![Finding::new(
                "DEMO-HDR-SIDECAR-CURRENT",
                format!("failed to decode HDR environment {input}: {error:?}"),
            )]
        })?;
    let sidecar = scena::render::precompute_environment_sidecar(
        &environment,
        scena::EnvironmentSidecarProfile::InteractiveWebGl2,
    )
    .map_err(|error| {
        vec![Finding::new(
            "DEMO-HDR-SIDECAR-CURRENT",
            format!("failed to precompute HDR sidecar for {input}: {error:?}"),
        )]
    })?;
    fs::write(&sidecar_path, sidecar.to_bytes()).map_err(|error| {
        vec![Finding::new(
            "DEMO-HDR-SIDECAR-CURRENT",
            format!(
                "failed to write HDR sidecar {}: {error}",
                sidecar_path.display()
            ),
        )]
    })?;
    println!(
        "wrote {} from {} resolution={} profile={} source_sha256={} bytes={}",
        sidecar_path.display(),
        input_path.display(),
        cubemap_resolution,
        sidecar.profile().name(),
        sidecar.source_sha256_hex(),
        fs::metadata(&sidecar_path).map(|m| m.len()).unwrap_or(0)
    );
    Ok(())
}
