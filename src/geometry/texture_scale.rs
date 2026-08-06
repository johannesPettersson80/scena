use super::{GeometryDesc, GeometryTopology};
use crate::Vec3;

impl GeometryDesc {
    pub(crate) fn photographic_texture_uv_scale(
        &self,
        world_scale: Vec3,
        tile_size_m: f32,
    ) -> Option<[f32; 2]> {
        if self.topology != GeometryTopology::Triangles
            || !tile_size_m.is_finite()
            || tile_size_m <= 0.0
        {
            return None;
        }
        let tex_coords = self.authored_tex_coords0()?;
        let scale = world_scale.abs();
        let mut metres_per_u = Vec::with_capacity(self.indices.len() / 3);
        let mut metres_per_v = Vec::with_capacity(self.indices.len() / 3);
        for triangle in self.indices.chunks_exact(3) {
            let p0 = self.vertices[triangle[0] as usize].position * scale;
            let p1 = self.vertices[triangle[1] as usize].position * scale;
            let p2 = self.vertices[triangle[2] as usize].position * scale;
            let uv0 = tex_coords[triangle[0] as usize];
            let uv1 = tex_coords[triangle[1] as usize];
            let uv2 = tex_coords[triangle[2] as usize];
            let du1 = uv1[0] - uv0[0];
            let dv1 = uv1[1] - uv0[1];
            let du2 = uv2[0] - uv0[0];
            let dv2 = uv2[1] - uv0[1];
            let determinant = du1 * dv2 - dv1 * du2;
            if !determinant.is_finite() || determinant.abs() <= 1.0e-10 {
                continue;
            }
            let inverse = determinant.recip();
            let edge1 = p1 - p0;
            let edge2 = p2 - p0;
            let dp_du = (edge1 * dv2 - edge2 * dv1) * inverse;
            let dp_dv = (edge2 * du1 - edge1 * du2) * inverse;
            let u = dp_du.length();
            let v = dp_dv.length();
            if u.is_finite() && u > 1.0e-8 {
                metres_per_u.push(u);
            }
            if v.is_finite() && v > 1.0e-8 {
                metres_per_v.push(v);
            }
        }
        let u = median_finite(&mut metres_per_u)?;
        let v = median_finite(&mut metres_per_v)?;
        Some([
            (u / tile_size_m).clamp(0.01, 1_024.0),
            (v / tile_size_m).clamp(0.01, 1_024.0),
        ])
    }
}

fn median_finite(values: &mut [f32]) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f32::total_cmp);
    let middle = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) * 0.5
    } else {
        values[middle]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn photographic_texture_scale_uses_physical_triangle_derivatives() {
        let geometry = GeometryDesc::plane(0.4, 0.2);

        let scale = geometry
            .photographic_texture_uv_scale(Vec3::ONE, 0.1)
            .expect("authored nondegenerate UVs produce a physical scale");

        assert!((scale[0] - 4.0).abs() <= 1.0e-5, "{scale:?}");
        assert!((scale[1] - 2.0).abs() <= 1.0e-5, "{scale:?}");
    }
}
