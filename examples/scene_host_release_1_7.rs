use scena::{
    AssetPath, Color, SceneHostAnimationLoopMode, SceneHostAnimationPlayOptions, SceneHostCore,
    SceneHostEasing, Transform, Vec3,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut host = SceneHostCore::headless(160, 120)?;
    let root = host.root_handle();

    host.set_anti_aliasing("fxaa")?;
    host.set_bloom_json(Some(
        r#"{"threshold_srgb": 190, "intensity": 0.35, "radius_px": 4}"#,
    ))?;
    host.set_ambient_occlusion_json(Some(
        r#"{"radius_px": 5, "intensity": 0.45, "depth_threshold": 0.025}"#,
    ))?;

    let instanced = pollster::block_on(host.instantiate_url_instanced_under(
        root,
        AssetPath::from("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
        3,
    ))?;
    for (index, handle) in instanced.iter().copied().enumerate() {
        host.set_transform(
            handle,
            Transform::at(Vec3::new(index as f32 * 0.45 - 0.45, -0.25, 0.0)),
        )?;
    }
    host.set_visible(instanced[0], false)?;
    host.set_node_tint(
        instanced[1],
        Some(Color::from_linear_rgba(0.1, 0.8, 0.25, 1.0)),
    )?;

    let animated = pollster::block_on(host.instantiate_url_under(
        root,
        AssetPath::from("tests/assets/gltf/animated_triangle_scene.glb"),
    ))?;
    let triangle = host.node_handle(animated, "AnimatedTriangle")?;
    let inventory = host.animation_inventory_json(animated)?;
    println!("animation inventory: {inventory}");
    let mixer = host.play_animation(
        animated,
        "MoveTriangle",
        SceneHostAnimationPlayOptions {
            loop_mode: SceneHostAnimationLoopMode::Repeat,
            speed: 1.0,
        },
    )?;
    host.advance(0.5)?;
    host.pause_animation(mixer)?;

    host.set_transform_eased(
        triangle,
        Transform::at(Vec3::new(0.0, 0.35, 0.0)),
        0.5,
        SceneHostEasing::EaseInOut,
    )?;
    host.set_node_tint_eased(
        triangle,
        Some(Color::from_linear_rgba(1.0, 0.15, 0.05, 1.0)),
        0.5,
        SceneHostEasing::Linear,
    )?;
    host.advance(0.25)?;

    host.frame_node_with_preset(triangle, "product_viewer_default")?;
    host.prepare()?;
    host.render()?;

    println!("stats: {}", host.stats_json());
    println!("inspection: {}", host.inspect_json()?);
    Ok(())
}
