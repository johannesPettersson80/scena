use std::collections::BTreeSet;

use scena::GeometryDesc;

#[test]
fn generated_showcase_triangle_meshes_carry_non_degenerate_uvs() {
    for (name, geometry) in [
        ("box_xyz", GeometryDesc::box_xyz(1.0, 1.0, 1.0)),
        ("sphere", GeometryDesc::sphere(1.0, 32, 16)),
        ("cylinder", GeometryDesc::cylinder(1.0, 1.0, 32)),
        ("plane", GeometryDesc::plane(1.0, 1.0)),
    ] {
        assert_uvs_cover_surface(name, &geometry);
    }
}

fn assert_uvs_cover_surface(name: &str, geometry: &GeometryDesc) {
    assert_eq!(
        geometry.vertices().len(),
        geometry.tex_coords0().len(),
        "{name} must have one texcoord per vertex"
    );

    let unique = geometry
        .tex_coords0()
        .iter()
        .map(|uv| {
            (
                (uv[0] * 1000.0).round() as i32,
                (uv[1] * 1000.0).round() as i32,
            )
        })
        .collect::<BTreeSet<_>>();
    assert!(
        unique.len() >= 4,
        "{name} must not collapse material textures to a single texel; got {unique:?}"
    );

    let (min_u, max_u, min_v, max_v) = geometry.tex_coords0().iter().fold(
        (f32::MAX, f32::MIN, f32::MAX, f32::MIN),
        |(min_u, max_u, min_v, max_v), uv| {
            (
                min_u.min(uv[0]),
                max_u.max(uv[0]),
                min_v.min(uv[1]),
                max_v.max(uv[1]),
            )
        },
    );
    assert!(
        max_u - min_u >= 0.9 && max_v - min_v >= 0.9,
        "{name} UVs must span the full material texture domain, got u={min_u}..{max_u} v={min_v}..{max_v}"
    );
}
