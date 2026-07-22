# Migrating Common Three.js Workflows

Type: Guide.

`scena` does not copy Three.js method names. It keeps the workflow but changes the failure
mode: typed handles, explicit `prepare()`, structured diagnostics, and renderer-owned proof
instead of silent runtime guesses.

## Load A Model

Three.js loaders usually return objects that can be inserted directly into a scene. In
`scena`, `Assets` owns loading and `Scene` owns instantiated nodes:

```rust
let assets = Assets::new();
let asset = assets.load_scene("model.gltf").await?;
let mut scene = Scene::new();
let import = scene.instantiate(&asset)?;
```

Use `SceneImport` for roots, names, paths, anchors, connectors, clips, and diagnostics.
Do not keep string paths as long-lived object IDs.

## Compose Or Replace Transform Scale

Three.js distinguishes assignment (`object.scale.set(...)`) from composition
(`object.scale.multiplyScalar(...)`). Scena now makes the same distinction in
builder form:

```rust
let transform = Transform::IDENTITY
    .with_scale(Vec3::new(2.0, 3.0, 4.0)) // replace, like scale.set(...)
    .scale_by(0.5);                       // compose, like multiplyScalar(...)

let uniform = transform.with_uniform_scale(3.0); // explicit replacement
```

In v1.8.0, `Transform::scale_by` replaced the current scale. If downstream
code depended on replacement, change that call to `with_uniform_scale`. Calls
starting from `Transform::IDENTITY`, `Transform::default()`, or `Transform::at`
produce the same result and need no migration.

## Frame The Camera

In Three.js, camera fitting often starts from a `Box3`, a center point, and a
hand-maintained orbit target:

```js
const box = new THREE.Box3().setFromObject(model);
const center = box.getCenter(new THREE.Vector3());
const size = box.getSize(new THREE.Vector3());
const distance = Math.max(size.x / aspect, size.y) / (2 * Math.tan(fovY / 2));

camera.position.set(center.x + distance * 0.8, center.y + distance * 0.35, center.z + distance * 0.7);
controls.target.copy(center);
controls.update();
```

In `scena`, ask the scene to frame known bounds in the actual viewport and
seed the orbit controller from the framing result:

```rust
let bounds = import.bounds_world(&scene).ok_or("model has no bounds")?;
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
let controls = OrbitControls::from_framing(framing);
```

`frame_bounds()` solves from projected AABB corners on both axes, so wide
objects on portrait/mobile viewports do not get clipped or under-filled.

When the scene contains several imports, prefer the aggregate helper and pass
the capture or canvas dimensions rather than reusing a stale camera aspect:

```rust
let framing = scene.frame_all_with_assets_and_options(
    camera,
    &assets,
    FramingOptions::new()
        .three_quarter_front_right()
        .margin_px(48.0)
        .include_helpers(false)
        .viewport(width, height),
)?;
```

This is the `Box3::setFromObject` equivalent for all visible scene content.
Hidden nodes and tagged inspection helpers do not distort the default fit. Use
`frame_import_with_options` for only one imported subtree.

Three.js code sometimes conflates moving an `Object3D.position` with centering
the object's `Box3`. Scena names those operations separately:

```rust
scene.move_origin_to(node, point)?; // move the node origin
scene.center_visible_bounds_on(node, &assets, point)?; // center visible geometry
```

The ambiguous legacy `center_on` spelling is deprecated.

Three.js turntable code often stores the view in `Spherical::theta` and
`Spherical::phi`. In `scena`, the same intent is expressed as azimuth from the
front axis and elevation above the horizon:

```js
// Three.js
controls.target.copy(box.getCenter(new THREE.Vector3()));
const spherical = new THREE.Spherical().setFromVector3(
  camera.position.clone().sub(controls.target),
);
spherical.theta = THREE.MathUtils.degToRad(-28);
spherical.phi = THREE.MathUtils.degToRad(90 - 18);
camera.position.copy(
  new THREE.Vector3().setFromSpherical(spherical).add(controls.target),
);
```

```rust
// scena
scene.frame_bounds(
    camera,
    bounds,
    FramingOptions::new().azimuth_elevation(-28.0, 18.0),
)?;
```

For a complete load-light-floor-frame-render flow, see
[Easy scene setup](easy-scene-setup.md).

## Compose The Scene With Named Primitives

Three.js tends to take raw hex colors, numeric intensities, and
metalness/roughness floats inline. `scena` ships named primitives for the
same ideas so first-path code reads like the workflow it expresses.

```js
// Three.js
scene.background = new THREE.Color(0x1a1d28);

const key = new THREE.DirectionalLight(0xffffff, 3.0);
key.position.set(2.5, 4.0, 3.0);
scene.add(key);

const fill = new THREE.DirectionalLight(0xc8d8ff, 1.2);
fill.position.set(-3.5, 2.0, 1.5);
scene.add(fill);

const steel = new THREE.MeshStandardMaterial({
  color: 0xb0b6c2,
  metalness: 1.0,
  roughness: 0.32,
});

renderer.toneMappingExposure = 1.0;
```

```rust
// scena
renderer.set_background(Background::DarkStudio);

scene.directional_light(DirectionalLight::key_light()).add()?;
scene.directional_light(DirectionalLight::fill_light()).add()?;

let steel = assets.create_material(MaterialDesc::metal(Color::LIGHT_GRAY));

renderer.set_auto_exposure(AutoExposureConfig::product_studio());
```

The named primitives are paired with [`Color::WHITE`], [`Color::CHARCOAL`],
[`Color::from_kelvin`], [`Color::from_hex`], `MaterialDesc::matte`,
`MaterialDesc::plastic`, `MaterialDesc::metal`, `rough_metal`, `chrome`,
`brushed_steel`, `clearcoat_plastic`, `satin`, `leather`, `clear_glass`,
`frosted_glass`, `MaterialDesc::rubber`, `Background::Studio` /
`DarkStudio` / `NeutralGray` / `Sky` / `Transparent`, and
`AutoExposureConfig::product_studio` / `indoor` / `outdoor` / `mixed`.
Raw constructors (`Color::from_srgb_u8` and raw FOV) remain available as
escape hatches; the named primitives are the first-path defaults.

## Camera Controls

Three.js `OrbitControls` is configured by toggling several independent
properties. `scena` [`OrbitControls`] starts from the framing result and keeps
host-ticked presentation modes explicit:

```rust
let controls = OrbitControls::from_framing(framing)
    .zoom_limits_bounds_relative(0.5, 4.0);

let presentation = OrbitControls::from_framing(framing).presentation();
let turntable = OrbitControls::from_framing(framing).turntable(6.0);
let direct = OrbitControls::from_framing(framing);
```

[`Color::WHITE`]: ../api.md
[`Color::CHARCOAL`]: ../api.md
[`Color::from_kelvin`]: ../api.md
[`Color::from_hex`]: ../api.md
[`OrbitControls`]: ../api.md

## Connect Objects

Prefer named anchors or connectors over copied matrices:

```rust
scene.connect_import_connectors(
    &source_import,
    "mount",
    &target_import,
    "socket",
    ConnectOptions::default().with_alignment(ConnectionAlignment::ForwardToBack),
)?;
```

This lets `scena` validate units, axes, handedness, roll policy, and stale connector names
before the source node is moved.

## Render

Rendering is a two-step lifecycle:

```rust
renderer.prepare_with_assets(&mut scene, &assets)?;
renderer.render_active(&scene)?;
```

`render()` must not hide asset fetches, first-use GPU uploads, shader compilation, or
browser initialization. Structural scene, asset, surface, or environment changes require
another `prepare()`.

For CLI migration, do not split a multi-import recipe into a first-asset load
plus manual follow-up imports. Commands accepting `<asset-or-recipe>` identify
recipes explicitly and run the complete document through `RecipeBuildPolicy`
and SceneHost. Raw `.gltf`/`.glb` inputs still use the direct asset path. A
rejected recipe import returns a nonzero `scena.recipe_build_result.v1` rather
than a successful partial render.

## Pick And Select

Use camera-ray picking helpers instead of projecting coordinates manually:

```rust
let viewport = Viewport::new(width, height, device_pixel_ratio)?;
let cursor = CursorPosition::physical(x, y);
scene.pick_and_select_with_assets(camera, cursor, viewport, &assets)?;
```

Picking uses camera rays and world-space triangles in the current rendered pose: morph,
then skin, then node/instance transforms. Use `pick_with_assets` for the normal
`scene.mesh(...)`/glTF workflow, or `pick_and_select_with_assets`/`pick_and_hover_with_assets`
when UI state should update in one call. Distances are world-space camera-ray distances;
normals follow transformed winding, including reversal under negative scale. Collapsed
singular-scale triangles are not hittable. Plain `pick` exists for legacy primitive
renderables. A miss should be treated as a diagnostic problem only after camera framing,
layer masks, visibility, and the current deformation pose are checked.

## Materials And Current Limits

Three.js has mature GPU material and postprocessing stacks. This checkout has
subtle post-bloom, directional-shadow receiver proof on GPU-device
WebGPU/WebGL2/native lanes, plus headless CPU screen-space ambient occlusion
and weighted blended OIT baselines. It still reports PBR, GPU/browser
SSAO/OIT, WebGL2 material parity, and CPU/reference directional shadows as
degraded or disabled until backend visual proof lands. Use capability reports
and diagnostics as the source of truth for what a backend can show today.
