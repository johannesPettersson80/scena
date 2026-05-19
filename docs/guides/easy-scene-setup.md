# Easy scene setup

`scena` includes viewer helpers for the common "load a model, make it readable,
and render it" workflow. The helpers are composable: framing, lighting, floor
placement, auto exposure, orbit controls, and connector mating stay separate so
applications can replace any part.

## Minimal model viewer

```rust
use scena::{
    Assets, AutoExposureConfig, Background, FramingOptions, GridFloorOptions,
    PerspectiveCamera, Renderer, Scene, Transform,
};

let assets = Assets::new();
let model = assets.load_scene("machine.glb").await?;

let mut scene = Scene::new();
let import = scene.instantiate(&model)?;
let bounds = import.bounds_world(&scene).ok_or("model has no bounds")?;

scene.add_studio_lighting()?;
scene.add_grid_floor(&assets, GridFloorOptions::new().under_bounds(bounds))?;

let width = 1280;
let height = 720;
let camera = scene.add_perspective_camera(
    scene.root(),
    PerspectiveCamera::standard(),
    Transform::default(),
)?;

let framing = scene.frame_bounds(
    camera,
    bounds,
    FramingOptions::new()
        .three_quarter_front_right()
        .fill(0.72)
        .margin_px(48.0)
        .viewport(width, height),
)?;

let mut renderer = Renderer::headless(width, height)?;
renderer.set_background(Background::Studio);
renderer.set_auto_exposure(AutoExposureConfig::product_studio());
renderer.prepare_with_assets(&mut scene, &assets)?;
renderer.render(&scene, camera)?;
```

The sequence is still explicit: load assets, instantiate scene state, add
lights/floor/camera, frame the bounds, prepare, render. `frame_bounds()` mutates
camera state and marks the scene dirty, but it does not fetch assets, prepare
GPU resources, or render.

## Good defaults

Use `Scene::add_studio_lighting()` when the asset does not author lights. It is
a broad product-viewer setup: a shadowed key light plus softer fill and rim
lights. It is not a replacement for a deliberately authored lighting rig.
For custom rigs, start from the same named presets instead of raw lux or
candela values:

```rust
scene.directional_light(DirectionalLight::key_light()).add()?;
scene.directional_light(DirectionalLight::fill_light()).add()?;
scene.point_light(PointLight::bulb_warm()).add()?;
```

Use named exposure scenarios to adapt output brightness from the rendered
frame:

```rust
renderer.set_auto_exposure(AutoExposureConfig::product_studio());
renderer.set_auto_exposure(AutoExposureConfig::indoor());
renderer.set_auto_exposure(AutoExposureConfig::outdoor());
renderer.set_auto_exposure(AutoExposureConfig::mixed());
```

Auto exposure prevents globally too-dark or too-bright frames. It does not
change light direction, material albedo, roughness, dynamic range, or
composition.

Use named backgrounds for the first render instead of typing raw RGB values:

```rust
renderer.set_background(Background::Studio);
renderer.set_background(Background::DarkStudio);
renderer.set_background(Background::Custom(Color::from_hex("#f5f7fb")?));
```

Use `Scene::add_grid_floor()` for a matte floor at a known plane. The default
floor is dark, rough, non-metallic, and sized from object bounds so it grounds
the object without becoming the subject.
For simple authored geometry, prefer honest material presets over raw
metallic/roughness numbers:

```rust
let body = assets.create_material(MaterialDesc::plastic(Color::BLUE));
let shaft = assets.create_material(MaterialDesc::metal(Color::LIGHT_GRAY));
let foot = assets.create_material(MaterialDesc::rubber());
```

Use `Scene::frame_bounds()` instead of manually tuning camera distance. The
framing solver projects the AABB into the requested viewport and solves from
both axes, so portrait/mobile and wide objects stay centered and unclipped.

## Camera views

Pick a camera angle the way you would in Blender or any CAD tool: by name, or
by azimuth and elevation in degrees. No coordinate math.

```rust
FramingOptions::new().front();                 // camera at +Z
FramingOptions::new().back();                  // camera at -Z
FramingOptions::new().left();                  // camera at -X
FramingOptions::new().right();                 // camera at +X
FramingOptions::new().top();                   // camera at +Y (looking down)
FramingOptions::new().bottom();                // camera at -Y (looking up)
FramingOptions::new().isometric();             // classic 3D isometric
FramingOptions::new().three_quarter_front_right();
FramingOptions::new().three_quarter_front_left();
FramingOptions::new().three_quarter_back_right();
FramingOptions::new().three_quarter_back_left();
```

```rust
// Custom angle: 28 degrees to the left of front, 18 degrees above horizon.
FramingOptions::new().azimuth_elevation(-28.0, 18.0);
```

Azimuth and elevation are in degrees and use the conventions documented on
`FramingOptions::azimuth_elevation`.

## Orbit controls

After framing, pass the returned `FramingOutcome` to controls so the first user
drag orbits around the framed object:

```rust
let framing = scene.frame_bounds(camera, bounds, FramingOptions::new().viewport(width, height))?;
let controls = scena::OrbitControls::from_framing(framing).cinematic();
```

Clamp wheel and pinch zoom relative to that framed distance when a viewer
should stay near the inspected object:

```rust
let controls = scena::OrbitControls::from_framing(framing)
    .cinematic()
    .zoom_limits_bounds_relative(0.5, 4.0);
```

Use `presentation()` or `turntable(rpm)` when the host should advance the
camera between input events:

```rust
let mut controls = scena::OrbitControls::from_framing(framing).presentation();
let delta_seconds = 1.0 / 60.0;
if matches!(controls.advance(delta_seconds), scena::OrbitControlAction::Orbit) {
    controls.apply_to_scene(&mut scene, camera)?;
}
```

Host adapters can then apply the controls to the scene camera each frame.

## Viewer pointer callbacks

Interactive viewers can route host pointer coordinates through the same typed
picking path used by direct scene queries. The callback receives hit, miss, and
error results without bypassing selection or hover state updates.

```rust
use std::cell::RefCell;
use std::rc::Rc;

let selected = Rc::new(RefCell::new(None));
let hovered = Rc::new(RefCell::new(None));

viewer.on_click({
    let selected = Rc::clone(&selected);
    move |result| *selected.borrow_mut() = result.ok().flatten().map(|hit| hit.target)
});

viewer.on_hover({
    let hovered = Rc::clone(&hovered);
    move |result| *hovered.borrow_mut() = result.ok().flatten().map(|hit| hit.target)
});

viewer.hover_at(pointer_x, pointer_y)?;
viewer.click_at(pointer_x, pointer_y)?;
```

## Animation playback

Imported glTF clips can be started by name without manually creating and
starting a mixer:

```rust
let mixer = scene.play_animation_by_name(&import, "idle")?;
scene.update_animation(mixer, delta_seconds)?;
```

Keep the returned mixer key when the host needs to pause, seek, change speed,
or switch loop mode.

## Connector mating

Authored connectors let two imported assets find each other without application
code typing coordinates or raw matrices:

```rust
let drive_part = assets.load_scene("drive_unit.glb").await?;
let load_part = assets.load_scene("load_unit.glb").await?;

let drive = scene.instantiate(&drive_part)?;
let load = scene.instantiate(&load_part)?;
scene.mate(&drive, "shaft", &load, "hub")?;
```

The connector names come from glTF extras. The demo assets intentionally cover
different authoring conventions: `drive_unit` is Y-up in millimeters and
`load_unit` is Z-up in meters. The asset loader normalizes that metadata so
`scene.mate(&drive, "shaft", &load, "hub")?` is the operation the app writes.

For replay or animation, compute framing bounds across all relevant poses:

```rust
let replay_bounds = scene.bounds_for_transforms(drive_root, &[before, after], &assets)?;
let label = scene.project_world_point(camera, connector_world_point, width, height)?;
```

If an interpolation path arcs outside its endpoints, include sampled
intermediate transforms in `bounds_for_transforms()`.

## Troubleshooting

If the object is tiny, lower the floor padding first and check that you passed
only the model/replay bounds that should drive composition. Do not compensate
with a hard-coded camera distance.

If the object clips while still looking zoomed out, use `frame_bounds()` and
inspect `FramingOutcome::projected_rect`. Clipping from one side usually means
the camera target was not derived from projected bounds.

If the floor or grid appears behind the model like a wall, confirm the floor is
created with `GridFloorOptions::floor_y(0.0)` or the intended ground plane and
that all grid vertices stay on that plane.

If labels detach from geometry, derive them from connector or anchor world
positions and call `project_world_point()` after camera and object transforms
change. Static CSS percentages are not a valid 3D label contract.

If a render is bright and flat, separate the causes: auto exposure controls
global brightness, studio lighting controls scene shape, and materials control
albedo/metalness/roughness. Pure white albedo or flat roughness maps can still
look wrong under correct lighting.
