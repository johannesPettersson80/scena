use crate::diagnostics::{Backend, PrepareError};
use crate::scene::Vec3;

use super::{
    PreparedDirectionalLight, PreparedLights, PreparedPointLight, PreparedSpotLight,
    directional_illuminance_lux, punctual_intensity_candela,
};
use crate::render::{RasterTarget, camera::CameraProjection};

pub(in crate::render) const LIGHT_TILE_SIZE_PX: u32 = 32;
pub(in crate::render) const MAX_TILED_GPU_PUNCTUAL_LIGHTS: usize = 512;
pub(in crate::render) const MAX_TILED_GPU_LIGHTS_PER_TILE: usize = 32;
pub(in crate::render) const MAX_TILED_GPU_LIGHT_REFERENCES: usize = 65_536;

const KIND_DIRECTIONAL: f32 = 0.0;
const KIND_POINT: f32 = 1.0;
const KIND_SPOT: f32 = 2.0;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(in crate::render) struct PreparedTiledGpuLightRecord {
    pub(in crate::render) position_intensity: [f32; 4],
    pub(in crate::render) direction_kind: [f32; 4],
    pub(in crate::render) color_range: [f32; 4],
    pub(in crate::render) cone: [f32; 4],
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::render) struct PreparedTiledGpuLightTile {
    pub(in crate::render) offset_count: [u32; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::render) struct TiledLightAssignment {
    pub(in crate::render) records: Vec<PreparedTiledGpuLightRecord>,
    pub(in crate::render) light_tile_indices: Vec<u32>,
    pub(in crate::render) tiles: Vec<PreparedTiledGpuLightTile>,
}

impl Default for TiledLightAssignment {
    fn default() -> Self {
        Self::inactive()
    }
}

impl TiledLightAssignment {
    pub(in crate::render) fn inactive() -> Self {
        Self {
            records: vec![PreparedTiledGpuLightRecord::default()],
            light_tile_indices: vec![0],
            tiles: vec![PreparedTiledGpuLightTile {
                offset_count: [1, 1, LIGHT_TILE_SIZE_PX, 0],
            }],
        }
    }

    pub(in crate::render) fn is_active(&self) -> bool {
        self.tiles
            .first()
            .is_some_and(|tile| tile.offset_count[3] != 0)
    }

    pub(in crate::render) fn record_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.records.len() * 64);
        for record in &self.records {
            encode_vec4_f32(&mut bytes, record.position_intensity);
            encode_vec4_f32(&mut bytes, record.direction_kind);
            encode_vec4_f32(&mut bytes, record.color_range);
            encode_vec4_f32(&mut bytes, record.cone);
        }
        bytes
    }

    pub(in crate::render) fn tile_index_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.light_tile_indices.len() * 4);
        for value in &self.light_tile_indices {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
        bytes
    }

    pub(in crate::render) fn tile_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.tiles.len() * 16);
        for tile in &self.tiles {
            for value in tile.offset_count {
                bytes.extend_from_slice(&value.to_ne_bytes());
            }
        }
        bytes
    }
}

pub(in crate::render) fn assignment_required(lights: &PreparedLights) -> bool {
    lights.directional.len() + lights.point.len() + lights.spot.len()
        > super::MAX_GPU_LIGHTS_PER_TYPE
}

pub(in crate::render) fn collect_tiled_light_assignment(
    lights: &PreparedLights,
    target: RasterTarget,
    camera: Option<&CameraProjection>,
) -> Result<TiledLightAssignment, PrepareError> {
    if !assignment_required(lights) {
        return Ok(TiledLightAssignment::inactive());
    }
    let record_count = lights.directional.len() + lights.point.len() + lights.spot.len();
    if record_count > MAX_TILED_GPU_PUNCTUAL_LIGHTS {
        return Err(tiled_light_capacity_error(
            target.backend,
            format!(
                "prepared lighting has {record_count} directional/point/spot entries, exceeding the tiled GPU light-assignment cap {MAX_TILED_GPU_PUNCTUAL_LIGHTS}"
            ),
        ));
    }

    let columns = target.width.div_ceil(LIGHT_TILE_SIZE_PX).max(1);
    let rows = target.height.div_ceil(LIGHT_TILE_SIZE_PX).max(1);
    let tile_count = columns.saturating_mul(rows) as usize;
    let mut records = Vec::with_capacity(record_count);
    let mut tile_lists = vec![Vec::<u32>::new(); tile_count];

    for light in &lights.directional {
        let index = push_directional_record(&mut records, light);
        for list in &mut tile_lists {
            list.push(index);
        }
    }
    for light in &lights.point {
        let index = push_point_record(&mut records, light);
        assign_lights_to_tiles(
            index,
            light.position,
            light.range,
            target,
            camera,
            &mut tile_lists,
        );
    }
    for light in &lights.spot {
        let index = push_spot_record(&mut records, light);
        assign_lights_to_tiles(
            index,
            light.position,
            light.range,
            target,
            camera,
            &mut tile_lists,
        );
    }

    let mut light_tile_indices = Vec::new();
    let mut tiles = Vec::with_capacity(tile_count + 1);
    tiles.push(PreparedTiledGpuLightTile {
        offset_count: [columns, rows, LIGHT_TILE_SIZE_PX, 1],
    });
    for list in tile_lists {
        if list.len() > MAX_TILED_GPU_LIGHTS_PER_TILE {
            return Err(tiled_light_capacity_error(
                target.backend,
                format!(
                    "a screen tile contains {} lights, exceeding the tiled GPU lights-per-tile cap {MAX_TILED_GPU_LIGHTS_PER_TILE}",
                    list.len()
                ),
            ));
        }
        let offset = light_tile_indices.len();
        light_tile_indices.extend(list);
        if light_tile_indices.len() > MAX_TILED_GPU_LIGHT_REFERENCES {
            return Err(tiled_light_capacity_error(
                target.backend,
                format!(
                    "tiled GPU light assignment projects {} tile references, exceeding the cap {MAX_TILED_GPU_LIGHT_REFERENCES}",
                    light_tile_indices.len()
                ),
            ));
        }
        tiles.push(PreparedTiledGpuLightTile {
            offset_count: [
                offset as u32,
                (light_tile_indices.len() - offset) as u32,
                0,
                0,
            ],
        });
    }

    Ok(TiledLightAssignment {
        records,
        light_tile_indices: light_tile_indices.into_iter().chain([0]).collect(),
        tiles,
    })
}

fn push_directional_record(
    records: &mut Vec<PreparedTiledGpuLightRecord>,
    light: &PreparedDirectionalLight,
) -> u32 {
    let index = records.len() as u32;
    records.push(PreparedTiledGpuLightRecord {
        position_intensity: [
            light.direction.x,
            light.direction.y,
            light.direction.z,
            directional_illuminance_lux(light.illuminance_lux),
        ],
        direction_kind: [0.0, 0.0, -1.0, KIND_DIRECTIONAL],
        color_range: [light.color.r, light.color.g, light.color.b, 0.0],
        cone: [0.0, 0.0, 0.0, 0.0],
    });
    index
}

fn push_point_record(
    records: &mut Vec<PreparedTiledGpuLightRecord>,
    light: &PreparedPointLight,
) -> u32 {
    let index = records.len() as u32;
    records.push(PreparedTiledGpuLightRecord {
        position_intensity: [
            light.position.x,
            light.position.y,
            light.position.z,
            punctual_intensity_candela(light.intensity_candela),
        ],
        direction_kind: [0.0, 0.0, -1.0, KIND_POINT],
        color_range: [
            light.color.r,
            light.color.g,
            light.color.b,
            light.range.unwrap_or(0.0).max(0.0),
        ],
        cone: [0.0, 0.0, 0.0, 0.0],
    });
    index
}

fn push_spot_record(
    records: &mut Vec<PreparedTiledGpuLightRecord>,
    light: &PreparedSpotLight,
) -> u32 {
    let index = records.len() as u32;
    records.push(PreparedTiledGpuLightRecord {
        position_intensity: [
            light.position.x,
            light.position.y,
            light.position.z,
            punctual_intensity_candela(light.intensity_candela),
        ],
        direction_kind: [
            light.direction.x,
            light.direction.y,
            light.direction.z,
            KIND_SPOT,
        ],
        color_range: [
            light.color.r,
            light.color.g,
            light.color.b,
            light.range.unwrap_or(0.0).max(0.0),
        ],
        cone: [light.inner_cone_cos, light.outer_cone_cos, 0.0, 0.0],
    });
    index
}

fn assign_lights_to_tiles(
    record_index: u32,
    position: Vec3,
    range: Option<f32>,
    target: RasterTarget,
    camera: Option<&CameraProjection>,
    tile_lists: &mut [Vec<u32>],
) {
    let columns = target.width.div_ceil(LIGHT_TILE_SIZE_PX).max(1);
    let rows = target.height.div_ceil(LIGHT_TILE_SIZE_PX).max(1);
    let Some(camera) = camera else {
        assign_all_tiles(record_index, tile_lists);
        return;
    };
    let Some(projected) = camera.project(position) else {
        assign_all_tiles(record_index, tile_lists);
        return;
    };
    let screen_x = (projected.ndc_x * 0.5 + 0.5) * target.width.max(1) as f32;
    let screen_y = (1.0 - (projected.ndc_y * 0.5 + 0.5)) * target.height.max(1) as f32;
    let radius_px = range
        .filter(|range| range.is_finite() && *range > 0.0)
        .and_then(|range| {
            camera
                .world_units_per_pixel_at(position)
                .map(|units| range / units.max(0.0001))
        })
        .filter(|radius| radius.is_finite() && *radius > 0.0);
    let Some(radius_px) = radius_px else {
        assign_all_tiles(record_index, tile_lists);
        return;
    };
    let min_x = ((screen_x - radius_px) / LIGHT_TILE_SIZE_PX as f32)
        .floor()
        .max(0.0) as u32;
    let max_x = ((screen_x + radius_px) / LIGHT_TILE_SIZE_PX as f32)
        .ceil()
        .min(columns.saturating_sub(1) as f32) as u32;
    let min_y = ((screen_y - radius_px) / LIGHT_TILE_SIZE_PX as f32)
        .floor()
        .max(0.0) as u32;
    let max_y = ((screen_y + radius_px) / LIGHT_TILE_SIZE_PX as f32)
        .ceil()
        .min(rows.saturating_sub(1) as f32) as u32;
    for tile_y in min_y..=max_y {
        for tile_x in min_x..=max_x {
            let tile_index = (tile_y * columns + tile_x) as usize;
            if let Some(list) = tile_lists.get_mut(tile_index) {
                list.push(record_index);
            }
        }
    }
}

fn assign_all_tiles(record_index: u32, tile_lists: &mut [Vec<u32>]) {
    for list in tile_lists {
        list.push(record_index);
    }
}

fn tiled_light_capacity_error(backend: Backend, help: String) -> PrepareError {
    PrepareError::BackendCapabilityMismatch {
        feature: "tiled_light_assignment_capacity",
        backend,
        help,
    }
}

fn encode_vec4_f32(bytes: &mut Vec<u8>, value: [f32; 4]) {
    for component in value {
        bytes.extend_from_slice(&component.to_ne_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Backend;
    use crate::material::Color;
    use crate::scene::{PerspectiveCamera, PointLight, Scene, Transform};

    #[test]
    fn tiled_light_assignment_activates_only_above_fixed_uniform_lane() {
        let mut scene = Scene::new();
        for index in 0..=super::super::MAX_GPU_LIGHTS_PER_TYPE {
            scene
                .point_light(
                    PointLight::default()
                        .with_color(Color::WHITE)
                        .with_intensity_candela(8.0)
                        .with_range(1.0),
                )
                .transform(Transform::at(Vec3::new(index as f32 * 0.1, 0.0, 1.0)))
                .add()
                .expect("point light inserts");
        }
        let camera = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::default(),
                Transform::at(Vec3::new(0.0, 0.0, 3.0)),
            )
            .expect("camera inserts");
        scene.set_active_camera(camera).expect("camera activates");
        let target = RasterTarget {
            width: 160,
            height: 96,
            backend: Backend::HeadlessGpu,
        };
        let camera = scene
            .active_camera()
            .and_then(|camera| CameraProjection::from_scene(&scene, camera, target).ok());
        let lights = PreparedLights::from_scene(&scene, Vec3::ZERO);

        let assignment = collect_tiled_light_assignment(&lights, target, camera.as_ref())
            .expect("tiled assignment builds");

        assert!(assignment.is_active());
        assert_eq!(assignment.records.len(), 17);
        assert_eq!(
            assignment.tiles[0].offset_count,
            [5, 3, LIGHT_TILE_SIZE_PX, 1]
        );
        assert!(
            assignment
                .tiles
                .iter()
                .skip(1)
                .any(|tile| tile.offset_count[1] > 0),
            "at least one screen tile should receive light indices"
        );
    }
}
