#![cfg(not(target_arch = "wasm32"))]

//! Core metallic-roughness materials must keep their CPU output byte for byte.
//!
//! The CPU renderer shades every material through the layered surface model
//! (`src/render/prepare/pbr_contract/surface.rs`). A material that uses no
//! extension layer has to come out of it exactly as it did before that model
//! existed, because approved reference images depend on those bytes. Each scene
//! below was rendered by commit 4da02d2, the last commit before the layered
//! model, built into its own target directory; the SHA-256 of that frame is the
//! pin. A scene whose frame a later, deliberate CPU change moved records that
//! change and the frame it produced alongside the original pin.

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

use scena::{
    AntiAliasing, AreaLight, AreaLightShape, Assets, Background, CameraKey, Color,
    DirectionalLight, GeometryDesc, MaterialDesc, PerspectiveCamera, PointLight, Renderer, Scene,
    SpotLight, Transform, Vec3,
};

const SIZE: u32 = 96;
const ARTIFACT_DIR: &str = "target/gate-artifacts/cpu-core-material-bytes";

/// The pins are frames of one platform. Transcendental functions come from the
/// platform's C library, which may round their last bit differently elsewhere
/// and move a pixel by one step without any change to scena.
const CAPTURE_PLATFORM: bool = cfg!(all(
    target_os = "linux",
    target_arch = "x86_64",
    target_env = "gnu"
));

struct CoreScene {
    name: &'static str,
    /// SHA-256 of the RGBA8 frame commit 4da02d2 rendered.
    frame_sha256_at_4da02d2: &'static str,
    /// A later, deliberate CPU change that moved this frame, and the SHA-256
    /// of the frame since. The layered surface model is not such a change.
    repinned: Option<(&'static str, &'static str)>,
    render: fn() -> Vec<u8>,
}

impl CoreScene {
    fn expected(&self) -> (&'static str, &'static str) {
        self.repinned
            .unwrap_or(("commit 4da02d2", self.frame_sha256_at_4da02d2))
    }
}

fn core_scenes() -> [CoreScene; 4] {
    [
        CoreScene {
            name: "sphere-directional-environment",
            frame_sha256_at_4da02d2: "99bfa83b995768d453a2e402637ea4c16b1a7a09920cdcb1e36f45e6f095455c",
            repinned: None,
            render: render_sphere_directional_environment,
        },
        CoreScene {
            name: "metal-studio-fxaa",
            frame_sha256_at_4da02d2: "c1a746171b79d03f72280c145fea82b978e65d6557878d67460bb94c11579144",
            repinned: None,
            render: render_metal_studio_fxaa,
        },
        CoreScene {
            name: "point-spot-area",
            frame_sha256_at_4da02d2: "2a5aa1243fc6d5a42f89f1ba1d7c9a5746f4624985ce187dea19a372340f0820",
            // Its roughness 0.3 cube and 0.22 chrome sphere are glossy, so
            // their triangles are subdivided on screen and their highlights
            // follow the surface instead of the tessellation.
            repinned: Some((
                "glossy CPU subdivision",
                "a1b5e1d257b227d4041217ed86c880db803265ddb7554565da084c0636a38225",
            )),
            render: render_point_spot_area,
        },
        CoreScene {
            name: "emissive-double-sided",
            frame_sha256_at_4da02d2: "fc9e9ec0727e2c0ebd50b56194c3eeca63c042dd69da2e994103082be1f52661",
            repinned: None,
            render: render_emissive_double_sided,
        },
    ]
}

#[test]
fn core_materials_render_the_bytes_they_rendered_before_the_layered_surface_model() {
    if !CAPTURE_PLATFORM {
        eprintln!(
            "skipping core_materials_render_the_bytes_they_rendered_before_the_layered_surface_model: \
             the pinned frames were captured on x86_64-unknown-linux-gnu and other C libraries \
             may round transcendental functions differently; the platform-independent proof is \
             the lib test surface_without_layers_reproduces_the_pre_port_direct_light_bits"
        );
        return;
    }
    fs::create_dir_all(ARTIFACT_DIR).expect("artifact directory creates");
    let mut mismatches = Vec::new();
    for scene in core_scenes() {
        let frame = (scene.render)();
        assert!(
            frame.chunks_exact(4).any(|pixel| pixel[..3] != frame[..3]),
            "{} must render more than its background",
            scene.name
        );
        let actual = sha256_hex(&frame);
        let (pinned_by, expected) = scene.expected();
        if actual != expected {
            let path = format!("{ARTIFACT_DIR}/{}.png", scene.name);
            write_png(Path::new(&path), &frame);
            mismatches.push(format!(
                "{}: rendered {actual}, {pinned_by} rendered {expected} (frame written to {path})",
                scene.name
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "core metallic-roughness CPU output changed:\n{}",
        mismatches.join("\n")
    );
}

fn finish(
    mut renderer: Renderer,
    scene: &mut Scene,
    assets: &Assets,
    camera: CameraKey,
) -> Vec<u8> {
    renderer
        .prepare_with_assets(scene, assets)
        .expect("core scene prepares");
    renderer.render(scene, camera).expect("core scene renders");
    renderer.frame_rgba8().to_vec()
}

fn camera_at(scene: &mut Scene, position: Vec3) -> CameraKey {
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(position),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    camera
}

/// Direct light plus image-based light on a dielectric.
fn render_sphere_directional_environment() -> Vec<u8> {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let geometry = assets.create_geometry(GeometryDesc::sphere(0.5, 48, 32));
    let material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(170, 60, 44),
        0.0,
        0.4,
    ));
    scene
        .mesh(geometry, material)
        .add()
        .expect("sphere inserts");
    scene
        .directional_light(DirectionalLight::default().with_illuminance_lux(10_000.0))
        .transform(Transform::default().rotate_x_deg(-32.0).rotate_y_deg(28.0))
        .add()
        .expect("light inserts");
    let camera = camera_at(&mut scene, Vec3::new(0.0, 0.0, 1.6));
    let mut renderer = Renderer::headless(SIZE, SIZE).expect("CPU renderer builds");
    renderer.set_anti_aliasing(AntiAliasing::None);
    renderer.set_background(Background::DarkStudio);
    renderer.set_environment(assets.default_environment());
    finish(renderer, &mut scene, &assets, camera)
}

/// `add_studio_lighting()` defaults, a shadowed key light and CPU FXAA.
fn render_metal_studio_fxaa() -> Vec<u8> {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.45, 48, 32));
    let metal = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(212, 175, 96),
        1.0,
        0.42,
    ));
    scene
        .mesh(sphere, metal)
        .transform(Transform::at(Vec3::new(0.0, 0.05, 0.0)))
        .add()
        .expect("sphere inserts");
    let floor = assets.create_geometry(GeometryDesc::plane(3.0, 3.0));
    let matte = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(120, 124, 130),
        0.0,
        0.85,
    ));
    scene
        .mesh(floor, matte)
        .transform(Transform::at(Vec3::new(0.0, -0.4, 0.0)))
        .add()
        .expect("floor inserts");
    scene.add_studio_lighting().expect("studio lights insert");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.45, 1.7)).rotate_x_deg(-14.0),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    let mut renderer = Renderer::headless(SIZE, SIZE).expect("CPU renderer builds");
    renderer.set_anti_aliasing(AntiAliasing::Fxaa);
    finish(renderer, &mut scene, &assets, camera)
}

/// Point, spot and area lights: inverse-square falloff, the spot cone, the LTC
/// specular integral and the point-sampled area-light diffuse lobe.
fn render_point_spot_area() -> Vec<u8> {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let cube = assets.create_geometry(GeometryDesc::box_xyz(0.6, 0.6, 0.6));
    let satin = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(64, 132, 190),
        0.35,
        0.3,
    ));
    scene
        .mesh(cube, satin)
        .transform(
            Transform::at(Vec3::new(-0.35, 0.0, 0.0))
                .rotate_y_deg(32.0)
                .rotate_x_deg(18.0),
        )
        .add()
        .expect("cube inserts");
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.32, 40, 28));
    let chrome = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(214, 216, 220),
        1.0,
        0.22,
    ));
    scene
        .mesh(sphere, chrome)
        .transform(Transform::at(Vec3::new(0.45, 0.0, 0.1)))
        .add()
        .expect("sphere inserts");
    scene
        .point_light(
            PointLight::default()
                .with_color(Color::from_linear_rgb(1.0, 0.82, 0.6))
                .with_intensity_candela(6.0)
                .with_range(8.0),
        )
        .transform(Transform::at(Vec3::new(-1.2, 1.1, 1.4)))
        .add()
        .expect("point light inserts");
    scene
        .spot_light(SpotLight::default().with_intensity_candela(9.0))
        .transform(
            Transform::at(Vec3::new(0.9, 1.3, 1.6))
                .rotate_x_deg(-35.0)
                .rotate_y_deg(25.0),
        )
        .add()
        .expect("spot light inserts");
    scene
        .area_light(
            AreaLight::default()
                .with_luminous_flux_lumens(60.0)
                .with_shape(AreaLightShape::rect(1.2, 0.8)),
        )
        .transform(Transform::at(Vec3::new(0.0, 1.4, 0.6)).rotate_x_deg(-90.0))
        .add()
        .expect("area light inserts");
    let camera = camera_at(&mut scene, Vec3::new(0.0, 0.0, 2.0));
    let mut renderer = Renderer::headless(SIZE, SIZE).expect("CPU renderer builds");
    renderer.set_anti_aliasing(AntiAliasing::None);
    renderer.set_background(Background::DarkStudio);
    finish(renderer, &mut scene, &assets, camera)
}

/// An emissive, double-sided panel: the emissive term and the two-sided
/// shading both pass through the surface model's entry point.
fn render_emissive_double_sided() -> Vec<u8> {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let panel = assets.create_geometry(GeometryDesc::plane(1.1, 1.1));
    let glowing = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(90, 160, 110), 0.0, 0.6)
            .with_emissive(Color::from_linear_rgb(0.05, 0.02, 0.12))
            .with_emissive_strength(2.0)
            .with_double_sided(true),
    );
    scene
        .mesh(panel, glowing)
        .transform(Transform::default().rotate_x_deg(62.0).rotate_y_deg(-20.0))
        .add()
        .expect("panel inserts");
    scene
        .directional_light(DirectionalLight::default().with_illuminance_lux(6_000.0))
        .transform(Transform::default().rotate_x_deg(-50.0).rotate_y_deg(-15.0))
        .add()
        .expect("light inserts");
    let camera = camera_at(&mut scene, Vec3::new(0.0, 0.0, 1.8));
    let mut renderer = Renderer::headless(SIZE, SIZE).expect("CPU renderer builds");
    renderer.set_anti_aliasing(AntiAliasing::None);
    renderer.set_background(Background::DarkStudio);
    finish(renderer, &mut scene, &assets, camera)
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn write_png(path: &Path, rgba: &[u8]) {
    let file = File::create(path).expect("create artifact PNG");
    let mut encoder = png::Encoder::new(BufWriter::new(file), SIZE, SIZE);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .expect("write PNG header")
        .write_image_data(rgba)
        .expect("write PNG pixels");
}
