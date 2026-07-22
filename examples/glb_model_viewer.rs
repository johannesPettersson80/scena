fn main() -> Result<(), Box<dyn std::error::Error>> {
    let first = pollster::block_on(scena::first_render_gltf_headless(
        "tests/assets/gltf/cad_terminal_block.gltf",
        320,
        240,
    ))?;
    println!(
        "glb_model_viewer roots={} diagnostics={}",
        first.import().roots().len(),
        first.diagnostics().len()
    );
    Ok(())
}
