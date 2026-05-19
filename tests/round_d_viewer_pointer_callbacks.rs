#![cfg(not(target_arch = "wasm32"))]

use std::cell::RefCell;
use std::rc::Rc;

use scena::{Hit, InteractiveGltfViewer, LookupError, PlatformSurface, interactive_gltf_viewer};

type PickEventLog = Rc<RefCell<Vec<Result<Option<Hit>, LookupError>>>>;

#[test]
fn viewer_click_and_hover_callbacks_receive_hit_and_no_hit_results() {
    let mut viewer = interactive_gltf_viewer(
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
        PlatformSurface::native_window(96, 64),
    )
    .build()
    .expect("interactive viewer builds");
    let (hit_x, hit_y) = first_hit_coordinate(&viewer);

    let click_events: PickEventLog = Rc::default();
    let hover_events: PickEventLog = Rc::default();

    viewer.on_click({
        let click_events = Rc::clone(&click_events);
        move |result| click_events.borrow_mut().push(result)
    });
    viewer.on_hover({
        let hover_events = Rc::clone(&hover_events);
        move |result| hover_events.borrow_mut().push(result)
    });

    let click_hit = viewer.click_at(hit_x, hit_y).expect("click picking runs");
    let hover_hit = viewer.hover_at(hit_x, hit_y).expect("hover picking runs");
    let click_miss = viewer
        .click_at(10_000.0, 10_000.0)
        .expect("click miss picking runs");
    let hover_miss = viewer
        .hover_at(10_000.0, 10_000.0)
        .expect("hover miss picking runs");

    assert!(click_hit.is_some(), "center click should hit the fixture");
    assert!(hover_hit.is_some(), "center hover should hit the fixture");
    assert!(click_miss.is_none(), "far click should miss the fixture");
    assert!(hover_miss.is_none(), "far hover should miss the fixture");

    let click_events = click_events.borrow();
    assert_eq!(click_events.len(), 2);
    assert!(matches!(click_events[0], Ok(Some(_))));
    assert_eq!(click_events[0], Ok(click_hit));
    assert_eq!(click_events[1], Ok(None));

    let hover_events = hover_events.borrow();
    assert_eq!(hover_events.len(), 2);
    assert!(matches!(hover_events[0], Ok(Some(_))));
    assert_eq!(hover_events[0], Ok(hover_hit));
    assert_eq!(hover_events[1], Ok(None));
}

fn first_hit_coordinate(viewer: &InteractiveGltfViewer) -> (f32, f32) {
    for y in (4..64).step_by(4) {
        for x in (4..96).step_by(4) {
            if viewer
                .pick_at(x as f32, y as f32)
                .expect("hit search pick runs")
                .is_some()
            {
                return (x as f32, y as f32);
            }
        }
    }
    panic!("fixture should expose at least one pickable pixel");
}
