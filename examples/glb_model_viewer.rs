fn main() -> Result<(), Box<dyn std::error::Error>> {
    let first = pollster::block_on(scena::first_render_gltf_headless(
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
        320,
        240,
    ))?;
    println!("glb_model_viewer roots={}", first.import().roots().len());
    Ok(())
}
