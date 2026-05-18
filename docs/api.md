# API overview

`scena` exposes a small set of public types that cover the normal 3D
application workflow: create assets, build a scene, prepare renderer resources,
and render frames.

The authoritative API reference is generated on docs.rs:

<https://docs.rs/scena/1.3.0/scena/>

Use this page as the conceptual map.

Additive public API changes in 1.2.0:

- `AssetLoadOptions`
- `Assets::load_scene_with_options`
- `Assets::load_scene_with_report_options`
- `DiagnosticCode::MaterialTextureMissingDecodedPixels`
- `RendererStats::material_textures_missing_decoded_pixels`

Additive public API changes in 1.3.0:

- `Scene::frame_bounds`
- `FramingOptions`
- `FramingOptions::azimuth_elevation`
- `FramingOptions::front`
- `FramingOptions::back`
- `FramingOptions::left`
- `FramingOptions::right`
- `FramingOptions::top`
- `FramingOptions::bottom`
- `FramingOptions::three_quarter_front_left`
- `FramingOptions::three_quarter_front_right`
- `FramingOptions::three_quarter_back_left`
- `FramingOptions::three_quarter_back_right`
- `FramingOutcome`
- `ScreenRect`
- `ProjectedPoint`
- `Scene::project_world_point`
- `Scene::bounds_for_transforms`
- `Scene::add_grid_floor`
- `GridFloorOptions`
- `GridFloorHandles`
- `Aabb::union`
- `OrbitControls::focus_on_framing`
- `OrbitControls::from_framing`
- `Scene::add_studio_lighting`
- `Renderer::set_auto_exposure`
- `AutoExposureConfig`
- `AutoExposureResult`

## Core types

| Type | Role |
|---|---|
| `Scene` | Owns graph state: nodes, transforms, cameras, lights, renderables, labels, imports, animations, picking targets, and dirty state. |
| `Assets` | Owns logical resources: geometry, materials, textures, environments, parsed glTF/GLB assets, cache identity, reload, and retain policy. |
| `Renderer` | Owns rendering state: backend resources, prepared scene data, surfaces, targets, stats, diagnostics, capability reports, and frame output. |
| `SceneImport` | Represents an instantiated imported asset with roots, names, paths, anchors, connectors, bounds, clips, and stale-import checks. |

The common pattern is:

```rust
let mut assets = scena::Assets::new();
let asset = assets.load_scene("model.glb")?;

let mut scene = scena::Scene::new();
let import = scene.instantiate(&asset)?;
let bounds = import.bounds_world(&scene).ok_or("model has no bounds")?;
scene.add_studio_lighting()?;
scene.add_grid_floor(&assets, scena::GridFloorOptions::new().under_bounds(bounds))?;

let camera = scene.add_perspective_camera(
    scene.root(),
    scena::PerspectiveCamera::default().with_aspect(1280.0 / 720.0),
    scena::Transform::default(),
)?;
let framing = scene.frame_bounds(
    camera,
    bounds,
    scena::FramingOptions::new()
        .three_quarter_front_right()
        .fill(0.72)
        .viewport(1280, 720),
)?;
let controls = scena::OrbitControls::from_framing(framing);

let mut renderer = scena::Renderer::headless(1280, 720)?;
renderer.set_auto_exposure(scena::AutoExposureConfig::default());
renderer.prepare_with_assets(&mut scene, &assets)?;
renderer.render(&scene, camera)?;
```

See the exact signatures on docs.rs and the runnable examples in `examples/`.

## Typed handles

`scena` uses typed handles instead of raw integers or string identifiers for
renderer-owned objects.

Examples include:

- `NodeKey`
- `CameraKey`
- `GeometryHandle`
- `MaterialHandle`
- `TextureHandle`
- `EnvironmentHandle`
- `AnimationMixerKey`
- `InstanceSetKey`
- `HitTarget`

Typed handles make wrong-kind usage visible at compile time. Missing or stale
handles return structured errors.

## Scene construction

`Scene` is the place for graph state:

- node hierarchy,
- transforms,
- cameras,
- lights,
- renderable instances,
- labels and helper geometry,
- imported asset instances,
- animation mixers,
- picking state,
- visibility and layers.

Scene builders return typed keys or handles. Hosts keep application-specific
state in their own model and map the visible portion into `Scene`.

## Asset ownership

`Assets` owns resource creation and loading:

- primitive geometry,
- materials,
- textures,
- environments,
- glTF/GLB scene assets,
- cache and reload state,
- external asset fetching.

The renderer does not fetch or parse assets during `render()`. Load and decode
assets before preparation.

## Renderer lifecycle

`Renderer` has an explicit lifecycle:

1. Build or mutate `Scene` and `Assets`.
2. Call `prepare()` or `prepare_with_assets()`.
3. Call `render()` or `render_active()`.
4. If scene, assets, surface, target, environment, or renderer settings change,
   call `prepare()` again.

This keeps fallible work visible to the host and makes frame rendering
predictable.

Common renderer calls:

- `Renderer::headless`
- `Renderer::headless_gpu`
- `Renderer::from_surface`
- `Renderer::prepare`
- `Renderer::prepare_with_assets`
- `Renderer::render`
- `Renderer::render_active`
- `Renderer::set_debug`
- `Renderer::set_debug_overlay`
- `Renderer::capability_report`
- `Renderer::gpu_adapter_report`

Common scene interaction calls:

- `Scene::pick_with_assets`
- `Scene::pick_and_select_with_assets`
- `Scene::connect_import_connectors`
- `Scene::frame_bounds`
- `Scene::project_world_point`
- `Scene::bounds_for_transforms`
- `Scene::add_grid_floor`
- `Scene::add_studio_lighting`
- `Scene::with_default_camera()`

Common public event and output types:

- `SurfaceEvent`
- `DebugOverlay`
- `RendererStats`
- `CapabilityReport`
- `GpuAdapterReport`
- `AdapterLimitsReport`
- `AssetEvictionStats`
- `AssetStoreId`

Common asset-store calls:

- `Assets::store_id()`
- `Assets::load_scene_with_options()`
- `Assets::load_scene_with_report_options()`
- `Assets::contains_geometry`
- `Assets::contains_material`
- `Assets::contains_texture`
- `Assets::contains_environment`
- `Assets::release_unreferenced`

Common import and connector contracts:

- `SceneImport`
- `AnchorKey`
- `ConnectorKey`
- `AnchorFrame`
- `ConnectorFrame`
- `ConnectorMetadata`
- `ConnectionAlignment`
- `ConnectionRoll`
- `ConnectionLineOverlay`
- `ConnectorRollPolicy`
- `ConnectorPolarity`

Common viewer helpers:

- `FramingOptions`
- `FramingOutcome`
- `GridFloorOptions`
- `GridFloorHandles`
- `InteractiveGltfViewer`
- `InteractiveGltfViewerBuilder`
- `interactive_gltf_viewer(path, surface)`
- `InteractiveGltfViewer::handle_surface_event`
- `Renderer::headless_default()`
- `Renderer::set_auto_exposure`

## Errors and diagnostics

Public failures use structured errors such as:

- `BuildError`
- `AssetError`
- `ImportError`
- `InstantiateError`
- `LookupError`
- `PrepareError`
- `RenderError`
- `AnimationError`
- `ConnectionError`
- `ColorParseError`

Most errors include a stable category plus contextual data. Use pattern matching
for application behavior and `.help()` or diagnostics output for user-facing
messages.

## Stats and capabilities

`Renderer` exposes runtime information for:

- backend capability reports,
- GPU adapter reports,
- renderer statistics,
- debug overlays,
- resource and frame counters.

Use capability reports when selecting optional effects or platform-specific
paths. Use stats for testing, diagnostics, and performance visibility.

## Where to go next

- [Getting started](getting-started.md)
- [Rendering](rendering.md)
- [Assets](assets.md)
- [Lifecycle](lifecycle.md)
- [Errors](errors.md)
- [Capabilities](capabilities.md)
