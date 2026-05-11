use crate::diagnostics::{Capabilities, CapabilityStatus, RenderError};
use crate::scene::{Camera, CameraKey, Quat, Scene, Transform, Vec3};

use super::RasterTarget;

#[derive(Debug, Clone)]
pub(super) struct CameraProjection {
    camera: Camera,
    world_from_camera: Transform,
    target: RasterTarget,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ProjectedVertex {
    pub(super) ndc_x: f32,
    pub(super) ndc_y: f32,
    pub(super) depth: f32,
}

impl CameraProjection {
    pub(super) fn from_scene(
        scene: &Scene,
        camera: CameraKey,
        target: RasterTarget,
    ) -> Result<Self, RenderError> {
        let camera_desc = scene
            .camera(camera)
            .cloned()
            .ok_or(RenderError::CameraNotFound(camera))?;
        let camera_node = scene
            .camera_node(camera)
            .ok_or(RenderError::CameraNotFound(camera))?;
        let world_from_camera = scene
            .world_transform(camera_node)
            .ok_or(RenderError::CameraNotFound(camera))?;
        Ok(Self {
            camera: camera_desc,
            world_from_camera,
            target,
        })
    }

    pub(super) fn project(&self, world_position: Vec3) -> Option<ProjectedVertex> {
        let view = self.world_to_view(world_position)?;
        match self.camera {
            Camera::Perspective(camera) => {
                let depth = -view.z;
                if !depth.is_finite() || depth < camera.near || depth > camera.far {
                    return None;
                }
                let aspect = positive_or(
                    camera.aspect,
                    self.target.width.max(1) as f32 / self.target.height.max(1) as f32,
                );
                let half_fov = camera.vertical_fov.radians() * 0.5;
                let focal = half_fov.tan().recip();
                if !focal.is_finite() {
                    return None;
                }
                Some(ProjectedVertex {
                    ndc_x: (view.x * focal / aspect) / depth,
                    ndc_y: (view.y * focal) / depth,
                    depth,
                })
            }
            Camera::Orthographic(camera) => {
                let depth = -view.z;
                if !depth.is_finite() || depth < camera.near || depth > camera.far {
                    return None;
                }
                let width = camera.right - camera.left;
                let height = camera.top - camera.bottom;
                if width.abs() <= f32::EPSILON || height.abs() <= f32::EPSILON {
                    return None;
                }
                Some(ProjectedVertex {
                    ndc_x: (view.x - camera.left) / width * 2.0 - 1.0,
                    ndc_y: (view.y - camera.bottom) / height * 2.0 - 1.0,
                    depth,
                })
            }
        }
    }

    pub(super) fn camera_depth(&self, world_position: Vec3) -> Option<f32> {
        self.world_to_view(world_position).map(|view| -view.z)
    }

    pub(super) const fn camera_position(&self) -> Vec3 {
        self.world_from_camera.translation
    }

    pub(super) const fn near_far(&self) -> [f32; 2] {
        match self.camera {
            Camera::Perspective(camera) => [camera.near, camera.far],
            Camera::Orthographic(camera) => [camera.near, camera.far],
        }
    }

    pub(super) fn clip_from_world_matrix(&self) -> Option<[f32; 16]> {
        let view = self.view_from_world_matrix()?;
        let projection = self.clip_from_view_matrix()?;
        debug_assert!(self.world_from_view_matrix().is_some());
        debug_assert!(self.view_from_clip_matrix().is_some());
        Some(multiply_matrices(projection, view))
    }

    pub(super) fn view_from_world_matrix(&self) -> Option<[f32; 16]> {
        let (view_rows, view_translation) = self.view_rows_and_translation()?;
        Some(row_major_to_column_major([
            [
                view_rows[0].x,
                view_rows[0].y,
                view_rows[0].z,
                view_translation.x,
            ],
            [
                view_rows[1].x,
                view_rows[1].y,
                view_rows[1].z,
                view_translation.y,
            ],
            [
                view_rows[2].x,
                view_rows[2].y,
                view_rows[2].z,
                view_translation.z,
            ],
            [0.0, 0.0, 0.0, 1.0],
        ]))
    }

    pub(super) fn world_from_view_matrix(&self) -> Option<[f32; 16]> {
        if !is_finite_nonzero_scale(self.world_from_camera.scale) {
            return None;
        }
        let x = scale_vec3(
            rotate_vec3(self.world_from_camera.rotation, Vec3::new(1.0, 0.0, 0.0)),
            self.world_from_camera.scale.x,
        );
        let y = scale_vec3(
            rotate_vec3(self.world_from_camera.rotation, Vec3::new(0.0, 1.0, 0.0)),
            self.world_from_camera.scale.y,
        );
        let z = scale_vec3(
            rotate_vec3(self.world_from_camera.rotation, Vec3::new(0.0, 0.0, 1.0)),
            self.world_from_camera.scale.z,
        );
        let t = self.world_from_camera.translation;
        Some([
            x.x, x.y, x.z, 0.0, y.x, y.y, y.z, 0.0, z.x, z.y, z.z, 0.0, t.x, t.y, t.z, 1.0,
        ])
    }

    pub(super) fn clip_from_view_matrix(&self) -> Option<[f32; 16]> {
        let rows = match self.camera {
            Camera::Perspective(camera) => {
                let aspect = positive_or(
                    camera.aspect,
                    self.target.width.max(1) as f32 / self.target.height.max(1) as f32,
                );
                let half_fov = camera.vertical_fov.radians() * 0.5;
                let focal = half_fov.tan().recip();
                if !focal.is_finite() || camera.far <= camera.near {
                    return None;
                }
                let scale_x = focal / aspect;
                let scale_y = focal;
                let (depth_scale, depth_bias) =
                    perspective_depth_terms(camera.near, camera.far, self.uses_reversed_z());
                [
                    [scale_x, 0.0, 0.0, 0.0],
                    [0.0, scale_y, 0.0, 0.0],
                    [0.0, 0.0, depth_scale, depth_bias],
                    [0.0, 0.0, -1.0, 0.0],
                ]
            }
            Camera::Orthographic(camera) => {
                let width = camera.right - camera.left;
                let height = camera.top - camera.bottom;
                let depth = camera.far - camera.near;
                if width.abs() <= f32::EPSILON
                    || height.abs() <= f32::EPSILON
                    || depth.abs() <= f32::EPSILON
                {
                    return None;
                }
                let scale_x = 2.0 / width;
                let scale_y = 2.0 / height;
                let (scale_z, bias_z) =
                    orthographic_depth_terms(camera.near, camera.far, self.uses_reversed_z());
                [
                    [scale_x, 0.0, 0.0, -(camera.right + camera.left) / width],
                    [0.0, scale_y, 0.0, -(camera.top + camera.bottom) / height],
                    [0.0, 0.0, scale_z, bias_z],
                    [0.0, 0.0, 0.0, 1.0],
                ]
            }
        };
        Some(row_major_to_column_major(rows))
    }

    pub(super) fn view_from_clip_matrix(&self) -> Option<[f32; 16]> {
        let rows = match self.camera {
            Camera::Perspective(camera) => {
                let aspect = positive_or(
                    camera.aspect,
                    self.target.width.max(1) as f32 / self.target.height.max(1) as f32,
                );
                let half_fov = camera.vertical_fov.radians() * 0.5;
                let focal = half_fov.tan().recip();
                if !focal.is_finite() || camera.far <= camera.near {
                    return None;
                }
                let scale_x = focal / aspect;
                let scale_y = focal;
                let (depth_scale, depth_bias) =
                    perspective_depth_terms(camera.near, camera.far, self.uses_reversed_z());
                if depth_bias.abs() <= f32::EPSILON {
                    return None;
                }
                [
                    [scale_x.recip(), 0.0, 0.0, 0.0],
                    [0.0, scale_y.recip(), 0.0, 0.0],
                    [0.0, 0.0, 0.0, -1.0],
                    [0.0, 0.0, depth_bias.recip(), depth_scale / depth_bias],
                ]
            }
            Camera::Orthographic(camera) => {
                let width = camera.right - camera.left;
                let height = camera.top - camera.bottom;
                let depth = camera.far - camera.near;
                if width.abs() <= f32::EPSILON
                    || height.abs() <= f32::EPSILON
                    || depth.abs() <= f32::EPSILON
                {
                    return None;
                }
                let scale_x = 2.0 / width;
                let scale_y = 2.0 / height;
                let (scale_z, bias_z) =
                    orthographic_depth_terms(camera.near, camera.far, self.uses_reversed_z());
                let bias_x = -(camera.right + camera.left) / width;
                let bias_y = -(camera.top + camera.bottom) / height;
                [
                    [scale_x.recip(), 0.0, 0.0, -bias_x / scale_x],
                    [0.0, scale_y.recip(), 0.0, -bias_y / scale_y],
                    [0.0, 0.0, scale_z.recip(), -bias_z / scale_z],
                    [0.0, 0.0, 0.0, 1.0],
                ]
            }
        };
        Some(row_major_to_column_major(rows))
    }

    fn world_to_view(&self, world_position: Vec3) -> Option<Vec3> {
        if !is_finite_nonzero_scale(self.world_from_camera.scale) {
            return None;
        }
        let translated = subtract_vec3(world_position, self.world_from_camera.translation);
        let rotated = rotate_vec3(inverse_quat(self.world_from_camera.rotation)?, translated);
        Some(Vec3::new(
            rotated.x / self.world_from_camera.scale.x,
            rotated.y / self.world_from_camera.scale.y,
            rotated.z / self.world_from_camera.scale.z,
        ))
    }

    fn uses_reversed_z(&self) -> bool {
        Capabilities::for_backend(self.target.backend).reversed_z_depth
            == CapabilityStatus::Supported
    }

    fn view_rows_and_translation(&self) -> Option<([Vec3; 3], Vec3)> {
        let scale = self.world_from_camera.scale;
        if !is_finite_nonzero_scale(scale) {
            return None;
        }
        let right = scale_vec3(
            rotate_vec3(self.world_from_camera.rotation, Vec3::new(1.0, 0.0, 0.0)),
            scale.x.recip(),
        );
        let up = scale_vec3(
            rotate_vec3(self.world_from_camera.rotation, Vec3::new(0.0, 1.0, 0.0)),
            scale.y.recip(),
        );
        let forward = scale_vec3(
            rotate_vec3(self.world_from_camera.rotation, Vec3::new(0.0, 0.0, 1.0)),
            scale.z.recip(),
        );
        let camera_position = self.world_from_camera.translation;
        let translation = Vec3::new(
            -dot(right, camera_position),
            -dot(up, camera_position),
            -dot(forward, camera_position),
        );
        Some(([right, up, forward], translation))
    }
}

fn inverse_quat(rotation: Quat) -> Option<Quat> {
    let length_squared = rotation.x * rotation.x
        + rotation.y * rotation.y
        + rotation.z * rotation.z
        + rotation.w * rotation.w;
    if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        return None;
    }
    let inverse_length_squared = length_squared.recip();
    Some(Quat::from_xyzw(-rotation.x * inverse_length_squared, -rotation.y * inverse_length_squared, -rotation.z * inverse_length_squared, rotation.w * inverse_length_squared))
}

fn rotate_vec3(rotation: Quat, vector: Vec3) -> Vec3 {
    let length_squared = rotation.x * rotation.x
        + rotation.y * rotation.y
        + rotation.z * rotation.z
        + rotation.w * rotation.w;
    if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        return vector;
    }
    let inverse_length = length_squared.sqrt().recip();
    let qx = rotation.x * inverse_length;
    let qy = rotation.y * inverse_length;
    let qz = rotation.z * inverse_length;
    let qw = rotation.w * inverse_length;
    let tx = 2.0 * (qy * vector.z - qz * vector.y);
    let ty = 2.0 * (qz * vector.x - qx * vector.z);
    let tz = 2.0 * (qx * vector.y - qy * vector.x);
    Vec3::new(
        vector.x + qw * tx + (qy * tz - qz * ty),
        vector.y + qw * ty + (qz * tx - qx * tz),
        vector.z + qw * tz + (qx * ty - qy * tx),
    )
}

fn is_finite_nonzero_scale(scale: Vec3) -> bool {
    scale.x.is_finite()
        && scale.y.is_finite()
        && scale.z.is_finite()
        && scale.x.abs() > f32::EPSILON
        && scale.y.abs() > f32::EPSILON
        && scale.z.abs() > f32::EPSILON
}

const fn subtract_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

const fn scale_vec3(value: Vec3, scale: f32) -> Vec3 {
    Vec3::new(value.x * scale, value.y * scale, value.z * scale)
}

fn dot(left: Vec3, right: Vec3) -> f32 {
    left.x * right.x + left.y * right.y + left.z * right.z
}

fn perspective_depth_terms(near: f32, far: f32, reversed_z: bool) -> (f32, f32) {
    let depth = far - near;
    if reversed_z {
        (near / depth, far * near / depth)
    } else {
        (-far / depth, -far * near / depth)
    }
}

fn orthographic_depth_terms(near: f32, far: f32, reversed_z: bool) -> (f32, f32) {
    let depth = far - near;
    if reversed_z {
        (1.0 / depth, far / depth)
    } else {
        (-1.0 / depth, -near / depth)
    }
}

fn multiply_matrices(left: [f32; 16], right: [f32; 16]) -> [f32; 16] {
    let mut output = [0.0; 16];
    for column in 0..4 {
        for row in 0..4 {
            output[column * 4 + row] = (0..4)
                .map(|index| left[index * 4 + row] * right[column * 4 + index])
                .sum();
        }
    }
    output
}

fn row_major_to_column_major(rows: [[f32; 4]; 4]) -> [f32; 16] {
    [
        rows[0][0], rows[1][0], rows[2][0], rows[3][0], rows[0][1], rows[1][1], rows[2][1],
        rows[3][1], rows[0][2], rows[1][2], rows[2][2], rows[3][2], rows[0][3], rows[1][3],
        rows[2][3], rows[3][3],
    ]
}

fn positive_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Backend;
    use crate::scene::{PerspectiveCamera, Scene};

    #[test]
    fn clip_from_world_matrix_matches_perspective_project_result() {
        let mut scene = Scene::new();
        let camera = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::default(),
                Transform::at(Vec3::new(2.0, 0.0, 3.0)),
            )
            .expect("camera inserts");
        let target = RasterTarget {
            width: 96,
            height: 96,
            backend: Backend::Headless,
        };
        let projection =
            CameraProjection::from_scene(&scene, camera, target).expect("projection builds");

        let projected = projection
            .project(Vec3::new(2.0, 0.0, 0.0))
            .expect("point projects");
        let clip = multiply_matrix_point(
            projection
                .clip_from_world_matrix()
                .expect("matrix is finite"),
            Vec3::new(2.0, 0.0, 0.0),
        );

        assert_near(clip[0] / clip[3], projected.ndc_x);
        assert_near(clip[1] / clip[3], projected.ndc_y);
    }

    #[test]
    fn camera_view_and_projection_inverse_matrices_round_trip_points() {
        let mut scene = Scene::new();
        let camera = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::default(),
                Transform::at(Vec3::new(2.0, 1.0, 4.0)).rotate_y_deg(20.0),
            )
            .expect("camera inserts");
        let target = RasterTarget {
            width: 128,
            height: 64,
            backend: Backend::Headless,
        };
        let projection =
            CameraProjection::from_scene(&scene, camera, target).expect("projection builds");

        let world = Vec3::new(2.25, 1.5, 0.5);
        let view = multiply_matrix_point(
            projection
                .view_from_world_matrix()
                .expect("view matrix is finite"),
            world,
        );
        let world_round_trip = multiply_matrix_vec4(
            projection
                .world_from_view_matrix()
                .expect("inverse view matrix is finite"),
            view,
        );
        assert_near(world_round_trip[0] / world_round_trip[3], world.x);
        assert_near(world_round_trip[1] / world_round_trip[3], world.y);
        assert_near(world_round_trip[2] / world_round_trip[3], world.z);

        let clip = multiply_matrix_vec4(
            projection
                .clip_from_view_matrix()
                .expect("projection matrix is finite"),
            view,
        );
        let view_round_trip = multiply_matrix_vec4(
            projection
                .view_from_clip_matrix()
                .expect("inverse projection matrix is finite"),
            clip,
        );
        assert_near(view_round_trip[0] / view_round_trip[3], view[0] / view[3]);
        assert_near(view_round_trip[1] / view_round_trip[3], view[1] / view[3]);
        assert_near(view_round_trip[2] / view_round_trip[3], view[2] / view[3]);
    }

    #[test]
    fn webgpu_projection_uses_reversed_z_depth_mapping() {
        let mut scene = Scene::new();
        let camera_desc = PerspectiveCamera::default();
        let near = camera_desc.near;
        let far = camera_desc.far;
        let camera = scene
            .add_perspective_camera(scene.root(), camera_desc, Transform::default())
            .expect("camera inserts");
        let projection = CameraProjection::from_scene(
            &scene,
            camera,
            RasterTarget {
                width: 64,
                height: 64,
                backend: Backend::WebGpu,
            },
        )
        .expect("projection builds");
        let matrix = projection
            .clip_from_view_matrix()
            .expect("projection matrix is finite");
        let near_clip = multiply_matrix_vec4(matrix, [0.0, 0.0, -near, 1.0]);
        let far_clip = multiply_matrix_vec4(matrix, [0.0, 0.0, -far, 1.0]);
        let near_depth = near_clip[2] / near_clip[3];
        let far_depth = far_clip[2] / far_clip[3];

        assert!(
            near_depth > far_depth,
            "reversed-Z maps near depth above far depth"
        );
        assert_near(near_depth, 1.0);
        assert_near(far_depth, 0.0);
    }

    fn multiply_matrix_point(matrix: [f32; 16], point: Vec3) -> [f32; 4] {
        let input = [point.x, point.y, point.z, 1.0];
        multiply_matrix_vec4(matrix, input)
    }

    fn multiply_matrix_vec4(matrix: [f32; 16], input: [f32; 4]) -> [f32; 4] {
        let mut output = [0.0; 4];
        for row in 0..4 {
            for column in 0..4 {
                output[row] += matrix[column * 4 + row] * input[column];
            }
        }
        output
    }

    fn assert_near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 1.0e-5,
            "expected {actual} to be near {expected}"
        );
    }
}
