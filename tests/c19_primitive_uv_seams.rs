#![cfg(not(target_arch = "wasm32"))]

use std::io::Cursor;

use base64::Engine as _;
use scena::{
    AntiAliasing, Assets, Background, Color, FramingOptions, GeometryDesc, GeometryTopology,
    GeometryVertex, MaterialDesc, Renderer, Scene, TextureColorSpace, Vec3,
};

const SEGMENTS: u32 = 12;

#[test]
fn cylinder_side_rows_duplicate_the_u1_seam_without_changing_caps_or_indices() {
    let geometry = GeometryDesc::cylinder(1.0, 2.0, SEGMENTS);
    let row = SEGMENTS as usize + 1;

    assert_eq!(geometry.vertices().len(), 52);
    assert_eq!(geometry.indices().len(), SEGMENTS as usize * 12);
    for ring in 0..2 {
        let first = ring * row;
        let seam = first + SEGMENTS as usize;
        assert_eq!(geometry.vertices()[first], geometry.vertices()[seam]);
        assert_eq!(geometry.tex_coords0()[first], [0.0, ring as f32]);
        assert_eq!(geometry.tex_coords0()[seam], [1.0, ring as f32]);
    }

    for segment in 0..SEGMENTS as usize {
        let triangle_indices = &geometry.indices()[segment * 12..segment * 12 + 6];
        let (min_u, max_u) = triangle_indices
            .iter()
            .map(|index| geometry.tex_coords0()[*index as usize][0])
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), u| {
                (min.min(u), max.max(u))
            });
        assert!(
            max_u - min_u <= 1.0 / SEGMENTS as f32 + 1.0e-6,
            "cylinder side segment {segment} spans u={min_u}..{max_u}"
        );
    }
}

#[test]
fn cone_last_face_uses_a_distinct_u1_base_vertex_and_local_tip_uv() {
    let geometry = GeometryDesc::cone(1.0, 2.0, SEGMENTS);
    assert_eq!(geometry.vertices().len(), 49);
    assert_eq!(geometry.indices().len(), SEGMENTS as usize * 6);

    let first_base = 0;
    let last_face = (SEGMENTS as usize - 1) * 3;
    let last_base_seam = last_face + 1;
    assert_eq!(
        geometry.vertices()[first_base].position,
        geometry.vertices()[last_base_seam].position
    );
    assert_eq!(geometry.tex_coords0()[first_base][0], 0.0);
    assert_eq!(geometry.tex_coords0()[last_base_seam][0], 1.0);
    assert_eq!(
        geometry.tex_coords0()[last_face + 2][0],
        (SEGMENTS as f32 - 0.5) / SEGMENTS as f32
    );

    for segment in 0..SEGMENTS as usize {
        let triangle_indices = &geometry.indices()[segment * 3..segment * 3 + 3];
        let (min_u, max_u) = triangle_indices
            .iter()
            .map(|index| geometry.tex_coords0()[*index as usize][0])
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), u| {
                (min.min(u), max.max(u))
            });
        assert!(
            max_u - min_u <= 1.0 / SEGMENTS as f32 + 1.0e-6,
            "cone side segment {segment} spans u={min_u}..{max_u}"
        );
    }
}

#[test]
fn rendered_last_cylinder_quad_crosses_only_its_local_checker_boundary() {
    let cylinder = GeometryDesc::cylinder(1.0, 2.0, SEGMENTS);
    let last_start = (SEGMENTS as usize - 1) * 12;
    let last = &cylinder.indices()[last_start..last_start + 6];
    let source_indices = [last[0], last[1], last[2], last[5]];
    let uvs = source_indices.map(|index| cylinder.tex_coords0()[index as usize]);
    let mut known_bad_uvs = uvs;
    known_bad_uvs[2][0] = 0.0;
    known_bad_uvs[3][0] = 0.0;

    let corrected_transitions = render_checker_transitions(uvs);
    let wrapped_transitions = render_checker_transitions(known_bad_uvs);

    assert!(
        corrected_transitions >= 1,
        "the corrected last quad must still show its local checker boundary"
    );
    assert!(
        corrected_transitions <= 3,
        "the corrected last quad must sample only u=11/12..1: transitions={corrected_transitions}"
    );
    assert!(
        wrapped_transitions >= 4 && corrected_transitions < wrapped_transitions,
        "the oracle must reject the old u=11/12..0 interpolation: corrected={corrected_transitions}, wrapped={wrapped_transitions}"
    );
}

fn render_checker_transitions(uvs: [[f32; 2]; 4]) -> usize {
    const WIDTH: u32 = 192;
    const HEIGHT: u32 = 128;
    let quad = GeometryDesc::try_new_with_vertex_colors_and_tex_coords(
        GeometryTopology::Triangles,
        [
            Vec3::new(-1.0, -0.7, 0.0),
            Vec3::new(-1.0, 0.7, 0.0),
            Vec3::new(1.0, -0.7, 0.0),
            Vec3::new(1.0, 0.7, 0.0),
        ]
        .map(|position| GeometryVertex {
            position,
            normal: Vec3::Z,
        })
        .to_vec(),
        vec![0, 1, 2, 2, 1, 3],
        vec![Color::WHITE; 4],
        uvs.to_vec(),
    )
    .expect("last cylinder side quad validates");

    let assets = Assets::new();
    let texture =
        pollster::block_on(assets.load_texture(checker_data_uri(), TextureColorSpace::Srgb))
            .expect("checker texture loads");
    let geometry = assets.create_geometry(quad);
    let material = assets.create_material(
        MaterialDesc::unlit(Color::WHITE)
            .with_base_color_texture(texture)
            .with_double_sided(true),
    );
    let mut scene = Scene::new();
    scene.mesh(geometry, material).add().expect("quad inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .frame_all_with_assets_and_options(
            camera,
            &assets,
            FramingOptions::new()
                .front()
                .fill(0.72)
                .viewport(WIDTH, HEIGHT),
        )
        .expect("quad frames");
    let mut renderer = Renderer::headless(WIDTH, HEIGHT).expect("renderer builds");
    renderer.set_anti_aliasing(AntiAliasing::None);
    renderer.set_background(Background::Black);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("quad prepares");
    renderer.render_active(&scene).expect("quad renders");

    let frame = renderer.frame_rgba8();
    let (min_x, min_y, max_x, max_y) =
        nonblack_bounds(frame, WIDTH, HEIGHT).expect("checker quad produces visible pixels");
    let y = (min_y + max_y) / 2;
    let mut classes = Vec::new();
    for x in min_x..=max_x {
        let offset = ((y * WIDTH + x) * 4) as usize;
        let pixel = &frame[offset..offset + 4];
        if pixel[0].max(pixel[2]) > 20 {
            classes.push(pixel[0] >= pixel[2]);
        }
    }
    let transitions = classes.windows(2).filter(|pair| pair[0] != pair[1]).count();
    assert!(!classes.is_empty(), "checker classification finds samples");
    transitions
}

fn checker_data_uri() -> String {
    const WIDTH: u32 = 16;
    const HEIGHT: u32 = 2;
    let mut rgba = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for _y in 0..HEIGHT {
        for x in 0..WIDTH {
            let pixel = if x % 2 == 0 {
                [240, 32, 32, 255]
            } else {
                [32, 32, 240, 255]
            };
            rgba.extend_from_slice(&pixel);
        }
    }
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut png), WIDTH, HEIGHT);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("checker PNG header writes");
        writer
            .write_image_data(&rgba)
            .expect("checker PNG pixels write");
    }
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    )
}

fn nonblack_bounds(frame: &[u8], width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    for y in 0..height {
        for x in 0..width {
            let offset = ((y * width + x) * 4) as usize;
            if frame[offset..offset + 3].iter().any(|channel| *channel > 8) {
                bounds = Some(match bounds {
                    Some((min_x, min_y, max_x, max_y)) => {
                        (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
                    }
                    None => (x, y, x, y),
                });
            }
        }
    }
    bounds
}
