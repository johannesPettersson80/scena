//! Path traces a studio product shot and writes it as a PNG.
//!
//! ```text
//! cargo run --release --example path_traced_still -- spheres --samples 128 --out spheres.png
//! cargo run --release --example path_traced_still -- demo/samples/khronos/WaterBottle.glb
//! ```
//!
//! The subject stands on a seamless backdrop under three softboxes. Nothing
//! here is specific to path tracing except the last call: the same scene
//! renders with `Renderer::render` too.

use std::f32::consts::FRAC_PI_2;

use scena::{
    Aabb, Angle, AreaLight, AreaLightShape, Assets, Color, DepthOfFieldConfig, GeometryDesc,
    GeometryTopology, GeometryVertex, MaterialDesc, PathTraceOptions, PerspectiveCamera, Renderer,
    Scene, Transform, Vec3,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let option = |flag: &str| {
        arguments
            .iter()
            .position(|argument| argument == flag)
            .and_then(|at| arguments.get(at + 1))
    };
    let number = |flag: &str, default: u32| {
        option(flag)
            .and_then(|value| value.parse().ok())
            .unwrap_or(default)
    };
    let subject = arguments
        .first()
        .filter(|first| !first.starts_with("--"))
        .map_or("spheres", String::as_str);
    let (width, height) = (number("--width", 960), number("--height", 640));
    let output = option("--out").map_or("path_traced_still.png", String::as_str);

    let assets = Assets::new();
    let mut scene = Scene::new();
    let bounds = if subject == "spheres" {
        material_spheres(&assets, &mut scene)?
    } else {
        let model = pollster::block_on(assets.load_scene(subject))?;
        let import = scene.instantiate(&model)?;
        import
            .bounds_world(&scene)
            .ok_or("the model has no visible geometry")?
    };
    let center = (bounds.min + bounds.max) * 0.5;
    let radius = (bounds.max - bounds.min).length() * 0.5;

    // The camera looks slightly down at the subject from the front right,
    // from as close as keeps every corner of its bounds in frame.
    let view = Vec3::new(0.55, 0.32, 1.0).normalize();
    let right = Vec3::Y.cross(view).normalize();
    let up = view.cross(right);
    let field_of_view = Angle::from_degrees(26.0);
    let half_height = (field_of_view.radians() * 0.5).tan();
    let half_width = half_height * width as f32 / height as f32;
    let distance = (0..8)
        .map(|corner| {
            let pick =
                |bit: u32, low: f32, high: f32| if corner >> bit & 1 == 0 { low } else { high };
            let offset = Vec3::new(
                pick(0, bounds.min.x, bounds.max.x),
                pick(1, bounds.min.y, bounds.max.y),
                pick(2, bounds.min.z, bounds.max.z),
            ) - center;
            (offset.dot(right).abs() / half_width).max(offset.dot(up).abs() / half_height)
                + offset.dot(view)
        })
        .fold(0.0_f32, f32::max)
        * 1.12;
    let eye = center + view * distance;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera {
            vertical_fov: field_of_view,
            aspect: 0.0,
            near: distance * 0.05,
            far: distance * 40.0,
        },
        Transform::at(eye).looking_at(center, Vec3::Y),
    )?;
    scene.set_active_camera(camera)?;

    let toward_camera = Vec3::new(view.x, 0.0, view.z).normalize();
    let side = Vec3::Y.cross(toward_camera);
    backdrop(
        &assets,
        &mut scene,
        Vec3::new(center.x, bounds.min.y, center.z),
        toward_camera,
        radius,
    )?;

    // Key from the upper left, a broad fill from the right, a strip behind.
    // Each entry: offset in subject radii, size, irradiance at the subject
    // (an area light of flux F delivers F / (4 pi d^2)), colour temperature.
    let softboxes = [
        (
            side * -2.6 + toward_camera * 2.2 + Vec3::Y * 3.0,
            (3.2, 2.2),
            4.2,
            5200.0,
        ),
        (
            side * 3.4 + toward_camera * 2.6 + Vec3::Y * 1.2,
            (3.6, 3.0),
            0.6,
            7500.0,
        ),
        (
            side * 1.4 - toward_camera * 2.4 + Vec3::Y * 3.4,
            (2.6, 0.7),
            1.6,
            6500.0,
        ),
    ];
    for (offset, (box_width, box_height), irradiance, kelvin) in softboxes {
        let position = center + offset * radius;
        let flux = irradiance * 4.0 * std::f32::consts::PI * (offset.length() * radius).powi(2);
        // The range lets each softbox fade before it reaches the far
        // backdrop, which leaves the wall darker than the floor.
        let light = AreaLight::default()
            .with_color(Color::from_kelvin(kelvin))
            .with_luminous_flux_lumens(flux)
            .with_range(offset.length() * radius * 2.5)
            .with_shape(AreaLightShape::rect(
                box_width * radius,
                box_height * radius,
            ));
        scene
            .area_light(light)
            .transform(Transform::at(position).looking_at(center, Vec3::Y))
            .add()?;
    }

    let mut renderer = Renderer::headless(width, height)?;
    renderer.set_environment(assets.default_environment());
    renderer.set_environment_intensity(0.15);
    renderer.set_background_color(Color::from_linear_rgb(0.02, 0.02, 0.022));
    // A lens about 2.5 % of the subject's size wide: sharp subject, soft backdrop.
    let focal_length_mm = (384.0 * radius).clamp(8.0, 600.0);
    renderer.set_depth_of_field(Some(DepthOfFieldConfig::physical(
        distance,
        focal_length_mm,
        24.0,
        16.0,
        9,
        8,
    )));

    let options = PathTraceOptions::new()
        .with_samples_per_pixel(number("--samples", 128))
        .with_max_bounces(number("--bounces", 10))
        .with_denoise(!arguments.iter().any(|argument| argument == "--no-denoise"));
    let started = std::time::Instant::now();
    let image =
        renderer.path_trace_with_progress(&scene, &assets, camera, &options, &|done, total| {
            if done % (total / 10).max(1) == 0 {
                eprintln!("  {:3}%", done * 100 / total);
            }
        })?;
    image.write_png(output)?;
    let stats = image.stats();
    eprintln!(
        "{output}: {width}x{height}, {} samples per pixel, {} triangles, {} lights, {:.1} s",
        stats.samples_per_pixel,
        stats.triangles,
        stats.lights,
        started.elapsed().as_secs_f32()
    );
    Ok(())
}

/// Two rows of spheres that walk through the material model.
fn material_spheres(
    assets: &Assets,
    scene: &mut Scene,
) -> Result<Aabb, Box<dyn std::error::Error>> {
    let color = Color::from_srgb_u8;
    let surface = MaterialDesc::pbr_metallic_roughness;
    let materials = [
        surface(color(236, 236, 232), 0.0, 0.95),
        surface(color(190, 40, 34), 0.0, 0.6),
        surface(color(28, 92, 200), 0.0, 0.25),
        surface(color(24, 150, 96), 0.0, 0.08),
        surface(color(170, 16, 20), 0.0, 0.55)
            .with_clearcoat_factor(1.0)
            .with_clearcoat_roughness_factor(0.04),
        surface(color(245, 250, 255), 0.0, 0.0)
            .with_transmission_factor(1.0)
            .with_ior(1.5)
            .with_thickness_factor(0.6),
        surface(color(255, 196, 92), 1.0, 0.45),
        surface(color(255, 196, 92), 1.0, 0.12),
        surface(color(232, 234, 236), 1.0, 0.3),
        surface(color(244, 162, 137), 1.0, 0.2),
        surface(color(250, 250, 250), 1.0, 0.03),
        surface(color(40, 42, 48), 1.0, 0.6),
    ];
    let sphere = assets.create_geometry(GeometryDesc::sphere(0.5, 96, 48));
    for (index, material) in materials.into_iter().enumerate() {
        let position = Vec3::new(
            (index % 6) as f32 * 1.22 - 3.05,
            0.5,
            (index / 6) as f32 * 1.5 - 0.75,
        );
        scene
            .mesh(sphere, assets.create_material(material))
            .transform(Transform::at(position))
            .add()?;
    }
    Ok(Aabb {
        min: Vec3::new(-3.55, 0.0, -1.25),
        max: Vec3::new(3.55, 1.0, 1.25),
    })
}

/// A seamless backdrop: a floor that sweeps up into a wall behind the subject.
fn backdrop(
    assets: &Assets,
    scene: &mut Scene,
    foot: Vec3,
    toward_camera: Vec3,
    radius: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    const SWEEP_STEPS: usize = 24;
    let side = Vec3::Y.cross(toward_camera);
    let (half_width, depth, sweep) = (40.0 * radius, 3.2 * radius, 2.5 * radius);
    let sweep_start = foot - toward_camera * depth;
    // The profile from the front edge of the floor to the top of the wall,
    // with the normal that faces the subject at each point.
    let mut profile = vec![(foot + toward_camera * (40.0 * radius), Vec3::Y)];
    for step in 0..=SWEEP_STEPS {
        let angle = step as f32 / SWEEP_STEPS as f32 * FRAC_PI_2;
        let position = sweep_start - toward_camera * (sweep * angle.sin())
            + Vec3::Y * (sweep * (1.0 - angle.cos()));
        profile.push((
            position,
            Vec3::Y * angle.cos() + toward_camera * angle.sin(),
        ));
    }
    let (top, wall_normal) = profile[profile.len() - 1];
    profile.push((top + Vec3::Y * (40.0 * radius), wall_normal));

    let vertices = profile
        .iter()
        .flat_map(|(position, normal)| {
            [-half_width, half_width].map(|offset| GeometryVertex {
                position: *position + side * offset,
                normal: *normal,
            })
        })
        .collect();
    let indices = (0..profile.len() as u32 - 1)
        .flat_map(|strip| {
            let corner = strip * 2;
            [
                corner,
                corner + 1,
                corner + 2,
                corner + 2,
                corner + 1,
                corner + 3,
            ]
        })
        .collect();
    let sweep = GeometryDesc::try_new(GeometryTopology::Triangles, vertices, indices)
        .map_err(|error| format!("backdrop geometry: {error:?}"))?;
    let geometry = assets.create_geometry(sweep);
    let paper =
        MaterialDesc::pbr_metallic_roughness(Color::from_linear_rgb(0.5, 0.49, 0.48), 0.0, 0.9);
    scene.mesh(geometry, assets.create_material(paper)).add()?;
    Ok(())
}
