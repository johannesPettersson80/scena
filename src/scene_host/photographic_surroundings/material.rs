use crate::{AssetFetcher, Color, NodeKey, Vec3};

use super::{SceneHostCore, linear_luminance};

#[derive(Debug, Clone, Copy)]
pub(super) struct SubjectMaterialAverage {
    pub(super) mean_color: Color,
    pub(super) mean_luminance: f32,
}

pub(super) fn subject_material_average<F: AssetFetcher>(
    host: &SceneHostCore<F>,
    subject_nodes: &[NodeKey],
) -> SubjectMaterialAverage {
    let inspection = host.scene.inspect_with_assets(&host.assets);
    let mut material_handles = Vec::new();
    for draw in inspection.draw_list() {
        if subject_nodes.contains(&draw.node()) && !material_handles.contains(&draw.material()) {
            material_handles.push(draw.material());
        }
    }
    let mut color = Vec3::ZERO;
    let mut count = 0.0;
    for handle in material_handles {
        let Some(material) = host.assets.material(handle) else {
            continue;
        };
        let base = material.base_color();
        color += Vec3::new(base.r, base.g, base.b);
        count += 1.0;
    }
    if count <= 0.0 {
        return SubjectMaterialAverage {
            mean_color: Color::GRAY,
            mean_luminance: linear_luminance(Color::GRAY),
        };
    }
    color /= count;
    let mean_color = Color::from_linear_rgb(color.x, color.y, color.z);
    SubjectMaterialAverage {
        mean_color,
        mean_luminance: linear_luminance(mean_color),
    }
}
