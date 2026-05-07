use scena::{
    Color, CursorPosition, HitTarget, InteractionStyle, PerspectiveCamera, Primitive, Renderer,
    Scene, Transform, Viewport,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = Scene::new();
    let node = scene.add_renderable(
        scene.root(),
        vec![Primitive::unlit_triangle()],
        Transform::default(),
    )?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::default(),
        Transform::default(),
    )?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(128, 128)?;
    renderer.set_hover_style(InteractionStyle::outline(
        Color::from_srgb_u8(255, 210, 64),
        2.0,
    ));
    renderer.set_selection_style(InteractionStyle::outline(
        Color::from_srgb_u8(64, 160, 255),
        3.0,
    ));
    renderer.prepare(&mut scene)?;

    let viewport = Viewport::new(128, 128, 1.0).expect("valid viewport");
    if let Some(hit) = scene.pick(camera, CursorPosition::physical(64.0, 64.0), viewport)? {
        scene.interaction_mut().set_hover(Some(hit.target()));
        scene
            .interaction_mut()
            .set_primary_selection(Some(HitTarget::Node(node)));
    }
    println!("hover={:?}", scene.interaction().hover());
    Ok(())
}
