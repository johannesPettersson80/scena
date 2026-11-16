//! Cascaded directional shadow fitting.
//!
//! Render preparation fits one orthographic `light_from_world` around every
//! occluder in the scene. That single frustum wastes almost all of its
//! resolution when a small subject stands on a large floor. Each cascade is a
//! crop of that scene frustum around one depth slice of the camera frustum, so
//! texels are spent where the camera is looking. The crop only scales and
//! offsets light-space XY: depth keeps the full scene range, so occluders
//! between the light and the slice still cast into it.
//!
//! The fit needs only the camera and the prepared light matrix, so it runs per
//! frame without touching render preparation.

use glam::{Mat4, Vec3, Vec4};

use crate::render::camera::CameraProjection;

pub(super) const MAX_SHADOW_CASCADES: usize = 4;

/// Receivers are pushed this many shadow texels along their geometric normal
/// before projection, which removes self-shadowing stripes on curved surfaces.
const NORMAL_OFFSET_TEXELS: f32 = 1.5;
/// Tangent of the light's angular radius. A studio key is far larger than the
/// sun (0.0047); this value gives penumbrae that harden visibly at contact.
const DEFAULT_LIGHT_TANGENT: f32 = 0.03;
/// Blend between logarithmic (1.0) and uniform (0.0) split spacing.
const SPLIT_LAMBDA: f32 = 0.8;
/// Lower bound on the near split distance as a fraction of the far distance.
const MIN_NEAR_TO_FAR_RATIO: f32 = 0.01;
/// Border kept around each crop so the widest filter kernel stays inside it.
const CROP_BORDER_TEXELS: f32 = 26.0;
/// Edge of one cascade cell relative to the 2x2 atlas.
const ATLAS_SCALE: f32 = 0.5;

/// Shadow data appended to the camera uniform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::render) struct ShadowCascadeUniform {
    pub(in crate::render) light_from_world: [[f32; 16]; MAX_SHADOW_CASCADES],
    /// View-space far distance of each cascade.
    pub(in crate::render) splits: [f32; MAX_SHADOW_CASCADES],
    /// `[cascade count, atlas scale per cascade, normal offset in texels,
    /// light angular-radius tangent]`.
    pub(in crate::render) params: [f32; 4],
}

impl ShadowCascadeUniform {
    /// One cascade covering the whole scene-fitted frustum.
    pub(in crate::render) fn single(light_from_world: [f32; 16]) -> Self {
        Self {
            light_from_world: [light_from_world; MAX_SHADOW_CASCADES],
            splits: [f32::MAX; MAX_SHADOW_CASCADES],
            params: [1.0, 1.0, NORMAL_OFFSET_TEXELS, DEFAULT_LIGHT_TANGENT],
        }
    }

    #[cfg(test)]
    pub(in crate::render) fn cascade_count(&self) -> usize {
        (self.params[0] as usize).clamp(1, MAX_SHADOW_CASCADES)
    }

    /// Fits `cascade_count` crops of `light_from_world` to the camera frustum.
    ///
    /// `cascade_resolution` is the edge length of one cascade in texels. Falls
    /// back to [`Self::single`] when the camera or light matrix is degenerate.
    pub(in crate::render) fn fit(
        camera: &CameraProjection,
        light_from_world: [f32; 16],
        cascade_count: usize,
        cascade_resolution: u32,
    ) -> Self {
        let cascade_count = cascade_count.clamp(1, MAX_SHADOW_CASCADES);
        if cascade_count == 1 {
            return Self::single(light_from_world);
        }
        // Past this point the map is an atlas. A fit that cannot be computed
        // still renders the scene matrix into every cell, so the fallback
        // must read one cell rather than the whole atlas.
        let mut single = Self::single(light_from_world);
        single.params[1] = ATLAS_SCALE;
        let (Some(view_from_world), Some(world_from_view), Some(view_from_clip)) = (
            camera.view_from_world_matrix(),
            camera.world_from_view_matrix(),
            camera.view_from_clip_matrix(),
        ) else {
            return single;
        };
        let light = Mat4::from_cols_array(&light_from_world);
        let world_from_light = light.inverse();
        if !world_from_light.is_finite() || light.determinant().abs() <= f32::MIN_POSITIVE {
            return single;
        }
        let view_from_world = Mat4::from_cols_array(&view_from_world);
        let world_from_view = Mat4::from_cols_array(&world_from_view);
        let Some(rays) = FrustumRays::new(Mat4::from_cols_array(&view_from_clip)) else {
            return single;
        };

        // Shadows only matter where the scene is: clamp the camera range to
        // the view depths spanned by the scene-fitted light volume.
        let (mut scene_near, mut scene_far) = (f32::INFINITY, f32::NEG_INFINITY);
        for corner in ndc_box_corners() {
            let world = world_from_light.project_point3(corner);
            let depth = -view_from_world.transform_point3(world).z;
            scene_near = scene_near.min(depth);
            scene_far = scene_far.max(depth);
        }
        let far = scene_far.min(rays.far);
        // A camera inside the shadowed volume would otherwise start the
        // logarithmic splits at its near plane and spend every cascade on the
        // first few centimetres.
        let near = scene_near.max(rays.near).max(far * MIN_NEAR_TO_FAR_RATIO);
        if !near.is_finite() || !far.is_finite() || far <= near * 1.0001 {
            return single;
        }

        let mut uniform = single;
        uniform.params = [
            cascade_count as f32,
            ATLAS_SCALE,
            NORMAL_OFFSET_TEXELS,
            DEFAULT_LIGHT_TANGENT,
        ];
        let border = CROP_BORDER_TEXELS / cascade_resolution.max(1) as f32;
        let mut slice_near = near;
        for cascade in 0..cascade_count {
            let slice_far = split_distance(near, far, cascade + 1, cascade_count);
            uniform.splits[cascade] = slice_far;
            let mut min = Vec3::splat(f32::INFINITY);
            let mut max = Vec3::splat(f32::NEG_INFINITY);
            for depth in [slice_near, slice_far] {
                for corner in rays.corners_at(depth) {
                    let world = world_from_view.transform_point3(corner);
                    let ndc = light.project_point3(world);
                    min = min.min(ndc);
                    max = max.max(ndc);
                }
            }
            // Never crop wider than the scene fit, and keep a filter border.
            let extent = (max - min).max(Vec3::splat(1.0e-4));
            let padded_min = (min - extent * border).max(Vec3::splat(-1.0));
            let padded_max = (max + extent * border).min(Vec3::splat(1.0));
            if padded_max.x - padded_min.x > 1.0e-5 && padded_max.y - padded_min.y > 1.0e-5 {
                let scale_x = 2.0 / (padded_max.x - padded_min.x);
                let scale_y = 2.0 / (padded_max.y - padded_min.y);
                let crop = Mat4::from_cols(
                    Vec4::new(scale_x, 0.0, 0.0, 0.0),
                    Vec4::new(0.0, scale_y, 0.0, 0.0),
                    Vec4::new(0.0, 0.0, 1.0, 0.0),
                    Vec4::new(
                        -(padded_max.x + padded_min.x) * 0.5 * scale_x,
                        -(padded_max.y + padded_min.y) * 0.5 * scale_y,
                        0.0,
                        1.0,
                    ),
                );
                uniform.light_from_world[cascade] = (crop * light).to_cols_array();
            }
            slice_near = slice_far;
        }
        // The last cascade must cover everything beyond its nominal split.
        uniform.splits[cascade_count - 1] = f32::MAX;
        uniform
    }
}

/// View-space corner rays of the camera frustum, valid for perspective and
/// orthographic projections.
struct FrustumRays {
    near_corners: [Vec3; 4],
    far_corners: [Vec3; 4],
    near: f32,
    far: f32,
}

impl FrustumRays {
    fn new(view_from_clip: Mat4) -> Option<Self> {
        let unproject = |x: f32, y: f32, z: f32| view_from_clip.project_point3(Vec3::new(x, y, z));
        let xy = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
        let mut first = xy.map(|(x, y)| unproject(x, y, 0.0));
        let mut second = xy.map(|(x, y)| unproject(x, y, 1.0));
        // Reversed-Z projections put the far plane at clip depth 0.
        if -first[0].z > -second[0].z {
            std::mem::swap(&mut first, &mut second);
        }
        let near = -first[0].z;
        let far = -second[0].z;
        (first.iter().chain(&second).all(|corner| corner.is_finite())
            && near.is_finite()
            && far.is_finite()
            && near >= 0.0
            && far > near)
            .then_some(Self {
                near_corners: first,
                far_corners: second,
                near,
                far,
            })
    }

    fn corners_at(&self, depth: f32) -> [Vec3; 4] {
        let t = (depth - self.near) / (self.far - self.near);
        std::array::from_fn(|index| self.near_corners[index].lerp(self.far_corners[index], t))
    }
}

fn ndc_box_corners() -> [Vec3; 8] {
    std::array::from_fn(|index| {
        Vec3::new(
            if index & 1 == 0 { -1.0 } else { 1.0 },
            if index & 2 == 0 { -1.0 } else { 1.0 },
            if index & 4 == 0 { 0.0 } else { 1.0 },
        )
    })
}

fn split_distance(near: f32, far: f32, index: usize, count: usize) -> f32 {
    let fraction = index as f32 / count as f32;
    let logarithmic = near.max(1.0e-4) * (far / near.max(1.0e-4)).powf(fraction);
    let uniform = near + (far - near) * fraction;
    SPLIT_LAMBDA * logarithmic + (1.0 - SPLIT_LAMBDA) * uniform
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_cascade_reuses_the_scene_matrix_and_never_splits() {
        let matrix = Mat4::orthographic_rh(-5.0, 5.0, -5.0, 5.0, 0.0, 20.0).to_cols_array();
        let uniform = ShadowCascadeUniform::single(matrix);
        assert_eq!(uniform.cascade_count(), 1);
        assert_eq!(uniform.light_from_world[0], matrix);
        assert_eq!(uniform.params[1], 1.0, "one cascade owns the whole map");
        assert!(uniform.splits.iter().all(|split| *split == f32::MAX));
    }

    fn test_camera() -> CameraProjection {
        use crate::diagnostics::Backend;
        use crate::render::RasterTarget;
        use crate::scene::{PerspectiveCamera, Scene, Transform, Vec3};

        let mut scene = Scene::new();
        let camera = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::default(),
                Transform::at(Vec3::new(0.0, 1.0, 6.0)),
            )
            .expect("camera inserts");
        let target = RasterTarget {
            width: 64,
            height: 64,
            backend: Backend::HeadlessGpu,
        };
        CameraProjection::from_scene(&scene, camera, target).expect("projection")
    }

    #[test]
    fn fitted_cascades_crop_the_scene_matrix_and_cover_the_far_range() {
        let camera = test_camera();
        let scene_fit = Mat4::orthographic_rh(-40.0, 40.0, -40.0, 40.0, 0.0, 100.0)
            * Mat4::look_at_rh(Vec3::new(0.0, 50.0, 0.0), Vec3::ZERO, Vec3::Z);
        let uniform = ShadowCascadeUniform::fit(&camera, scene_fit.to_cols_array(), 4, 1024);
        assert_eq!(uniform.cascade_count(), 4);
        assert_eq!(uniform.params[1], ATLAS_SCALE);
        assert_eq!(
            uniform.splits[3],
            f32::MAX,
            "the last cascade has no far limit"
        );
        assert!(uniform.splits[0] < uniform.splits[1] && uniform.splits[1] < uniform.splits[2]);
        // Each crop magnifies the scene fit: a cascade never covers more of
        // the light frustum than the fit it was cut from.
        for cascade in &uniform.light_from_world {
            let crop = Mat4::from_cols_array(cascade) * scene_fit.inverse();
            assert!(crop.x_axis.x >= 1.0 - 1.0e-4 && crop.y_axis.y >= 1.0 - 1.0e-4);
        }
    }

    #[test]
    fn unfittable_atlas_falls_back_to_one_cell_of_the_scene_matrix() {
        // A singular light matrix cannot be cropped. The caster still fills
        // every atlas cell with it, so the receiver must sample one cell.
        let camera = test_camera();
        let degenerate = [0.0; 16];
        let uniform = ShadowCascadeUniform::fit(&camera, degenerate, 4, 1024);
        assert_eq!(uniform.cascade_count(), 1);
        assert_eq!(uniform.params[1], ATLAS_SCALE);
        assert!(
            uniform
                .light_from_world
                .iter()
                .all(|matrix| *matrix == degenerate)
        );
        assert!(uniform.splits.iter().all(|split| *split == f32::MAX));
    }

    #[test]
    fn split_distances_grow_monotonically_and_reach_the_far_plane() {
        let mut previous = 0.5;
        for index in 1..=4 {
            let split = split_distance(0.5, 80.0, index, 4);
            assert!(split > previous, "split {index} must lie beyond {previous}");
            previous = split;
        }
        assert!((previous - 80.0).abs() < 1.0e-3);
    }

    #[test]
    fn frustum_rays_interpolate_between_near_and_far_planes() {
        let projection = Mat4::perspective_rh(60.0_f32.to_radians(), 1.5, 0.1, 50.0);
        let rays = FrustumRays::new(projection.inverse()).expect("perspective frustum is valid");
        assert!((rays.near - 0.1).abs() < 1.0e-3);
        assert!((rays.far - 50.0).abs() < 1.0e-2);
        let half_height = 10.0 * 30.0_f32.to_radians().tan();
        for corner in rays.corners_at(10.0) {
            assert!((-corner.z - 10.0).abs() < 1.0e-3);
            assert!((corner.y.abs() - half_height).abs() < 1.0e-2);
            assert!((corner.x.abs() - half_height * 1.5).abs() < 1.0e-2);
        }
    }

    #[test]
    fn frustum_rays_accept_reversed_depth_projections() {
        let forward = Mat4::perspective_rh(60.0_f32.to_radians(), 1.0, 0.1, 50.0);
        let reversed = Mat4::from_cols(
            Vec4::new(1.0, 0.0, 0.0, 0.0),
            Vec4::new(0.0, 1.0, 0.0, 0.0),
            Vec4::new(0.0, 0.0, -1.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 1.0),
        ) * forward;
        let rays = FrustumRays::new(reversed.inverse()).expect("reversed frustum is valid");
        assert!(rays.near < rays.far);
        assert!((rays.near - 0.1).abs() < 1.0e-3);
    }
}
