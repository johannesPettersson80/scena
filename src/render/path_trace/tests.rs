//! End-to-end checks of the path tracer against scenes with known answers.
//!
//! Images are tiny and sample counts modest so the suite stays quick in an
//! unoptimized build; every expectation is a property with a wide margin
//! around the noise that leaves.

use crate::{
    AlphaMode, Angle, AreaLight, AreaLightShape, Assets, Background, CameraKey, Color,
    DirectionalLight, EnvironmentDesc, GeometryDesc, GeometryTopology, GeometryVertex,
    MaterialDesc, PathTraceError, PathTraceOptions, PathTracedImage, PerspectiveCamera, PointLight,
    Renderer, Scene, SpotLight, Transform, Vec3,
};

const SIZE: u32 = 24;
/// Height of the camera that looks straight down at the floor.
const EYE_HEIGHT: f32 = 4.0;
const FOV_DEGREES: f32 = 40.0;

struct Stage {
    assets: Assets,
    scene: Scene,
    camera: CameraKey,
}

impl Stage {
    fn new(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let mut scene = Scene::new();
        let camera = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera {
                    vertical_fov: Angle::from_degrees(FOV_DEGREES),
                    aspect: 0.0,
                    near: 0.1,
                    far: 100.0,
                },
                Transform::at(eye).looking_at(target, up),
            )
            .expect("camera inserts");
        scene
            .set_active_camera(camera)
            .expect("camera becomes active");
        Self {
            assets: Assets::new(),
            scene,
            camera,
        }
    }

    /// A camera above the origin looking straight down, with world `+x` to
    /// the right of the image and `+z` towards its bottom.
    fn top_down() -> Self {
        Self::new(Vec3::new(0.0, EYE_HEIGHT, 0.0), Vec3::ZERO, Vec3::NEG_Z)
    }

    fn mesh(
        &mut self,
        geometry: GeometryDesc,
        material: MaterialDesc,
        transform: Transform,
    ) -> crate::NodeKey {
        let geometry = self.assets.create_geometry(geometry);
        let material = self.assets.create_material(material);
        self.scene
            .mesh(geometry, material)
            .transform(transform)
            .add()
            .expect("mesh inserts")
    }

    fn floor(&mut self, size: f32) {
        self.mesh(
            GeometryDesc::plane(size, size),
            matte(0.8),
            Transform::IDENTITY,
        );
    }

    /// A floor of many small quads. The CPU rasterizer lights vertices, so
    /// it needs them close together to resolve a nearby light.
    fn fine_floor(&mut self, size: f32, cells: u32) {
        let side = cells + 1;
        let vertices = (0..side * side)
            .map(|index| GeometryVertex {
                position: Vec3::new(
                    ((index % side) as f32 / cells as f32 - 0.5) * size,
                    0.0,
                    ((index / side) as f32 / cells as f32 - 0.5) * size,
                ),
                normal: Vec3::Y,
            })
            .collect();
        let indices = (0..cells * cells)
            .flat_map(|cell| {
                let corner = cell / cells * side + cell % cells;
                [
                    corner,
                    corner + side,
                    corner + 1,
                    corner + 1,
                    corner + side,
                    corner + side + 1,
                ]
            })
            .collect();
        let grid = GeometryDesc::try_new(GeometryTopology::Triangles, vertices, indices)
            .expect("grid builds");
        self.mesh(grid, matte(0.8), Transform::IDENTITY);
    }

    fn trace(&self, renderer: &Renderer, options: PathTraceOptions) -> PathTracedImage {
        renderer
            .path_trace(&self.scene, &self.assets, self.camera, &options)
            .expect("the scene path traces")
    }
}

fn matte(albedo: f32) -> MaterialDesc {
    MaterialDesc::pbr_metallic_roughness(Color::from_linear_rgb(albedo, albedo, albedo), 0.0, 1.0)
}

fn renderer() -> Renderer {
    Renderer::headless(SIZE, SIZE).expect("headless renderer builds")
}

fn exact(samples: u32, bounces: u32) -> PathTraceOptions {
    PathTraceOptions::new()
        .with_samples_per_pixel(samples)
        .with_max_bounces(bounces)
        .with_denoise(false)
        .with_radiance_clamp(0.0)
}

/// Pointing a light or camera down from `position`.
fn facing_down(position: Vec3) -> Transform {
    Transform::at(position).looking_at(position - Vec3::Y, Vec3::Z)
}

/// Pixel of the top-down camera that shows the floor point `(x, 0, z)`.
fn floor_pixel(x: f32, z: f32) -> (u32, u32) {
    let half = EYE_HEIGHT * (FOV_DEGREES.to_radians() * 0.5).tan();
    let to_pixel = |world: f32| ((world / half * 0.5 + 0.5) * SIZE as f32) as u32;
    (to_pixel(x), to_pixel(z))
}

/// Mean linear radiance and coverage over a square of pixels.
fn radiance(image: &PathTracedImage, (x, y): (u32, u32), radius: u32) -> [f32; 4] {
    let mut sum = [0.0_f32; 4];
    let mut count = 0.0;
    for py in y.saturating_sub(radius)..=(y + radius).min(image.height() - 1) {
        for px in x.saturating_sub(radius)..=(x + radius).min(image.width() - 1) {
            let pixel = image.linear_radiance()[(py * image.width() + px) as usize];
            for (total, channel) in sum.iter_mut().zip(pixel) {
                *total += channel;
            }
            count += 1.0;
        }
    }
    sum.map(|total| total / count)
}

fn mean_rgb8(rgba8: &[u8], (x, y): (u32, u32), radius: u32) -> f32 {
    let mut sum = 0.0;
    let mut count = 0.0;
    for py in y - radius..=y + radius {
        for px in x - radius..=x + radius {
            let offset = ((py * SIZE + px) * 4) as usize;
            sum += rgba8[offset..offset + 3]
                .iter()
                .map(|channel| f32::from(*channel))
                .sum::<f32>()
                / 3.0;
            count += 1.0;
        }
    }
    sum / count
}

fn uniform_sky(assets: &Assets, radiance: f32) -> crate::EnvironmentHandle {
    let faces = std::array::from_fn(|_| vec![[radiance; 3]; 16]);
    assets.create_environment(
        EnvironmentDesc::from_cubemap_radiance("memory://uniform-sky", 4, faces)
            .expect("uniform sky builds"),
    )
}

#[test]
fn a_white_surface_under_a_uniform_sky_reflects_the_sky_and_no_more() {
    // In a uniformly bright world a white object can at most match its
    // surroundings; anything brighter would be created energy.
    let mut stage = Stage::new(Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO, Vec3::Y);
    stage.mesh(
        GeometryDesc::sphere(0.8, 24, 12),
        matte(1.0),
        Transform::IDENTITY,
    );
    let mut renderer = renderer();
    renderer.set_environment(uniform_sky(&stage.assets, 0.5));
    let image = stage.trace(&renderer, exact(96, 12));
    let center = radiance(&image, (SIZE / 2, SIZE / 2), 3);
    assert!(
        (center[3] - 1.0).abs() < 1.0e-6,
        "the sphere covers the centre: {center:?}"
    );
    assert!(
        center[0] <= 0.5 * 1.01,
        "the sphere outshines the sky: {center:?}"
    );
    assert!(
        center[0] >= 0.5 * 0.85,
        "the sphere lost too much light: {center:?}"
    );
}

type AddLight = fn(&mut Scene);

#[test]
fn every_light_type_exposes_the_floor_as_the_rasterizer_does() {
    let lights: [(&str, AddLight); 4] = [
        ("directional", |scene| {
            let light = DirectionalLight::default()
                .with_illuminance_lux(12_000.0)
                .with_shadows(false);
            scene
                .directional_light(light)
                .transform(facing_down(Vec3::Y))
                .add()
                .expect("light inserts");
        }),
        ("point", |scene| {
            let light = PointLight::default().with_intensity_candela(3.0);
            scene
                .point_light(light)
                .transform(Transform::at(Vec3::new(0.2, 1.4, -0.1)))
                .add()
                .expect("light inserts");
        }),
        ("spot", |scene| {
            let light = SpotLight::default()
                .with_intensity_candela(3.0)
                .with_inner_cone_angle(Angle::from_degrees(25.0))
                .with_outer_cone_angle(Angle::from_degrees(40.0));
            scene
                .spot_light(light)
                .transform(facing_down(Vec3::new(0.0, 1.4, 0.0)))
                .add()
                .expect("light inserts");
        }),
        ("area", |scene| {
            let light = AreaLight::default()
                .with_luminous_flux_lumens(40.0)
                .with_shape(AreaLightShape::rect(0.5, 0.3));
            scene
                .area_light(light)
                .transform(facing_down(Vec3::new(0.0, 1.4, 0.0)))
                .add()
                .expect("light inserts");
        }),
    ];
    for (name, add_light) in lights {
        let mut stage = Stage::top_down();
        stage.fine_floor(4.0, 48);
        add_light(&mut stage.scene);
        let mut renderer = renderer();
        let traced = stage.trace(
            &renderer,
            exact(48, 1).with_directional_light_angular_radius(0.0),
        );
        renderer
            .prepare_with_assets(&mut stage.scene, &stage.assets)
            .expect("scene prepares");
        renderer
            .render(&stage.scene, stage.camera)
            .expect("scene renders");
        let center = (SIZE / 2, SIZE / 2);
        let (traced, rastered) = (
            mean_rgb8(traced.rgba8(), center, 3),
            mean_rgb8(renderer.frame_rgba8(), center, 3),
        );
        assert!(
            rastered > 40.0,
            "{name}: the rasterizer lights the floor ({rastered})"
        );
        assert!(
            (traced - rastered).abs() < 0.08 * rastered + 2.0,
            "{name}: path traced {traced}, rasterized {rastered}"
        );
    }
}

#[test]
fn an_occluder_casts_a_shadow() {
    let mut stage = Stage::top_down();
    stage.floor(6.0);
    stage.mesh(
        GeometryDesc::box_xyz(0.5, 0.5, 1.4),
        matte(0.8),
        Transform::at(Vec3::new(0.0, 0.25, 0.0)),
    );
    // Light travels down and towards -x at a slope of one in two, so the
    // half-metre-high box shades the floor from x = -0.25 to x = -1.25.
    let light = Transform::at(Vec3::ZERO).looking_at(Vec3::new(-2.0, -1.0, 0.0), Vec3::Y);
    let key = DirectionalLight::default()
        .with_illuminance_lux(30_000.0)
        .with_shadows(true);
    stage
        .scene
        .directional_light(key)
        .transform(light)
        .add()
        .expect("light inserts");
    let image = stage.trace(&renderer(), exact(32, 1));
    let lit = radiance(&image, floor_pixel(0.9, 0.0), 1)[0];
    let shadowed = radiance(&image, floor_pixel(-0.8, 0.0), 1)[0];
    assert!(lit > 0.15, "open floor is lit: {lit}");
    assert!(
        shadowed < 0.02 * lit,
        "floor behind the box is shadowed: {shadowed} against {lit}"
    );
}

#[test]
fn a_light_set_not_to_cast_shadows_reaches_behind_the_occluder() {
    let traced = |shadows: bool| {
        let mut stage = Stage::top_down();
        stage.floor(6.0);
        stage.mesh(
            GeometryDesc::box_xyz(0.5, 0.5, 1.4),
            matte(0.8),
            Transform::at(Vec3::new(0.0, 0.25, 0.0)),
        );
        let light = Transform::at(Vec3::ZERO).looking_at(Vec3::new(-2.0, -1.0, 0.0), Vec3::Y);
        let key = DirectionalLight::default()
            .with_illuminance_lux(30_000.0)
            .with_shadows(shadows);
        stage
            .scene
            .directional_light(key)
            .transform(light)
            .add()
            .expect("light inserts");
        let image = stage.trace(&renderer(), exact(16, 1));
        (
            radiance(&image, floor_pixel(0.9, 0.0), 1)[0],
            radiance(&image, floor_pixel(-0.8, 0.0), 1)[0],
        )
    };
    let (open, behind) = traced(false);
    assert!(
        (behind - open).abs() < 0.03 * open,
        "the floor is lit evenly: {open} and {behind}"
    );
    assert!(
        traced(true).1 < 0.02 * open,
        "the same light with shadows leaves the floor behind dark"
    );
}

#[test]
fn light_bounces_onto_surfaces_the_light_cannot_reach() {
    // The wall is parallel to the light, so only the lit floor can light it.
    let mut stage = Stage::new(Vec3::new(-2.5, 0.8, 0.0), Vec3::new(1.0, 0.5, 0.0), Vec3::Y);
    stage.floor(6.0);
    stage.mesh(
        GeometryDesc::box_xyz(0.2, 2.0, 4.0),
        matte(0.8),
        Transform::at(Vec3::new(1.1, 1.0, 0.0)),
    );
    let sun = DirectionalLight::default().with_illuminance_lux(30_000.0);
    stage
        .scene
        .directional_light(sun)
        .transform(facing_down(Vec3::Y))
        .add()
        .expect("light inserts");
    let wall = (SIZE / 2, SIZE / 3);
    let options = |bounces| exact(64, bounces).with_directional_light_angular_radius(0.0);
    let direct = radiance(&stage.trace(&renderer(), options(1)), wall, 2);
    let bounced = radiance(&stage.trace(&renderer(), options(4)), wall, 2);
    assert!(
        (direct[3] - 1.0).abs() < 1.0e-6,
        "the probe looks at the wall"
    );
    assert!(
        direct[0] < 1.0e-4,
        "nothing reaches the wall directly: {direct:?}"
    );
    assert!(bounced[0] > 0.05, "the floor lights the wall: {bounced:?}");
}

#[test]
fn a_glowing_surface_lights_the_floor_as_its_area_and_radiance_predict() {
    let mut stage = Stage::top_down();
    stage.floor(6.0);
    let (side, glow, height) = (0.1_f32, 50.0_f32, 1.0_f32);
    let lamp = MaterialDesc::pbr_metallic_roughness(Color::BLACK, 0.0, 1.0)
        .with_emissive(Color::WHITE)
        .with_emissive_strength(glow)
        .with_double_sided(true);
    stage.mesh(
        GeometryDesc::plane(side, side),
        lamp,
        Transform::at(Vec3::new(0.0, height, 0.0)),
    );
    let image = stage.trace(&renderer(), exact(192, 1));
    // Irradiance from a small parallel emitter: L * A * cos^2 / d^2, with
    // the cosine at both ends equal to height / d.
    let offset = 0.45_f32;
    let distance_squared = offset * offset + height * height;
    let irradiance = glow * side * side * (height * height / distance_squared) / distance_squared;
    let expected = 0.8 / std::f32::consts::PI * irradiance;
    let measured = radiance(&image, floor_pixel(offset, 0.0), 1)[0];
    assert!(
        (measured - expected).abs() < 0.1 * expected,
        "measured {measured}, expected about {expected}"
    );
}

#[test]
fn glass_shows_what_lies_behind_it_minus_its_reflections() {
    let panel = Color::from_linear_rgb(0.9, 0.3, 0.1);
    let glass = MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.0)
        .with_transmission_factor(1.0)
        .with_ior(1.5);
    // A sheet reflects at both faces; with the bounces between them it
    // passes (1 - F) / (1 + F) of the light for F = 4 %.
    let sheet = 0.96 / 1.04;
    // A slab seen face-on passes (1 - F)^2 on the first crossing.
    let slab = 0.96 * 0.96;
    for (name, shape, material, expected) in [
        (
            "thin sheet",
            GeometryDesc::plane(1.5, 1.5),
            glass.clone().with_double_sided(true),
            sheet,
        ),
        (
            "solid slab",
            GeometryDesc::box_xyz(1.5, 0.2, 1.5),
            glass.with_thickness_factor(0.2),
            slab,
        ),
    ] {
        let mut stage = Stage::top_down();
        stage.mesh(
            GeometryDesc::plane(6.0, 6.0),
            MaterialDesc::unlit(panel),
            Transform::IDENTITY,
        );
        stage.mesh(shape, material, Transform::at(Vec3::new(0.0, 1.0, 0.0)));
        let image = stage.trace(&renderer(), exact(96, 8));
        let through = radiance(&image, (SIZE / 2, SIZE / 2), 2);
        let ratio = through[0] / panel.r;
        assert!(
            (ratio - expected).abs() < 0.04,
            "{name}: {ratio} of the panel shows through, expected about {expected}"
        );
        assert!(
            (through[1] / panel.g - ratio).abs() < 0.02,
            "{name}: clear glass keeps the colour: {through:?}"
        );
    }
}

#[test]
fn cut_outs_and_blended_surfaces_pass_the_right_share_of_light() {
    let (front, back) = (
        Color::from_linear_rgb(1.0, 0.0, 0.0),
        Color::from_linear_rgb(0.0, 1.0, 0.0),
    );
    let cases = [
        ("blend", AlphaMode::Blend, 0.25, 0.25),
        ("cut out", AlphaMode::Mask { cutoff: 0.5 }, 0.25, 0.0),
        ("kept", AlphaMode::Mask { cutoff: 0.5 }, 0.75, 1.0),
    ];
    for (name, mode, alpha, expected_front) in cases {
        let mut stage = Stage::top_down();
        stage.mesh(
            GeometryDesc::plane(6.0, 6.0),
            MaterialDesc::unlit(back),
            Transform::IDENTITY,
        );
        let veil = MaterialDesc::unlit(Color::from_linear_rgba(front.r, front.g, front.b, alpha))
            .with_alpha_mode(mode);
        stage.mesh(
            GeometryDesc::plane(6.0, 6.0),
            veil,
            Transform::at(Vec3::new(0.0, 1.0, 0.0)),
        );
        let image = stage.trace(&renderer(), exact(128, 2));
        let seen = radiance(&image, (SIZE / 2, SIZE / 2), 3);
        assert!(
            (seen[0] - expected_front).abs() < 0.03
                && (seen[1] - (1.0 - expected_front)).abs() < 0.03,
            "{name}: saw {seen:?}"
        );
    }
}

#[test]
fn the_same_seed_gives_the_same_image_and_another_seed_another() {
    let mut stage = Stage::top_down();
    stage.floor(6.0);
    stage.mesh(
        GeometryDesc::sphere(0.4, 12, 6),
        matte(0.6),
        Transform::at(Vec3::new(0.0, 0.4, 0.0)),
    );
    stage.scene.add_studio_lighting().expect("lighting inserts");
    let renderer = renderer();
    let options = PathTraceOptions::new()
        .with_samples_per_pixel(4)
        .with_max_bounces(3);
    let first = stage.trace(&renderer, options);
    assert_eq!(
        first,
        stage.trace(&renderer, options),
        "a render is reproducible"
    );
    assert_ne!(
        first.rgba8(),
        stage.trace(&renderer, options.with_seed(7)).rgba8()
    );
}

#[test]
fn a_scene_far_from_the_origin_renders_like_one_at_the_origin() {
    // Coordinates are measured from the camera, so distance from the world
    // origin costs no precision. Offsets are whole numbers, which f32 still
    // represents exactly this far out.
    let render = |shift: Vec3| {
        let eye = Vec3::new(0.0, EYE_HEIGHT, 0.0) + shift;
        let mut stage = Stage::new(eye, shift, Vec3::NEG_Z);
        stage.mesh(
            GeometryDesc::plane(6.0, 6.0),
            matte(0.8),
            Transform::at(shift),
        );
        stage.mesh(
            GeometryDesc::sphere(0.5, 16, 8),
            matte(0.5),
            Transform::at(shift + Vec3::new(0.0, 0.5, 0.0)),
        );
        let light = PointLight::default().with_intensity_candela(6.0);
        stage
            .scene
            .point_light(light)
            .transform(Transform::at(shift + Vec3::new(1.0, 2.0, 0.0)))
            .add()
            .expect("light");
        stage.trace(&renderer(), exact(8, 2))
    };
    let (near, far) = (
        render(Vec3::ZERO),
        render(Vec3::new(2_000_000.0, 0.0, -4_000_000.0)),
    );
    let difference: f32 = near
        .linear_radiance()
        .iter()
        .zip(far.linear_radiance())
        .map(|(a, b)| (a[0] - b[0]).abs())
        .sum::<f32>()
        / near.linear_radiance().len() as f32;
    assert!(difference < 1.0e-3, "mean difference {difference}");
    assert!(
        radiance(&near, floor_pixel(1.0, 0.0), 1)[0] > 0.05,
        "the reference image is lit"
    );
}

#[test]
fn instances_and_mirrored_nodes_cover_the_pixels_the_rasterizer_covers() {
    let mut stage = Stage::new(Vec3::new(1.5, 2.0, 4.0), Vec3::new(0.0, 0.3, 0.0), Vec3::Y);
    let cube = stage
        .assets
        .create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let single_sided = stage.assets.create_material(matte(0.7));
    let (_, set) = stage
        .scene
        .add_instance_set_node(
            stage.scene.root(),
            cube,
            single_sided,
            Transform::at(Vec3::new(-0.8, 0.0, 0.0)),
        )
        .expect("instance set inserts");
    for offset in [
        Vec3::ZERO,
        Vec3::new(0.0, 0.7, 0.0),
        Vec3::new(0.0, 0.0, -0.9),
    ] {
        stage
            .scene
            .push_instance(set, Transform::at(offset))
            .expect("instance inserts");
    }
    // A mirrored node reverses the winding of its single-sided faces.
    let mirrored = Transform::at(Vec3::new(0.9, 0.3, 0.0)).with_scale(Vec3::new(-1.0, 1.4, 1.0));
    stage
        .scene
        .mesh(cube, single_sided)
        .transform(mirrored)
        .add()
        .expect("mesh inserts");
    stage.scene.add_studio_lighting().expect("lighting inserts");

    const WIDE: usize = 48;
    let mut renderer =
        Renderer::headless(WIDE as u32, WIDE as u32).expect("headless renderer builds");
    renderer.set_background(Background::Transparent);
    let traced = stage.trace(&renderer, exact(8, 1));
    renderer
        .prepare_with_assets(&mut stage.scene, &stage.assets)
        .expect("scene prepares");
    renderer
        .render(&stage.scene, stage.camera)
        .expect("scene renders");
    let covered = |rgba8: &[u8]| {
        rgba8
            .chunks_exact(4)
            .map(|pixel| pixel[3] > 127)
            .collect::<Vec<_>>()
    };
    let (traced, rastered) = (covered(traced.rgba8()), covered(renderer.frame_rgba8()));
    // The CPU rasterizer places pixel centres on whole coordinates, half a
    // pixel from where the tracer and the GPU place them, so silhouettes may
    // differ by a pixel. Away from a silhouette the two must agree.
    let mut interior = 0;
    for y in 1..WIDE - 1 {
        for x in 1..WIDE - 1 {
            let around = |mask: &[bool]| {
                (y - 1..=y + 1)
                    .flat_map(|ny| (x - 1..=x + 1).map(move |nx| ny * WIDE + nx))
                    .map(|i| mask[i])
                    .collect::<Vec<_>>()
            };
            let neighbourhood = around(&rastered);
            if neighbourhood.iter().all(|covered| *covered) {
                interior += 1;
                assert!(
                    traced[y * WIDE + x],
                    "the tracer misses a pixel inside a box at ({x}, {y})"
                );
            } else if neighbourhood.iter().all(|covered| !*covered) {
                assert!(
                    !traced[y * WIDE + x],
                    "the tracer covers empty space at ({x}, {y})"
                );
            }
        }
    }
    assert!(
        interior > 150,
        "the rasterizer draws the boxes: {interior} interior pixels"
    );
}

#[test]
fn an_empty_scene_shows_only_the_background() {
    let stage = Stage::top_down();
    let mut renderer = renderer();
    let image = stage.trace(&renderer, exact(2, 2));
    assert!(
        image
            .rgba8()
            .chunks_exact(4)
            .all(|pixel| pixel == [0, 0, 0, 255])
    );
    assert!(
        image
            .linear_radiance()
            .iter()
            .all(|pixel| *pixel == [0.0; 4])
    );
    renderer.set_background(Background::Transparent);
    assert!(
        stage
            .trace(&renderer, exact(2, 2))
            .rgba8()
            .chunks_exact(4)
            .all(|pixel| pixel[3] == 0)
    );
    renderer.set_background(Background::White);
    assert!(
        stage
            .trace(&renderer, exact(2, 2))
            .rgba8()
            .chunks_exact(4)
            .all(|pixel| pixel == [255; 4])
    );
}

#[test]
fn the_denoised_image_keeps_the_brightness_of_the_traced_one() {
    let mut stage = Stage::top_down();
    stage.floor(6.0);
    stage.mesh(
        GeometryDesc::sphere(0.5, 16, 8),
        matte(0.6),
        Transform::at(Vec3::new(0.0, 0.5, 0.0)),
    );
    let mut renderer = renderer();
    renderer.set_environment(uniform_sky(&stage.assets, 1.0));
    let noisy = exact(6, 4);
    let mean = |image: &PathTracedImage| {
        image
            .linear_radiance()
            .iter()
            .map(|pixel| pixel[0])
            .sum::<f32>()
            / 576.0
    };
    let (raw, filtered) = (
        stage.trace(&renderer, noisy),
        stage.trace(&renderer, noisy.with_denoise(true)),
    );
    assert!(filtered.stats().denoised && !raw.stats().denoised);
    assert_ne!(raw.rgba8(), filtered.rgba8());
    assert!(
        (mean(&raw) - mean(&filtered)).abs() < 0.03 * mean(&raw),
        "{} against {}",
        mean(&raw),
        mean(&filtered)
    );
}

#[test]
fn nodes_left_out_of_a_trace_are_not_seen_and_the_scene_is_untouched() {
    let mut stage = Stage::top_down();
    stage.mesh(
        GeometryDesc::plane(6.0, 6.0),
        MaterialDesc::unlit(Color::from_linear_rgb(0.0, 1.0, 0.0)),
        Transform::IDENTITY,
    );
    let cover = MaterialDesc::unlit(Color::from_linear_rgb(1.0, 0.0, 0.0));
    let cover = stage.mesh(
        GeometryDesc::plane(6.0, 6.0),
        cover,
        Transform::at(Vec3::new(0.0, 1.0, 0.0)),
    );
    let revision = stage.scene.dirty_state();
    let renderer = renderer();
    let center = (SIZE / 2, SIZE / 2);
    let with_cover = stage.trace(&renderer, exact(2, 1));
    assert!(
        radiance(&with_cover, center, 2)[0] > 0.99,
        "the cover hides the floor"
    );
    let without_cover = renderer
        .path_trace_without(
            &stage.scene,
            &stage.assets,
            stage.camera,
            &exact(2, 1),
            &[cover],
            &|_, _| {},
        )
        .expect("the scene path traces");
    let seen = radiance(&without_cover, center, 2);
    assert!(
        seen[0] < 0.01 && seen[1] > 0.99,
        "the floor shows once the cover is left out: {seen:?}"
    );
    assert_eq!(
        without_cover.stats().triangles,
        with_cover.stats().triangles - 2
    );
    assert_eq!(
        stage.scene.dirty_state(),
        revision,
        "tracing does not change the scene"
    );
}

#[test]
fn a_traced_image_can_be_developed_again_at_another_exposure() {
    let mut stage = Stage::top_down();
    stage.floor(6.0);
    let sun = DirectionalLight::default().with_illuminance_lux(8_000.0);
    stage
        .scene
        .directional_light(sun)
        .transform(facing_down(Vec3::Y))
        .add()
        .expect("light inserts");
    let mut renderer = renderer();
    let mut image = stage.trace(&renderer, exact(4, 1));
    let (first, linear) = (image.rgba8().to_vec(), image.linear_radiance().to_vec());
    renderer.set_exposure_ev(1.0);
    renderer.develop_path_traced(&mut image);
    let center = (SIZE / 2, SIZE / 2);
    assert!(
        mean_rgb8(image.rgba8(), center, 3) > mean_rgb8(&first, center, 3) + 20.0,
        "a stop more exposure is brighter"
    );
    assert_eq!(
        image.linear_radiance(),
        linear.as_slice(),
        "the radiance is untouched"
    );
    // Developing is the same step a fresh trace ends with.
    assert_eq!(image.rgba8(), stage.trace(&renderer, exact(4, 1)).rgba8());
    renderer.set_exposure_ev(0.0);
    renderer.develop_path_traced(&mut image);
    assert_eq!(image.rgba8(), first.as_slice());
}

#[test]
fn stats_and_progress_describe_the_render() {
    let mut stage = Stage::top_down();
    stage.mesh(
        GeometryDesc::box_xyz(1.0, 1.0, 1.0),
        matte(0.5),
        Transform::IDENTITY,
    );
    let line = GeometryDesc::try_new(
        GeometryTopology::Lines,
        [Vec3::ZERO, Vec3::X]
            .map(|position| GeometryVertex {
                position,
                normal: Vec3::Y,
            })
            .to_vec(),
        vec![0, 1],
    )
    .expect("line geometry builds");
    stage.mesh(
        line,
        MaterialDesc::line(Color::WHITE, 0.01),
        Transform::IDENTITY,
    );
    stage.scene.add_studio_lighting().expect("lighting inserts");
    let rows = std::sync::Mutex::new(Vec::new());
    let image = renderer()
        .path_trace_with_progress(
            &stage.scene,
            &stage.assets,
            stage.camera,
            &exact(2, 2),
            &|done, total| {
                rows.lock().expect("progress lock").push((done, total));
            },
        )
        .expect("the scene path traces");
    let stats = image.stats();
    assert_eq!(
        (stats.triangles, stats.materials, stats.skipped_stroke_nodes),
        (12, 1, 1)
    );
    assert_eq!(
        (
            stats.samples_per_pixel,
            stats.max_bounces,
            stats.camera_paths
        ),
        (2, 2, 2 * 576)
    );
    assert!(
        stats.lights >= 2
            && stats.emissive_triangles == 0
            && stats.environment_resolution.is_none()
    );
    let mut rows = rows.into_inner().expect("progress lock");
    rows.sort_unstable();
    assert_eq!(rows, (1..=SIZE).map(|row| (row, SIZE)).collect::<Vec<_>>());
}

#[test]
fn missing_inputs_are_reported_instead_of_rendered_around() {
    let stage = Stage::top_down();
    let renderer = renderer();
    let foreign_camera = stage.camera;
    let empty = Scene::new();
    assert_eq!(
        renderer.path_trace(&empty, &stage.assets, foreign_camera, &exact(1, 1)),
        Err(PathTraceError::CameraNotFound(foreign_camera))
    );
    // A scene whose mesh lives in other assets than the ones passed in.
    let mut stage = Stage::top_down();
    let node = stage.mesh(
        GeometryDesc::box_xyz(1.0, 1.0, 1.0),
        matte(0.5),
        Transform::IDENTITY,
    );
    let error = renderer.path_trace(&stage.scene, &Assets::new(), stage.camera, &exact(1, 1));
    assert_eq!(error, Err(PathTraceError::GeometryNotFound { node }));
    assert!(
        error
            .unwrap_err()
            .to_string()
            .contains("not in the assets passed to path_trace")
    );
    // An environment handle the given assets do not hold.
    let mut lit = Renderer::headless(SIZE, SIZE).expect("headless renderer builds");
    lit.set_environment(uniform_sky(&Assets::new(), 1.0));
    let error = lit.path_trace(&stage.scene, &stage.assets, stage.camera, &exact(1, 1));
    assert!(
        matches!(error, Err(PathTraceError::EnvironmentNotFound { .. })),
        "{error:?}"
    );
}

mod detail;
