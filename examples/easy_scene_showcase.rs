//! Easy scene showcase render harness.
//!
//! Produces presentation-quality PNGs that demonstrate every Round A/B/C/D
//! named primitive. Hero shot uses scena's own connector parts assembled
//! through the mate API; preset comparison grids use spheres so the
//! material / light / background / exposure differences are visible.
//! Output lands under `target/easy-scene-showcase/` so a maintainer can inspect
//! renders before committing the keepers to `docs/assets/easy-scene-showcase/`.
//!
//! Run:
//!
//! ```bash
//! cargo run --example easy_scene_showcase --release
//! ```

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use scena::{
    AlphaMode, AnimationLoopMode, AntiAliasing, Assets, AutoExposureConfig, Background, Color,
    ConnectOptions, CursorPosition, DirectionalLight, EnvironmentPreset, FramingOptions,
    GeometryDesc, GridFloorOptions, InteractionStyle, MaterialDesc,
    OrderIndependentTransparencyConfig, PerspectiveCamera, PointLight, PostBloomConfig, Renderer,
    Scene, ScreenSpaceAmbientOcclusionConfig, Transform, Vec3, Viewport, headless_gltf_viewer,
};

const HERO_W: u32 = 1920;
const HERO_H: u32 = 1080;
const PANEL_W: u32 = 480;
const PANEL_H: u32 = 480;
const WIDE_W: u32 = 480;
const WIDE_H: u32 = 320;

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from("target/easy-scene-showcase");
    fs::create_dir_all(&out)?;
    eprintln!("rendering easy scene showcase into {}", out.display());

    render_hero_connector(&out)?;
    render_color_constants(&out)?;
    render_lens_presets(&out)?;
    render_light_presets(&out)?;
    render_material_presets(&out)?;
    render_background_presets(&out)?;
    render_auto_exposure_presets(&out)?;
    render_environment_presets(&out)?;
    render_capture_png_showcase(&out)?;
    render_material_variants(&out)?;
    render_picking_outline_hover(&out)?;
    render_connector_magnet_preview(&out)?;
    render_khronos_rigged_simple(&out)?;
    render_aa_on_off(&out)?;
    render_ssao_on_off(&out)?;
    render_bloom_on_off(&out)?;
    render_oit_order_invariance(&out)?;
    render_animated_orbit_damping(&out)?;
    render_animated_orbit_zoom(&out)?;
    render_animated_animation_playback(&out)?;
    render_animated_pointer_callbacks(&out)?;
    render_animated_hot_reload(&out)?;
    render_material_extension_clearcoat(&out)?;
    render_material_extension_sheen(&out)?;
    render_material_extension_anisotropy(&out)?;
    render_material_extension_iridescence(&out)?;
    render_material_extension_dispersion(&out)?;
    render_material_extension_transmission(&out)?;

    eprintln!("done — keepers go under docs/assets/easy-scene-showcase/");
    Ok(())
}

// ===========================================================================
// Hero — scena's connector assembly as a single static beauty render, using
// the shipped easy-scene ergonomics in one place (Background::DarkStudio, studio
// lighting, AutoExposureConfig::product_studio, EnvironmentPreset::Studio,
// `add_perspective_camera_default_for`).
// ===========================================================================

fn render_hero_connector(out: &Path) -> Result<(), Box<dyn Error>> {
    let assets = Assets::new();
    let drive = pollster::block_on(assets.load_scene("tests/assets/gltf/drive_unit.glb"))?;
    let load = pollster::block_on(assets.load_scene("tests/assets/gltf/load_unit.glb"))?;
    let environment =
        pollster::block_on(assets.load_environment_preset(EnvironmentPreset::Studio))?;

    let mut scene = Scene::new();
    let load_import = scene.instantiate(&load)?;
    let drive_import = scene.instantiate(&drive)?;
    scene.mate(&drive_import, "shaft", &load_import, "hub")?;

    let load_bounds = load_import.bounds_world(&scene);
    let drive_bounds = drive_import.bounds_world(&scene);
    let bounds = match (load_bounds, drive_bounds) {
        (Some(a), Some(b)) => a.union(b),
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => return Err("connector parts have no bounds".into()),
    };

    scene.add_studio_lighting()?;
    scene.add_grid_floor(&assets, GridFloorOptions::new().under_bounds(bounds))?;

    let camera = scene.add_perspective_camera_default_for(bounds, (HERO_W, HERO_H))?;
    scene.frame_bounds(
        camera,
        bounds,
        FramingOptions::new()
            .three_quarter_front_right()
            .fill(0.62)
            .margin_px(96.0)
            .viewport(HERO_W, HERO_H),
    )?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(HERO_W, HERO_H)?;
    renderer.set_environment(environment);
    renderer.set_background(Background::DarkStudio);
    renderer.set_auto_exposure(AutoExposureConfig::product_studio());
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    write_png(
        renderer.frame_rgba8(),
        HERO_W,
        HERO_H,
        &out.join("hero-connector-assembly.png"),
    )?;
    Ok(())
}

// ===========================================================================
// §2.1 Color named constants — labelled-sphere grid against DarkStudio.
// ===========================================================================

fn render_color_constants(out: &Path) -> Result<(), Box<dyn Error>> {
    let entries: &[Color] = &[
        Color::BLACK,
        Color::CHARCOAL,
        Color::DARK_GRAY,
        Color::GRAY,
        Color::LIGHT_GRAY,
        Color::STUDIO_BACKDROP,
        Color::WHITE,
        Color::WARM_WHITE,
        Color::COOL_WHITE,
        Color::RED,
        Color::ORANGE,
        Color::YELLOW,
        Color::GREEN,
        Color::CYAN,
        Color::BLUE,
        Color::MAGENTA,
    ];
    let columns = 4_u32;
    let rows = 4_u32;
    let panel_w = 360_u32;
    let panel_h = 360_u32;
    let composite_w = panel_w * columns;
    let composite_h = panel_h * rows;
    let mut rgba = vec![0_u8; (composite_w * composite_h * 4) as usize];
    for (index, color) in entries.iter().enumerate() {
        let tile = render_color_sphere(*color, panel_w, panel_h)?;
        let col = index as u32 % columns;
        let row = index as u32 / columns;
        composite_blit(
            &mut rgba,
            composite_w,
            &tile,
            panel_w,
            panel_h,
            col * panel_w,
            row * panel_h,
        );
    }
    write_png(
        &rgba,
        composite_w,
        composite_h,
        &out.join("named-color-constants.png"),
    )?;
    Ok(())
}

fn render_color_sphere(color: Color, width: u32, height: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let assets = Assets::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.5, 96, 64));
    let material = assets.create_material(MaterialDesc::plastic(color));

    let mut scene = Scene::new();
    scene.mesh(sphere, material).add()?;
    scene.add_studio_lighting()?;
    let environment =
        pollster::block_on(assets.load_environment_preset(EnvironmentPreset::Studio))?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::portrait(),
        Transform::at(Vec3::new(0.0, 0.0, 2.2)),
    )?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(width, height)?;
    renderer.set_environment(environment);
    renderer.set_background(Background::DarkStudio);
    renderer.set_auto_exposure(AutoExposureConfig::product_studio());
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    Ok(renderer.frame_rgba8().to_vec())
}

// ===========================================================================
// §2.2 PerspectiveCamera lens presets — geometric scene at each lens.
// ===========================================================================

fn render_lens_presets(out: &Path) -> Result<(), Box<dyn Error>> {
    let lenses: &[PerspectiveCamera] = &[
        PerspectiveCamera::wide_angle(),
        PerspectiveCamera::standard(),
        PerspectiveCamera::portrait(),
        PerspectiveCamera::telephoto(),
    ];
    let composite = compose_horizontal(lenses.len() as u32, PANEL_W, PANEL_H, |i| {
        render_subject_with_lens(lenses[i], PANEL_W, PANEL_H)
    })?;
    write_png(
        &composite,
        PANEL_W * lenses.len() as u32,
        PANEL_H,
        &out.join("lens-presets.png"),
    )?;
    Ok(())
}

fn render_subject_with_lens(
    lens: PerspectiveCamera,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let assets = Assets::new();
    let metal_sphere = assets.create_geometry(GeometryDesc::sphere(0.45, 96, 64));
    let plinth = assets.create_geometry(GeometryDesc::box_xyz(2.0, 0.06, 2.0));
    let cylinder = assets.create_geometry(GeometryDesc::cylinder(0.18, 0.6, 48));
    let metal = assets.create_material(MaterialDesc::metal(Color::LIGHT_GRAY));
    let plastic = assets.create_material(MaterialDesc::plastic(Color::ORANGE));
    let dark = assets.create_material(MaterialDesc::plastic(Color::CHARCOAL));
    let environment =
        pollster::block_on(assets.load_environment_preset(EnvironmentPreset::Studio))?;

    let mut scene = Scene::new();
    scene
        .mesh(metal_sphere, metal)
        .transform(Transform::at(Vec3::new(0.0, 0.5, 0.0)))
        .add()?;
    scene
        .mesh(cylinder, plastic)
        .transform(Transform::at(Vec3::new(-0.7, 0.35, 0.0)))
        .add()?;
    scene
        .mesh(cylinder, plastic)
        .transform(Transform::at(Vec3::new(0.7, 0.35, 0.0)))
        .add()?;
    scene
        .mesh(plinth, dark)
        .transform(Transform::at(Vec3::new(0.0, 0.0, 0.0)))
        .add()?;
    scene.add_studio_lighting()?;

    let camera = scene.add_perspective_camera(
        scene.root(),
        lens,
        Transform::at(Vec3::new(0.0, 1.4, 3.0)),
    )?;
    scene.look_at_point(camera, Vec3::new(0.0, 0.5, 0.0))?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(width, height)?;
    renderer.set_environment(environment);
    renderer.set_background(Background::DarkStudio);
    renderer.set_auto_exposure(AutoExposureConfig::product_studio());
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    Ok(renderer.frame_rgba8().to_vec())
}

// ===========================================================================
// §2.5 Light presets — same sphere under each light.
// ===========================================================================

fn render_light_presets(out: &Path) -> Result<(), Box<dyn Error>> {
    // 4×2 grid: top row 4 directional presets, bottom row 3 point presets
    // + `Scene::add_studio_lighting()` composite (which is the same key /
    // fill / rim rig at its conventional camera-relative angles).
    let panel_w = PANEL_W;
    let panel_h = PANEL_H;
    let tiles: Vec<Vec<u8>> = vec![
        render_with_directional(DirectionalLight::sun(), panel_w, panel_h)?,
        render_with_directional(DirectionalLight::key_light(), panel_w, panel_h)?,
        render_with_directional(DirectionalLight::fill_light(), panel_w, panel_h)?,
        render_with_directional(DirectionalLight::rim_light(), panel_w, panel_h)?,
        render_with_point(PointLight::softbox(), panel_w, panel_h)?,
        render_with_point(PointLight::bulb_warm(), panel_w, panel_h)?,
        render_with_point(PointLight::bulb_cool(), panel_w, panel_h)?,
        render_with_studio_combo(panel_w, panel_h)?,
    ];
    let columns = 4_u32;
    let rows = 2_u32;
    let composite_w = panel_w * columns;
    let composite_h = panel_h * rows;
    let mut rgba = dark_studio_canvas(composite_w, composite_h);
    for (i, tile) in tiles.iter().enumerate() {
        let col = (i as u32) % columns;
        let row = (i as u32) / columns;
        composite_blit(
            &mut rgba,
            composite_w,
            tile,
            panel_w,
            panel_h,
            col * panel_w,
            row * panel_h,
        );
    }
    write_png(
        &rgba,
        composite_w,
        composite_h,
        &out.join("light-presets.png"),
    )?;
    Ok(())
}

fn render_with_studio_combo(width: u32, height: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let assets = Assets::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.5, 96, 64));
    let material = assets.create_material(MaterialDesc::plastic(Color::LIGHT_GRAY));
    let mut scene = Scene::new();
    scene.mesh(sphere, material).add()?;
    scene.add_studio_lighting()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::portrait(),
        Transform::at(Vec3::new(0.0, 0.0, 2.0)),
    )?;
    scene.set_active_camera(camera)?;
    render_scene(scene, &assets, width, height)
}

fn render_with_directional(
    light: DirectionalLight,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let assets = Assets::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.5, 96, 64));
    let material = assets.create_material(MaterialDesc::plastic(Color::LIGHT_GRAY));

    let mut scene = Scene::new();
    scene.mesh(sphere, material).add()?;
    scene.directional_light(light).add()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::portrait(),
        Transform::at(Vec3::new(0.0, 0.0, 2.0)),
    )?;
    scene.set_active_camera(camera)?;
    render_scene(scene, &assets, width, height)
}

fn render_with_point(
    light: PointLight,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let assets = Assets::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.5, 96, 64));
    let material = assets.create_material(MaterialDesc::plastic(Color::LIGHT_GRAY));

    let mut scene = Scene::new();
    scene.mesh(sphere, material).add()?;
    // Moderate distance — close enough to give visible illumination, far
    // enough to avoid blowing out the highlight side of the sphere.
    scene
        .point_light(light)
        .transform(Transform::at(Vec3::new(1.6, 1.8, 1.6)))
        .add()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::portrait(),
        Transform::at(Vec3::new(0.0, 0.0, 2.0)),
    )?;
    scene.set_active_camera(camera)?;
    render_scene(scene, &assets, width, height)
}

// ===========================================================================
// §2.6 MaterialDesc presets — expanded PBR preset set against the Studio
// HDR so reflections / specular response are visible.
// ===========================================================================

fn render_material_presets(out: &Path) -> Result<(), Box<dyn Error>> {
    let materials: Vec<MaterialDesc> = vec![
        MaterialDesc::matte(Color::BLUE),
        MaterialDesc::plastic(Color::BLUE),
        MaterialDesc::metal(Color::LIGHT_GRAY),
        MaterialDesc::rough_metal(Color::LIGHT_GRAY),
        MaterialDesc::chrome(),
        MaterialDesc::brushed_steel(),
        MaterialDesc::clearcoat_plastic(Color::BLUE),
        MaterialDesc::satin(Color::MAGENTA),
        MaterialDesc::leather(Color::ORANGE),
        MaterialDesc::clear_glass(Color::CYAN),
        MaterialDesc::frosted_glass(Color::COOL_WHITE),
        MaterialDesc::rubber(),
    ];
    let columns = 4_u32;
    let rows = materials.len().div_ceil(columns as usize) as u32;
    let mut composite = dark_studio_canvas(PANEL_W * columns, PANEL_H * rows);
    for (i, material) in materials.into_iter().enumerate() {
        let tile = render_material_sphere(material, PANEL_W, PANEL_H)?;
        let col = i as u32 % columns;
        let row = i as u32 / columns;
        composite_blit(
            &mut composite,
            PANEL_W * columns,
            &tile,
            PANEL_W,
            PANEL_H,
            col * PANEL_W,
            row * PANEL_H,
        );
    }
    write_png(
        &composite,
        PANEL_W * columns,
        PANEL_H * rows,
        &out.join("material-presets.png"),
    )?;
    Ok(())
}

fn render_material_sphere(
    material: MaterialDesc,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let assets = Assets::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.38, 128, 96));
    let material = assets.create_material(material);
    let environment =
        pollster::block_on(assets.load_environment_preset(EnvironmentPreset::Studio))?;

    let mut scene = Scene::new();
    scene.mesh(sphere, material).add()?;
    // IBL only — let the studio HDR drive specular response on metallic
    // surfaces. Studio directional lights would otherwise dominate and make
    // metal look diffuse / plastic.
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::portrait(),
        Transform::at(Vec3::new(0.0, 0.0, 1.8)),
    )?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(width, height)?;
    renderer.set_environment(environment);
    renderer.set_background(Background::DarkStudio);
    renderer.set_exposure_ev(0.5);
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    Ok(renderer.frame_rgba8().to_vec())
}

// ===========================================================================
// §2.7 Background presets — orange sphere against each backdrop.
// ===========================================================================

fn render_background_presets(out: &Path) -> Result<(), Box<dyn Error>> {
    let schemes: &[Background] = &[
        Background::Studio,
        Background::DarkStudio,
        Background::NeutralGray,
        Background::White,
        Background::Black,
        Background::Sky,
        Background::Transparent,
        Background::Custom(Color::CHARCOAL),
    ];
    let columns = 4_u32;
    let rows = 2_u32;
    let mut rgba = vec![0_u8; (WIDE_W * columns * WIDE_H * rows * 4) as usize];
    for (i, bg) in schemes.iter().enumerate() {
        let tile = render_background_subject(*bg, WIDE_W, WIDE_H)?;
        let col = i as u32 % columns;
        let row = i as u32 / columns;
        composite_blit(
            &mut rgba,
            WIDE_W * columns,
            &tile,
            WIDE_W,
            WIDE_H,
            col * WIDE_W,
            row * WIDE_H,
        );
    }
    write_png(
        &rgba,
        WIDE_W * columns,
        WIDE_H * rows,
        &out.join("background-presets.png"),
    )?;
    Ok(())
}

fn render_background_subject(
    background: Background,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let assets = Assets::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.45, 96, 64));
    let material = assets.create_material(MaterialDesc::plastic(Color::ORANGE));

    let mut scene = Scene::new();
    scene.mesh(sphere, material).add()?;
    scene.add_studio_lighting()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::standard(),
        Transform::at(Vec3::new(0.0, 0.0, 1.8)),
    )?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(width, height)?;
    renderer.set_background(background);
    // Fixed EV keeps the subject sphere consistent across panels so only
    // the background changes — otherwise auto-exposure compensates for the
    // background brightness and makes the sphere look different in each tile.
    renderer.set_exposure_ev(0.5);
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    Ok(renderer.frame_rgba8().to_vec())
}

// ===========================================================================
// §2.9 AutoExposureConfig presets.
// ===========================================================================

fn render_auto_exposure_presets(out: &Path) -> Result<(), Box<dyn Error>> {
    let configs: Vec<AutoExposureConfig> = vec![
        AutoExposureConfig::product_studio(),
        AutoExposureConfig::indoor(),
        AutoExposureConfig::outdoor(),
        AutoExposureConfig::mixed(),
    ];
    let composite = compose_horizontal(configs.len() as u32, PANEL_W, PANEL_H, |i| {
        render_subject_with_exposure(configs[i], PANEL_W, PANEL_H)
    })?;
    write_png(
        &composite,
        PANEL_W * configs.len() as u32,
        PANEL_H,
        &out.join("auto-exposure-presets.png"),
    )?;
    Ok(())
}

fn render_subject_with_exposure(
    config: AutoExposureConfig,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let assets = Assets::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.5, 96, 64));
    let material = assets.create_material(MaterialDesc::metal(Color::LIGHT_GRAY));
    let environment =
        pollster::block_on(assets.load_environment_preset(EnvironmentPreset::Studio))?;

    let mut scene = Scene::new();
    scene.mesh(sphere, material).add()?;
    scene.add_studio_lighting()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::portrait(),
        Transform::at(Vec3::new(0.0, 0.0, 2.0)),
    )?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(width, height)?;
    renderer.set_environment(environment);
    renderer.set_background(Background::DarkStudio);
    renderer.set_auto_exposure(config);
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    Ok(renderer.frame_rgba8().to_vec())
}

// ===========================================================================
// §5 EnvironmentPreset — NeutralStudio vs Studio side-by-side.
// ===========================================================================

fn render_environment_presets(out: &Path) -> Result<(), Box<dyn Error>> {
    let presets = [EnvironmentPreset::NeutralStudio, EnvironmentPreset::Studio];
    let composite = compose_horizontal(presets.len() as u32, PANEL_W, PANEL_H, |i| {
        render_subject_with_environment(presets[i], PANEL_W, PANEL_H)
    })?;
    write_png(
        &composite,
        PANEL_W * presets.len() as u32,
        PANEL_H,
        &out.join("environment-presets.png"),
    )?;
    Ok(())
}

fn render_subject_with_environment(
    preset: EnvironmentPreset,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let assets = Assets::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.5, 96, 64));
    let material = assets.create_material(MaterialDesc::metal(Color::LIGHT_GRAY));
    let environment = pollster::block_on(assets.load_environment_preset(preset))?;

    let mut scene = Scene::new();
    scene.mesh(sphere, material).add()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::portrait(),
        Transform::at(Vec3::new(0.0, 0.0, 2.0)),
    )?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(width, height)?;
    renderer.set_environment(environment);
    renderer.set_background(Background::DarkStudio);
    renderer.set_auto_exposure(AutoExposureConfig::product_studio());
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    Ok(renderer.frame_rgba8().to_vec())
}

// ===========================================================================
// §4.6 Screenshot one-liner — the file dropped here was written via the
// new v1.4 PNG-encode path (RGBA8 -> png crate), demonstrating
// capture_png_bytes as the underlying behavior.
// ===========================================================================

fn render_capture_png_showcase(out: &Path) -> Result<(), Box<dyn Error>> {
    let assets = Assets::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.5, 96, 64));
    let material = assets.create_material(MaterialDesc::metal(Color::ORANGE));
    let environment =
        pollster::block_on(assets.load_environment_preset(EnvironmentPreset::Studio))?;

    let mut scene = Scene::new();
    scene.mesh(sphere, material).add()?;
    scene.add_studio_lighting()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::portrait(),
        Transform::at(Vec3::new(0.0, 0.0, 2.0)),
    )?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(960, 540)?;
    renderer.set_environment(environment);
    renderer.set_background(Background::DarkStudio);
    renderer.set_auto_exposure(AutoExposureConfig::product_studio());
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;

    write_png(
        renderer.frame_rgba8(),
        960,
        540,
        &out.join("capture-png.png"),
    )?;
    Ok(())
}

// ===========================================================================
// §5 KHR_materials_variants — same model under 3 variant selections.
// ===========================================================================

fn render_material_variants(out: &Path) -> Result<(), Box<dyn Error>> {
    let variants: &[(Option<&str>, &str)] = &[
        (None, "default"),
        (Some("midnight"), "midnight"),
        (Some("noon"), "noon"),
    ];
    let composite = compose_horizontal(variants.len() as u32, PANEL_W, PANEL_H, |i| {
        render_material_variant_tile(variants[i].0, PANEL_W, PANEL_H)
    })?;
    write_png(
        &composite,
        PANEL_W * variants.len() as u32,
        PANEL_H,
        &out.join("material-variants.png"),
    )?;
    Ok(())
}

fn render_material_variant_tile(
    variant: Option<&str>,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut viewer = pollster::block_on(
        headless_gltf_viewer("tests/assets/gltf/material_variants_scene.gltf")
            .size(width, height)
            .build(),
    )?;
    if let Some(name) = variant {
        viewer.set_active_material_variant(Some(name))?;
    }
    viewer.render_next_frame()?;
    Ok(viewer.snapshot_rgba8().to_vec())
}

// ===========================================================================
// §5 Picking + outline + hover — viewport-centre pick with both styles set.
// ===========================================================================

fn render_picking_outline_hover(out: &Path) -> Result<(), Box<dyn Error>> {
    let width = HERO_W / 2;
    let height = HERO_H / 2;
    let assets = Assets::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.4, 96, 64));
    let cube = assets.create_geometry(GeometryDesc::box_xyz(0.6, 0.6, 0.6));
    let plinth = assets.create_geometry(GeometryDesc::box_xyz(2.5, 0.05, 2.5));
    let blue = assets.create_material(MaterialDesc::plastic(Color::BLUE));
    let orange = assets.create_material(MaterialDesc::plastic(Color::ORANGE));
    let dark = assets.create_material(MaterialDesc::plastic(Color::CHARCOAL));
    let environment =
        pollster::block_on(assets.load_environment_preset(EnvironmentPreset::Studio))?;

    let mut scene = Scene::new();
    scene
        .mesh(sphere, blue)
        .transform(Transform::at(Vec3::new(-0.55, 0.4, 0.0)))
        .add()?;
    scene
        .mesh(cube, orange)
        .transform(Transform::at(Vec3::new(0.55, 0.3, 0.0)))
        .add()?;
    scene
        .mesh(plinth, dark)
        .transform(Transform::at(Vec3::new(0.0, 0.0, 0.0)))
        .add()?;
    scene.add_studio_lighting()?;

    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::standard(),
        Transform::at(Vec3::new(0.0, 1.0, 2.6)),
    )?;
    scene.look_at_point(camera, Vec3::new(0.0, 0.35, 0.0))?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(width, height)?;
    renderer.set_environment(environment);
    renderer.set_background(Background::DarkStudio);
    renderer.set_exposure_ev(0.5);
    renderer.set_hover_style(InteractionStyle::outline(Color::YELLOW, 3.0));
    renderer.set_selection_style(InteractionStyle::outline(Color::CYAN, 4.0));
    renderer.prepare_with_assets(&mut scene, &assets)?;

    // Select the orange cube on the right
    let viewport = Viewport::new(width, height, 1.0).ok_or("invalid viewport")?;
    scene.pick_and_select_with_assets(
        camera,
        CursorPosition::physical(width as f32 * 0.66, height as f32 * 0.45),
        viewport,
        &assets,
    )?;
    // Hover the blue sphere on the left
    scene.pick_and_hover_with_assets(
        camera,
        CursorPosition::physical(width as f32 * 0.34, height as f32 * 0.5),
        viewport,
        &assets,
    )?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;

    write_png(
        renderer.frame_rgba8(),
        width,
        height,
        &out.join("picking-outline-hover.png"),
    )?;
    Ok(())
}

// ===========================================================================
// §6 Connector magnet preview — out-of-range vs snap-ready
// ===========================================================================

fn render_connector_magnet_preview(out: &Path) -> Result<(), Box<dyn Error>> {
    let panel_w = PANEL_W * 2;
    let panel_h = PANEL_H;
    let out_of_range = render_connector_pair(0.95, panel_w, panel_h)?;
    let snap_ready = render_connector_pair(0.0, panel_w, panel_h)?;
    let composite_w = panel_w * 2;
    let composite_h = panel_h;
    let mut rgba = dark_studio_canvas(composite_w, composite_h);
    composite_blit(
        &mut rgba,
        composite_w,
        &out_of_range,
        panel_w,
        panel_h,
        0,
        0,
    );
    composite_blit(
        &mut rgba,
        composite_w,
        &snap_ready,
        panel_w,
        panel_h,
        panel_w,
        0,
    );
    write_png(
        &rgba,
        composite_w,
        composite_h,
        &out.join("connector-magnet-preview.png"),
    )?;
    Ok(())
}

fn render_connector_pair(detach: f32, width: u32, height: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let assets = Assets::new();
    let drive = pollster::block_on(assets.load_scene("tests/assets/gltf/drive_unit.glb"))?;
    let load = pollster::block_on(assets.load_scene("tests/assets/gltf/load_unit.glb"))?;
    let environment =
        pollster::block_on(assets.load_environment_preset(EnvironmentPreset::Studio))?;

    let mut scene = Scene::new();
    let load_import = scene.instantiate(&load)?;
    let drive_import = scene.instantiate(&drive)?;
    scene.connect_import_connectors(
        &drive_import,
        "shaft",
        &load_import,
        "hub",
        ConnectOptions::default().with_axial_gap(detach),
    )?;

    let load_bounds = load_import.bounds_world(&scene);
    let drive_bounds = drive_import.bounds_world(&scene);
    let bounds = match (load_bounds, drive_bounds) {
        (Some(a), Some(b)) => a.union(b),
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => return Err("connector parts have no bounds".into()),
    };
    scene.add_studio_lighting()?;
    scene.add_grid_floor(&assets, GridFloorOptions::new().under_bounds(bounds))?;
    let camera = scene.add_perspective_camera_default_for(bounds, (width, height))?;
    scene.frame_bounds(
        camera,
        bounds,
        FramingOptions::new()
            .three_quarter_front_right()
            .fill(0.6)
            .viewport(width, height),
    )?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(width, height)?;
    renderer.set_environment(environment);
    renderer.set_background(Background::DarkStudio);
    renderer.set_auto_exposure(AutoExposureConfig::product_studio());
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    Ok(renderer.frame_rgba8().to_vec())
}

// ===========================================================================
// §4.1 Khronos sample loader — RiggedSimple (not WaterBottle)
// ===========================================================================

fn render_khronos_rigged_simple(out: &Path) -> Result<(), Box<dyn Error>> {
    let assets = Assets::new();
    #[cfg(feature = "khronos-samples")]
    let model = pollster::block_on(assets.khronos().rigged_simple())?;
    #[cfg(not(feature = "khronos-samples"))]
    let model = pollster::block_on(
        assets.load_scene("tests/assets/gltf/khronos/RiggedSimple/RiggedSimple.gltf"),
    )?;
    let environment =
        pollster::block_on(assets.load_environment_preset(EnvironmentPreset::Studio))?;

    let mut scene = Scene::new();
    let import = scene.instantiate(&model)?;
    let bounds = import
        .bounds_world(&scene)
        .ok_or("rigged_simple has no bounds")?;
    scene.add_studio_lighting()?;
    scene.add_grid_floor(&assets, GridFloorOptions::new().under_bounds(bounds))?;
    let camera = scene.add_perspective_camera_default_for(bounds, (PANEL_W * 2, PANEL_H))?;
    scene.frame_bounds(
        camera,
        bounds,
        FramingOptions::new()
            .three_quarter_front_right()
            .fill(0.7)
            .viewport(PANEL_W * 2, PANEL_H),
    )?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(PANEL_W * 2, PANEL_H)?;
    renderer.set_environment(environment);
    renderer.set_background(Background::DarkStudio);
    renderer.set_auto_exposure(AutoExposureConfig::product_studio());
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    write_png(
        renderer.frame_rgba8(),
        PANEL_W * 2,
        PANEL_H,
        &out.join("khronos-rigged-simple.png"),
    )?;
    Ok(())
}

// ===========================================================================
// §3.1 ON/OFF pairs: AA, SSAO, bloom, OIT
// ===========================================================================

fn render_aa_on_off(out: &Path) -> Result<(), Box<dyn Error>> {
    let pair = compose_on_off_pair(
        |r| r.set_anti_aliasing(AntiAliasing::None),
        |r| r.set_anti_aliasing(AntiAliasing::Fxaa),
        scene_thin_edge,
    )?;
    write_png(
        &pair,
        PANEL_W * 2,
        PANEL_H,
        &out.join("renderer-aa-on-off.png"),
    )?;
    Ok(())
}

fn render_ssao_on_off(out: &Path) -> Result<(), Box<dyn Error>> {
    let pair = compose_on_off_pair(
        |r| r.clear_screen_space_ambient_occlusion(),
        |r| r.set_screen_space_ambient_occlusion(Some(ScreenSpaceAmbientOcclusionConfig::subtle())),
        scene_contact_corner,
    )?;
    write_png(
        &pair,
        PANEL_W * 2,
        PANEL_H,
        &out.join("renderer-ssao-on-off.png"),
    )?;
    Ok(())
}

fn render_bloom_on_off(out: &Path) -> Result<(), Box<dyn Error>> {
    let pair = compose_on_off_pair(
        |r| r.clear_bloom(),
        |r| r.set_bloom(Some(PostBloomConfig::subtle())),
        scene_emissive,
    )?;
    write_png(
        &pair,
        PANEL_W * 2,
        PANEL_H,
        &out.join("renderer-bloom-on-off.png"),
    )?;
    Ok(())
}

fn render_oit_order_invariance(out: &Path) -> Result<(), Box<dyn Error>> {
    let panel_w = PANEL_W;
    let panel_h = PANEL_H;
    let order_a = render_oit_scene(false, panel_w, panel_h)?;
    let order_b = render_oit_scene(true, panel_w, panel_h)?;
    let composite_w = panel_w * 2;
    let composite_h = panel_h;
    let mut rgba = dark_studio_canvas(composite_w, composite_h);
    composite_blit(&mut rgba, composite_w, &order_a, panel_w, panel_h, 0, 0);
    composite_blit(
        &mut rgba,
        composite_w,
        &order_b,
        panel_w,
        panel_h,
        panel_w,
        0,
    );
    write_png(
        &rgba,
        composite_w,
        composite_h,
        &out.join("renderer-oit-order-invariance.png"),
    )?;
    Ok(())
}

fn render_oit_scene(
    reverse_order: bool,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let assets = Assets::new();
    let plane = assets.create_geometry(GeometryDesc::box_xyz(0.6, 0.6, 0.02));
    let translucent_red = Color::from_linear_rgba(Color::RED.r, Color::RED.g, Color::RED.b, 0.45);
    let translucent_green =
        Color::from_linear_rgba(Color::GREEN.r, Color::GREEN.g, Color::GREEN.b, 0.45);
    let translucent_blue =
        Color::from_linear_rgba(Color::BLUE.r, Color::BLUE.g, Color::BLUE.b, 0.45);
    let red = assets
        .create_material(MaterialDesc::plastic(translucent_red).with_alpha_mode(AlphaMode::Blend));
    let green = assets.create_material(
        MaterialDesc::plastic(translucent_green).with_alpha_mode(AlphaMode::Blend),
    );
    let blue = assets
        .create_material(MaterialDesc::plastic(translucent_blue).with_alpha_mode(AlphaMode::Blend));

    let mut scene = Scene::new();
    let mats = if reverse_order {
        [blue, green, red]
    } else {
        [red, green, blue]
    };
    let offsets = if reverse_order {
        [
            Vec3::new(0.0, 0.0, 0.3),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, -0.3),
        ]
    } else {
        [
            Vec3::new(0.0, 0.0, -0.3),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.3),
        ]
    };
    for (mat, offset) in mats.into_iter().zip(offsets) {
        scene
            .mesh(plane, mat)
            .transform(Transform::at(offset))
            .add()?;
    }
    scene.add_studio_lighting()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::standard(),
        Transform::at(Vec3::new(0.0, 0.0, 2.0)),
    )?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(width, height)?;
    renderer.set_background(Background::DarkStudio);
    renderer.set_exposure_ev(0.5);
    renderer.set_order_independent_transparency(Some(
        OrderIndependentTransparencyConfig::weighted_blended(),
    ));
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    Ok(renderer.frame_rgba8().to_vec())
}

fn compose_on_off_pair<S, F1, F2>(
    off_apply: F1,
    on_apply: F2,
    mut scene_factory: S,
) -> Result<Vec<u8>, Box<dyn Error>>
where
    S: FnMut() -> Result<(Scene, Assets), Box<dyn Error>>,
    F1: Fn(&mut Renderer),
    F2: Fn(&mut Renderer),
{
    let off_frame = render_with_renderer_tweak(&mut scene_factory, &off_apply, PANEL_W, PANEL_H)?;
    let on_frame = render_with_renderer_tweak(&mut scene_factory, &on_apply, PANEL_W, PANEL_H)?;
    let composite_w = PANEL_W * 2;
    let composite_h = PANEL_H;
    let mut rgba = dark_studio_canvas(composite_w, composite_h);
    composite_blit(&mut rgba, composite_w, &off_frame, PANEL_W, PANEL_H, 0, 0);
    composite_blit(
        &mut rgba,
        composite_w,
        &on_frame,
        PANEL_W,
        PANEL_H,
        PANEL_W,
        0,
    );
    Ok(rgba)
}

fn render_with_renderer_tweak<S, F>(
    scene_factory: &mut S,
    apply: F,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Box<dyn Error>>
where
    S: FnMut() -> Result<(Scene, Assets), Box<dyn Error>>,
    F: Fn(&mut Renderer),
{
    let (mut scene, assets) = scene_factory()?;
    let mut renderer = Renderer::headless(width, height)?;
    renderer.set_background(Background::DarkStudio);
    renderer.set_exposure_ev(0.5);
    apply(&mut renderer);
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    Ok(renderer.frame_rgba8().to_vec())
}

fn scene_thin_edge() -> Result<(Scene, Assets), Box<dyn Error>> {
    // High-contrast diagonal edge to make FXAA's smoothing visible.
    let assets = Assets::new();
    let cube = assets.create_geometry(GeometryDesc::box_xyz(0.7, 0.7, 0.7));
    let material = assets.create_material(MaterialDesc::plastic(Color::WHITE));
    let mut scene = Scene::new();
    scene
        .mesh(cube, material)
        .transform(Transform::default().rotate_y_deg(30.0).rotate_x_deg(15.0))
        .add()?;
    scene.directional_light(DirectionalLight::sun()).add()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::portrait(),
        Transform::at(Vec3::new(0.0, 0.0, 2.0)),
    )?;
    scene.set_active_camera(camera)?;
    Ok((scene, assets))
}

fn scene_contact_corner() -> Result<(Scene, Assets), Box<dyn Error>> {
    // Sphere resting on a plinth — the contact ring is where SSAO should darken.
    let assets = Assets::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.4, 64, 48));
    let plinth = assets.create_geometry(GeometryDesc::box_xyz(2.0, 0.05, 2.0));
    let mat = assets.create_material(MaterialDesc::plastic(Color::LIGHT_GRAY));
    let mut scene = Scene::new();
    scene
        .mesh(sphere, mat)
        .transform(Transform::at(Vec3::new(0.0, 0.45, 0.0)))
        .add()?;
    scene
        .mesh(plinth, mat)
        .transform(Transform::at(Vec3::new(0.0, 0.0, 0.0)))
        .add()?;
    scene
        .directional_light(DirectionalLight::key_light())
        .add()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::standard(),
        Transform::at(Vec3::new(0.5, 1.0, 2.5)),
    )?;
    scene.look_at_point(camera, Vec3::new(0.0, 0.4, 0.0))?;
    scene.set_active_camera(camera)?;
    Ok((scene, assets))
}

fn scene_emissive() -> Result<(Scene, Assets), Box<dyn Error>> {
    // Bright emissive subject to give bloom something to bleed from.
    let assets = Assets::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.35, 64, 48));
    let mat = assets.create_material(
        MaterialDesc::plastic(Color::WHITE)
            .with_emissive(Color::WARM_WHITE)
            .with_emissive_strength(8.0),
    );
    let mut scene = Scene::new();
    scene.mesh(sphere, mat).add()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::portrait(),
        Transform::at(Vec3::new(0.0, 0.0, 1.8)),
    )?;
    scene.set_active_camera(camera)?;
    Ok((scene, assets))
}

// ===========================================================================
// §2.8 / §4.2 / §4.4 / §4.5 / §4.7 animated proofs — frame sequences saved
// as numbered PNGs, ready to be stitched into GIFs by the post-process step.
// ===========================================================================

fn render_animated_orbit_damping(out: &Path) -> Result<(), Box<dyn Error>> {
    let frames_dir = out.join("frames-orbit-damping");
    fs::create_dir_all(&frames_dir)?;
    let mut yaw_deg = 0.0_f32;
    let mut velocity = 90.0_f32;
    let damping = 0.85_f32;
    for frame in 0..16 {
        yaw_deg += velocity * (1.0 / 16.0);
        velocity *= damping;
        let rgba = render_orbit_frame(yaw_deg, PANEL_W, PANEL_H)?;
        write_png(
            &rgba,
            PANEL_W,
            PANEL_H,
            &frames_dir.join(format!("frame-{frame:02}.png")),
        )?;
    }
    Ok(())
}

fn render_orbit_frame(yaw_deg: f32, width: u32, height: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let assets = Assets::new();
    let cube = assets.create_geometry(GeometryDesc::box_xyz(0.7, 0.7, 0.7));
    let mat = assets.create_material(MaterialDesc::plastic(Color::ORANGE));
    let mut scene = Scene::new();
    scene
        .mesh(cube, mat)
        .transform(Transform::default().rotate_y_deg(yaw_deg))
        .add()?;
    scene.add_studio_lighting()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::standard(),
        Transform::at(Vec3::new(0.0, 0.0, 2.5)),
    )?;
    scene.set_active_camera(camera)?;
    let mut renderer = Renderer::headless(width, height)?;
    renderer.set_background(Background::DarkStudio);
    renderer.set_exposure_ev(0.5);
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    Ok(renderer.frame_rgba8().to_vec())
}

fn render_animated_orbit_zoom(out: &Path) -> Result<(), Box<dyn Error>> {
    let frames_dir = out.join("frames-orbit-zoom");
    fs::create_dir_all(&frames_dir)?;
    // Zoom-in then zoom-out, with clamps at the limits.
    let distances = [3.2_f32, 2.6, 2.0, 1.5, 1.5, 2.0, 2.6, 3.2];
    for (frame, distance) in distances.into_iter().enumerate() {
        let rgba = render_zoom_frame(distance, PANEL_W, PANEL_H)?;
        write_png(
            &rgba,
            PANEL_W,
            PANEL_H,
            &frames_dir.join(format!("frame-{frame:02}.png")),
        )?;
    }
    Ok(())
}

fn render_zoom_frame(distance: f32, width: u32, height: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let assets = Assets::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.4, 64, 48));
    let mat = assets.create_material(MaterialDesc::metal(Color::LIGHT_GRAY));
    let environment =
        pollster::block_on(assets.load_environment_preset(EnvironmentPreset::Studio))?;
    let mut scene = Scene::new();
    scene.mesh(sphere, mat).add()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::standard(),
        Transform::at(Vec3::new(0.0, 0.0, distance)),
    )?;
    scene.set_active_camera(camera)?;
    let mut renderer = Renderer::headless(width, height)?;
    renderer.set_environment(environment);
    renderer.set_background(Background::DarkStudio);
    renderer.set_exposure_ev(0.5);
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    Ok(renderer.frame_rgba8().to_vec())
}

fn render_animated_animation_playback(out: &Path) -> Result<(), Box<dyn Error>> {
    let frames_dir = out.join("frames-animation-playback");
    fs::create_dir_all(&frames_dir)?;
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/animated_connector_scene.gltf"))?;

    let mut scene = Scene::new();
    let import = scene.instantiate(&scene_asset)?;
    let animated = import.node("AnimatedMount")?;
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.35, 0.35, 0.35));
    let material = assets.create_material(MaterialDesc::plastic(Color::CYAN));
    scene.mesh(geometry, material).parent(animated).add()?;
    scene.add_studio_lighting()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::standard(),
        Transform::at(Vec3::new(0.45, 0.0, 3.0)),
    )?;
    scene.set_active_camera(camera)?;

    let mixer = scene.play_animation_by_name(&import, "MoveMount")?;
    let _ = scene.set_animation_loop_mode(mixer, AnimationLoopMode::Repeat);

    let mut renderer = Renderer::headless(PANEL_W, PANEL_H)?;
    renderer.set_background(Background::DarkStudio);
    renderer.set_exposure_ev(0.5);

    let delta = 0.12_f32;
    for frame in 0..10 {
        scene.update_animation(mixer, delta)?;
        renderer.prepare_with_assets(&mut scene, &assets)?;
        renderer.render_active(&scene)?;
        write_png(
            renderer.frame_rgba8(),
            PANEL_W,
            PANEL_H,
            &frames_dir.join(format!("frame-{frame:02}.png")),
        )?;
    }
    Ok(())
}

fn render_animated_pointer_callbacks(out: &Path) -> Result<(), Box<dyn Error>> {
    let frames_dir = out.join("frames-pointer-callbacks");
    fs::create_dir_all(&frames_dir)?;
    // Sequence: idle → hover sphere → hover cube → click cube → idle.
    let cursor_sequence = [
        None,
        Some((0.3_f32, 0.5_f32)),
        Some((0.7_f32, 0.5_f32)),
        Some((0.7_f32, 0.5_f32)),
        None,
    ];
    for (frame, cursor) in cursor_sequence.into_iter().enumerate() {
        let rgba = render_pointer_frame(cursor, frame == 3, PANEL_W, PANEL_H)?;
        write_png(
            &rgba,
            PANEL_W,
            PANEL_H,
            &frames_dir.join(format!("frame-{frame:02}.png")),
        )?;
    }
    Ok(())
}

fn render_pointer_frame(
    cursor_norm: Option<(f32, f32)>,
    click: bool,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let assets = Assets::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.4, 64, 48));
    let cube = assets.create_geometry(GeometryDesc::box_xyz(0.55, 0.55, 0.55));
    let blue = assets.create_material(MaterialDesc::plastic(Color::BLUE));
    let orange = assets.create_material(MaterialDesc::plastic(Color::ORANGE));

    let mut scene = Scene::new();
    scene
        .mesh(sphere, blue)
        .transform(Transform::at(Vec3::new(-0.55, 0.0, 0.0)))
        .add()?;
    scene
        .mesh(cube, orange)
        .transform(Transform::at(Vec3::new(0.55, 0.0, 0.0)))
        .add()?;
    scene.add_studio_lighting()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::standard(),
        Transform::at(Vec3::new(0.0, 0.0, 2.6)),
    )?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(width, height)?;
    renderer.set_background(Background::DarkStudio);
    renderer.set_exposure_ev(0.5);
    renderer.set_hover_style(InteractionStyle::outline(Color::YELLOW, 3.0));
    renderer.set_selection_style(InteractionStyle::outline(Color::CYAN, 4.0));
    renderer.prepare_with_assets(&mut scene, &assets)?;
    if let Some((nx, ny)) = cursor_norm {
        let viewport = Viewport::new(width, height, 1.0).ok_or("viewport")?;
        let pos = CursorPosition::physical(nx * width as f32, ny * height as f32);
        if click {
            scene.pick_and_select_with_assets(camera, pos, viewport, &assets)?;
        } else {
            scene.pick_and_hover_with_assets(camera, pos, viewport, &assets)?;
        }
        renderer.prepare_with_assets(&mut scene, &assets)?;
    }
    renderer.render_active(&scene)?;
    Ok(renderer.frame_rgba8().to_vec())
}

fn render_animated_hot_reload(out: &Path) -> Result<(), Box<dyn Error>> {
    let frames_dir = out.join("frames-hot-reload");
    fs::create_dir_all(&frames_dir)?;
    // Before / after for an asset edit: same sphere with two different colors.
    let colors = [Color::BLUE, Color::BLUE, Color::ORANGE, Color::ORANGE];
    for (frame, color) in colors.into_iter().enumerate() {
        let rgba = render_hot_reload_frame(color, PANEL_W, PANEL_H)?;
        write_png(
            &rgba,
            PANEL_W,
            PANEL_H,
            &frames_dir.join(format!("frame-{frame:02}.png")),
        )?;
    }
    Ok(())
}

fn render_hot_reload_frame(
    color: Color,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let assets = Assets::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.4, 64, 48));
    let mat = assets.create_material(MaterialDesc::plastic(color));
    let mut scene = Scene::new();
    scene.mesh(sphere, mat).add()?;
    scene.add_studio_lighting()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::portrait(),
        Transform::at(Vec3::new(0.0, 0.0, 2.0)),
    )?;
    scene.set_active_camera(camera)?;
    let mut renderer = Renderer::headless(width, height)?;
    renderer.set_background(Background::DarkStudio);
    renderer.set_exposure_ev(0.5);
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    Ok(renderer.frame_rgba8().to_vec())
}

// ===========================================================================
// §3.1 Material-extension before/after pairs — same sphere with the extension
// factor at zero vs at its v1.4 ergonomic value. Control on the left, with-
// extension on the right.
// ===========================================================================

fn render_material_extension_clearcoat(out: &Path) -> Result<(), Box<dyn Error>> {
    let pair = compose_material_extension_pair(
        MaterialDesc::plastic(Color::CHARCOAL),
        MaterialDesc::plastic(Color::CHARCOAL)
            .with_clearcoat_factor(1.0)
            .with_clearcoat_roughness_factor(0.06),
    )?;
    write_png(
        &pair,
        PANEL_W * 2,
        PANEL_H,
        &out.join("material-extension-clearcoat.png"),
    )?;
    Ok(())
}

fn render_material_extension_sheen(out: &Path) -> Result<(), Box<dyn Error>> {
    let pair = compose_material_extension_pair(
        MaterialDesc::matte(Color::DARK_GRAY),
        MaterialDesc::matte(Color::DARK_GRAY)
            .with_sheen_color_factor(Color::WARM_WHITE)
            .with_sheen_roughness_factor(0.35),
    )?;
    write_png(
        &pair,
        PANEL_W * 2,
        PANEL_H,
        &out.join("material-extension-sheen.png"),
    )?;
    Ok(())
}

fn render_material_extension_anisotropy(out: &Path) -> Result<(), Box<dyn Error>> {
    let pair = compose_material_extension_pair(
        MaterialDesc::metal(Color::LIGHT_GRAY),
        MaterialDesc::metal(Color::LIGHT_GRAY)
            .with_anisotropy_strength_factor(0.9)
            .with_anisotropy_rotation_radians(0.7),
    )?;
    write_png(
        &pair,
        PANEL_W * 2,
        PANEL_H,
        &out.join("material-extension-anisotropy.png"),
    )?;
    Ok(())
}

fn render_material_extension_iridescence(out: &Path) -> Result<(), Box<dyn Error>> {
    let pair = compose_material_extension_pair(
        MaterialDesc::plastic(Color::COOL_WHITE),
        MaterialDesc::plastic(Color::COOL_WHITE)
            .with_iridescence_factor(1.0)
            .with_iridescence_ior(1.34)
            .with_iridescence_thickness_range_nm(220.0, 720.0),
    )?;
    write_png(
        &pair,
        PANEL_W * 2,
        PANEL_H,
        &out.join("material-extension-iridescence.png"),
    )?;
    Ok(())
}

fn render_material_extension_dispersion(out: &Path) -> Result<(), Box<dyn Error>> {
    let pair = compose_material_extension_pair(
        MaterialDesc::plastic(Color::COOL_WHITE),
        MaterialDesc::plastic(Color::COOL_WHITE)
            .with_dispersion_factor(0.95)
            .with_ior(1.5),
    )?;
    write_png(
        &pair,
        PANEL_W * 2,
        PANEL_H,
        &out.join("material-extension-dispersion.png"),
    )?;
    Ok(())
}

fn render_material_extension_transmission(out: &Path) -> Result<(), Box<dyn Error>> {
    let pair = compose_material_extension_pair(
        MaterialDesc::plastic(Color::COOL_WHITE),
        MaterialDesc::plastic(Color::COOL_WHITE)
            .with_transmission_factor(0.9)
            .with_ior(1.5)
            .with_attenuation_distance(0.6)
            .with_attenuation_color(Color::CYAN),
    )?;
    write_png(
        &pair,
        PANEL_W * 2,
        PANEL_H,
        &out.join("material-extension-transmission.png"),
    )?;
    Ok(())
}

fn compose_material_extension_pair(
    control: MaterialDesc,
    with_extension: MaterialDesc,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let left = render_extension_sphere(control)?;
    let right = render_extension_sphere(with_extension)?;
    let composite_w = PANEL_W * 2;
    let composite_h = PANEL_H;
    let mut rgba = dark_studio_canvas(composite_w, composite_h);
    composite_blit(&mut rgba, composite_w, &left, PANEL_W, PANEL_H, 0, 0);
    composite_blit(&mut rgba, composite_w, &right, PANEL_W, PANEL_H, PANEL_W, 0);
    Ok(rgba)
}

fn render_extension_sphere(material: MaterialDesc) -> Result<Vec<u8>, Box<dyn Error>> {
    let assets = Assets::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.5, 128, 96));
    let material = assets.create_material(material);
    let environment =
        pollster::block_on(assets.load_environment_preset(EnvironmentPreset::Studio))?;
    let mut scene = Scene::new();
    scene.mesh(sphere, material).add()?;
    scene
        .directional_light(DirectionalLight::sun())
        .transform(Transform::default().rotate_x_deg(-30.0).rotate_y_deg(20.0))
        .add()?;
    scene
        .directional_light(DirectionalLight::fill_light())
        .transform(
            Transform::default()
                .rotate_x_deg(-10.0)
                .rotate_y_deg(-120.0),
        )
        .add()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::portrait(),
        Transform::at(Vec3::new(0.0, 0.0, 1.8)),
    )?;
    scene.set_active_camera(camera)?;
    // CPU/reference path. The full extension PBR lobes (clearcoat second-
    // specular, sheen Charlie, anisotropic GGX, iridescence thin-film,
    // dispersion channel-spread, transmission/volume) live in the WebGPU /
    // WebGL2 shaders and are exercised by the M6 browser proof on the CI
    // GPU lanes — not visible from a CPU/Pi render. The CPU path implements
    // factor parsing and texture sampling so this render still proves the
    // material descriptor round-trips, even if the lobe is not differentially
    // visible in this snapshot.
    let mut renderer = Renderer::headless(PANEL_W, PANEL_H)?;
    renderer.set_environment(environment);
    renderer.set_background(Background::DarkStudio);
    renderer.set_exposure_ev(0.5);
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    Ok(renderer.frame_rgba8().to_vec())
}

// ===========================================================================
// helpers
// ===========================================================================

fn render_scene(
    mut scene: Scene,
    assets: &Assets,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut renderer = Renderer::headless(width, height)?;
    renderer.set_background(Background::DarkStudio);
    renderer.set_auto_exposure(AutoExposureConfig::product_studio());
    renderer.prepare_with_assets(&mut scene, assets)?;
    renderer.render_active(&scene)?;
    Ok(renderer.frame_rgba8().to_vec())
}

fn compose_horizontal<F>(
    tile_count: u32,
    tile_width: u32,
    tile_height: u32,
    mut render: F,
) -> Result<Vec<u8>, Box<dyn Error>>
where
    F: FnMut(usize) -> Result<Vec<u8>, Box<dyn Error>>,
{
    let composite_w = tile_width * tile_count;
    let composite_h = tile_height;
    let mut rgba = vec![0_u8; (composite_w * composite_h * 4) as usize];
    for i in 0..tile_count as usize {
        let tile = render(i)?;
        composite_blit(
            &mut rgba,
            composite_w,
            &tile,
            tile_width,
            tile_height,
            (i as u32) * tile_width,
            0,
        );
    }
    Ok(rgba)
}

fn dark_studio_canvas(width: u32, height: u32) -> Vec<u8> {
    // DarkStudio in 8-bit sRGB: roughly #1a1d28.
    let mut rgba = vec![0_u8; (width * height * 4) as usize];
    for chunk in rgba.chunks_mut(4) {
        chunk[0] = 26;
        chunk[1] = 29;
        chunk[2] = 40;
        chunk[3] = 255;
    }
    rgba
}

fn composite_blit(
    dst: &mut [u8],
    dst_width: u32,
    src: &[u8],
    src_width: u32,
    src_height: u32,
    dst_x: u32,
    dst_y: u32,
) {
    for row in 0..src_height {
        let src_start = (row * src_width * 4) as usize;
        let dst_start = (((dst_y + row) * dst_width + dst_x) * 4) as usize;
        let bytes = (src_width * 4) as usize;
        dst[dst_start..dst_start + bytes].copy_from_slice(&src[src_start..src_start + bytes]);
    }
}

fn write_png(rgba: &[u8], width: u32, height: u32, path: &Path) -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(rgba)?;
    }
    fs::write(path, bytes)?;
    eprintln!("  wrote {}", path.display());
    Ok(())
}
