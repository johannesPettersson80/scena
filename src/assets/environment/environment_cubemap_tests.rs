use super::*;

const NEUTRAL_STUDIO_FIXTURE: &str =
    include_str!("../../../tests/assets/environment/generated/neutral-studio-cubemap.fixture.toml");

#[test]
fn cubemap_fixture_parser_decodes_six_faces_with_real_radiance_values() {
    let parsed = EnvironmentCubemapFaces::try_parse_fixture(NEUTRAL_STUDIO_FIXTURE)
        .expect("bundled SCENA_CUBEMAP_V1 fixture must parse");
    assert_eq!(parsed.resolution, 256, "fixture declares 256-pixel faces");
    assert_eq!(
        parsed.face_radiance,
        [
            [0.78, 0.82, 0.88],
            [0.62, 0.68, 0.76],
            [1.00, 0.98, 0.92],
            [0.28, 0.30, 0.34],
            [0.70, 0.74, 0.82],
            [0.56, 0.60, 0.68],
        ],
        "parser must read face radiance in the WebGPU px/nx/py/ny/pz/nz layer order"
    );
}

#[test]
fn equirectangular_hdr_default_cubemap_resolution_matches_specular_ibl_fixture_resolution() {
    assert_eq!(
        DEFAULT_ENVIRONMENT_CUBEMAP_FACE_RESOLUTION, 256,
        "real HDR environments need 256-pixel cubemap faces so smooth metals keep \
         enough environment detail for specular IBL; 64-pixel faces flatten chrome"
    );
}

#[test]
fn cubemap_fixture_parser_rejects_invalid_magic_header() {
    assert!(
        EnvironmentCubemapFaces::try_parse_fixture(
            "OOPS_NOT_A_CUBEMAP\n[face.px]\nradiance = 1.0 1.0 1.0"
        )
        .is_none(),
        "missing magic header must not silently degrade to a default cubemap"
    );
}

#[test]
fn cubemap_fixture_parser_rejects_negative_radiance() {
    let bad = "SCENA_CUBEMAP_V1\nresolution = 4\n[face.px]\nradiance = -0.1 0.0 0.0\n";
    assert!(
        EnvironmentCubemapFaces::try_parse_fixture(bad).is_none(),
        "negative radiance is physically meaningless and must fail parsing"
    );
}

#[test]
fn cube_face_direction_at_face_center_returns_face_normal() {
    for (face_index, normal) in ENVIRONMENT_CUBEMAP_FACE_NORMALS.iter().enumerate() {
        let direction = cube_face_direction(face_index, 0.0, 0.0);
        let expected = Vec3::new(normal[0], normal[1], normal[2]);
        let dx = direction.x - expected.x;
        let dy = direction.y - expected.y;
        let dz = direction.z - expected.z;
        assert!(
            dx * dx + dy * dy + dz * dz < 1e-6,
            "face {face_index} center direction must equal the face normal"
        );
    }
}

#[test]
fn cubemap_face_pixels_at_face_center_recover_face_radiance() {
    let mut radiance = [[0.0_f32; 3]; 6];
    radiance[0] = [0.9, 0.1, 0.1];
    radiance[1] = [0.1, 0.9, 0.1];
    radiance[2] = [0.1, 0.1, 0.9];
    radiance[3] = [0.5, 0.4, 0.3];
    radiance[4] = [0.3, 0.4, 0.5];
    radiance[5] = [0.7, 0.7, 0.7];
    let cube = EnvironmentCubemapFaces {
        face_radiance: radiance,
        resolution: 8,
        face_pixels: None,
    };
    let pixels = cube.build_face_pixels_rgba32f();
    for (face_index, face_pixels) in pixels.iter().enumerate() {
        let center_pixel_index = ((4 * 8) + 4) * 4;
        let r = face_pixels[center_pixel_index];
        let g = face_pixels[center_pixel_index + 1];
        let b = face_pixels[center_pixel_index + 2];
        let a = face_pixels[center_pixel_index + 3];
        let expected = radiance[face_index];
        let dominant = expected.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        assert!(
            a == 1.0 && (r - g - b).abs() < 1.0,
            "face {face_index} center alpha must be 1 and the radiance triplet is finite",
        );
        for (channel, raw) in [r, g, b].iter().enumerate() {
            if (expected[channel] - dominant).abs() < 1e-6 {
                assert!(
                    *raw > expected[channel] * 0.6,
                    "face {face_index} dominant channel must retain >60% of its face-center radiance"
                );
            }
        }
    }
}

#[test]
fn cubemap_face_pixels_at_face_corners_blend_three_adjacent_faces() {
    let radiance = [
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 0.0],
    ];
    let cube = EnvironmentCubemapFaces {
        face_radiance: radiance,
        resolution: 4,
        face_pixels: None,
    };
    let pixels = cube.build_face_pixels_rgba32f();
    let resolution = 4_usize;
    let face_pixels = &pixels[0];
    let top_left_index = 0;
    let r = face_pixels[top_left_index];
    let g = face_pixels[top_left_index + 1];
    let b = face_pixels[top_left_index + 2];
    assert!(
        r > 0.0 && g > 0.0 && b > 0.0,
        "px face top-left corner direction (+X,+Y,+Z) must blend px=red, py=green, pz=blue \
         radiances; got r={r} g={g} b={b}"
    );
    let bottom_right_index = ((resolution - 1) * resolution + (resolution - 1)) * 4;
    let r2 = face_pixels[bottom_right_index];
    let g2 = face_pixels[bottom_right_index + 1];
    let b2 = face_pixels[bottom_right_index + 2];
    assert!(
        r2 > 0.0 && g2 == 0.0 && b2 == 0.0,
        "px face (-Y,-Z) corner must keep red but drop py/pz contributions; \
         got r={r2} g={g2} b={b2}"
    );
}

#[test]
fn lambertian_irradiance_averages_six_face_radiances() {
    let radiance = [
        [0.78, 0.82, 0.88],
        [0.62, 0.68, 0.76],
        [1.00, 0.98, 0.92],
        [0.28, 0.30, 0.34],
        [0.70, 0.74, 0.82],
        [0.56, 0.60, 0.68],
    ];
    let cube = EnvironmentCubemapFaces {
        face_radiance: radiance,
        resolution: 64,
        face_pixels: None,
    };
    let irradiance = cube.lambertian_irradiance();
    let expected = [
        (0.78 + 0.62 + 1.00 + 0.28 + 0.70 + 0.56) / 6.0,
        (0.82 + 0.68 + 0.98 + 0.30 + 0.74 + 0.60) / 6.0,
        (0.88 + 0.76 + 0.92 + 0.34 + 0.82 + 0.68) / 6.0,
    ];
    for channel in 0..3 {
        assert!(
            (irradiance[channel] - expected[channel]).abs() < 1e-5,
            "channel {channel} mean = {} must equal six-face average = {}",
            irradiance[channel],
            expected[channel]
        );
    }
}
