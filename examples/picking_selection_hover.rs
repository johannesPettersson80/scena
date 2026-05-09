use scena::{
    Assets, Color, CursorPosition, GeometryDesc, InteractionStyle, MaterialDesc, Renderer, Scene,
    Viewport,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.7, 0.45, 0.35));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(64, 160, 255)));

    let mut scene = Scene::new();
    scene.mesh(geometry, material).add()?;
    let camera = scene.add_default_camera()?;
    scene.frame_all_with_assets(camera, &assets)?;
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
    renderer.prepare_with_assets(&mut scene, &assets)?;

    let viewport = Viewport::new(128, 128, 1.0).expect("static viewport is valid");

    // Step 1 — hover-only: simulate a mouse-move event over the cube. This
    // updates only `interaction().hover()`; the primary selection stays
    // unchanged. In a real interactive loop this is the path the host
    // wires to pointer-move events.
    scene.pick_and_hover_with_assets(
        camera,
        CursorPosition::physical(64.0, 64.0),
        viewport,
        &assets,
    )?;
    println!(
        "after hover-only pass: hover={:?} primary_selection={:?}",
        scene.interaction().hover(),
        scene.interaction().primary_selection(),
    );

    // Step 2 — select on a primary-button click: simulate a mouse-down
    // event at the same position. This updates both `interaction().hover()`
    // and `interaction().primary_selection()` so the host can apply the
    // selection style on top of the existing hover style. In a real
    // interactive loop this is the path the host wires to pointer-down
    // events.
    scene.pick_and_select_with_assets(
        camera,
        CursorPosition::physical(64.0, 64.0),
        viewport,
        &assets,
    )?;
    println!(
        "after primary-select pass: hover={:?} primary_selection={:?}",
        scene.interaction().hover(),
        scene.interaction().primary_selection(),
    );

    // Step 3 — clear: simulate a pointer-leave event by hovering off-mesh.
    // The empty cursor position misses the cube and clears hover; the
    // primary selection persists until an explicit clear or new selection.
    scene.pick_and_hover_with_assets(
        camera,
        CursorPosition::physical(0.0, 0.0),
        viewport,
        &assets,
    )?;
    println!(
        "after pointer-leave pass: hover={:?} primary_selection={:?}",
        scene.interaction().hover(),
        scene.interaction().primary_selection(),
    );
    Ok(())
}
