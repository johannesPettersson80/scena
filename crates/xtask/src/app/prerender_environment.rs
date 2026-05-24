use crate::app::prelude::*;

pub(crate) fn run_prerender_environment(input: &str) -> Result<(), Vec<Finding>> {
    let input_path = PathBuf::from(input);
    let sidecar_path = PathBuf::from(format!("{input}.prefilter.bin"));
    let assets = scena::Assets::new();
    let handle = pollster::block_on(assets.load_environment(input)).map_err(|error| {
        vec![Finding::new(
            "DEMO-HDR-SIDECAR-CURRENT",
            format!("failed to load HDR environment {input}: {error:?}"),
        )]
    })?;
    let environment = assets.environment(handle).ok_or_else(|| {
        vec![Finding::new(
            "DEMO-HDR-SIDECAR-CURRENT",
            format!("environment handle for {input} was not retained"),
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
        "wrote {} from {} profile={} source_sha256={} bytes={}",
        sidecar_path.display(),
        input_path.display(),
        sidecar.profile().name(),
        sidecar.source_sha256_hex(),
        fs::metadata(&sidecar_path).map(|m| m.len()).unwrap_or(0)
    );
    Ok(())
}
