use super::*;

pub(super) fn semantic_attribution(
    primitive: PreparedPrimitive,
    instance: Option<crate::scene::InstanceId>,
    material_pass: super::super::materials::MaterialPass,
) -> PreparedPrimitive {
    let (opaque, alpha_cutoff) = match material_pass {
        super::super::materials::MaterialPass::Opaque => (true, None),
        super::super::materials::MaterialPass::Blend => (false, None),
        super::super::materials::MaterialPass::Mask { cutoff } => (true, Some(cutoff)),
    };
    primitive
        .with_source_instance(instance)
        .with_semantic_material(opaque, alpha_cutoff)
}

pub(super) fn cpu_texture_sample_slot_count(material: &MaterialDesc) -> u64 {
    let transmissive = material.kind() == MaterialKind::PbrMetallicRoughness
        && material.transmission_factor() > 0.001;
    let mut count = [
        material.base_color_texture(),
        material.normal_texture(),
        material.metallic_roughness_texture(),
        material.occlusion_texture(),
        material.emissive_texture(),
        material.clearcoat_texture(),
        material.clearcoat_roughness_texture(),
        material.clearcoat_normal_texture(),
        material.sheen_color_texture(),
        material.sheen_roughness_texture(),
        material.anisotropy_texture(),
        material.iridescence_texture(),
        material.iridescence_thickness_texture(),
    ]
    .into_iter()
    .filter(Option::is_some)
    .count() as u64;
    count += u64::from(transmissive && material.transmission_texture().is_some());
    count += u64::from(
        transmissive && material.thickness_factor() > 0.0 && material.thickness_texture().is_some(),
    );
    count
}

pub(super) fn triangle_screen_edge_pixels(
    corners: [CpuBakeCorner; 3],
    camera: Option<&CameraProjection>,
    target: crate::render::RasterTarget,
) -> f32 {
    let Some(camera) = camera else {
        return 0.0;
    };
    let projected = corners.map(|corner| camera.project(corner.position));
    let [Some(a), Some(b), Some(c)] = projected else {
        return 0.0;
    };
    let width = target.width.max(1) as f32 * 0.5;
    let height = target.height.max(1) as f32 * 0.5;
    let edge = |from: crate::render::camera::ProjectedVertex,
                to: crate::render::camera::ProjectedVertex| {
        let dx = (to.ndc_x - from.ndc_x) * width;
        let dy = (to.ndc_y - from.ndc_y) * height;
        dx.hypot(dy)
    };
    edge(a, b).max(edge(b, c)).max(edge(c, a))
}

pub(super) fn triangle_uv_span(corners: [CpuBakeCorner; 3]) -> f32 {
    let edge = |a: [f32; 2], b: [f32; 2]| (b[0] - a[0]).hypot(b[1] - a[1]);
    edge(corners[0].uv, corners[1].uv)
        .max(edge(corners[1].uv, corners[2].uv))
        .max(edge(corners[2].uv, corners[0].uv))
}

pub(super) fn material_reflection(material: &MaterialDesc) -> Option<PreparedMaterialReflection> {
    if material.kind() != MaterialKind::PbrMetallicRoughness {
        return None;
    }
    PreparedMaterialReflection::new(material.metallic_factor(), material.roughness_factor())
}

pub(super) fn camera_facing_double_sided_normal(
    normal: Vec3,
    double_sided: bool,
    position: Vec3,
    camera_position: Option<Vec3>,
) -> Vec3 {
    let Some(camera_position) = camera_position.filter(|_| double_sided) else {
        return normal;
    };
    if normal.dot(camera_position - position) < 0.0 {
        -normal
    } else {
        normal
    }
}

pub(super) fn brighter_color(
    left: crate::material::Color,
    right: crate::material::Color,
) -> crate::material::Color {
    if relative_luminance(right) > relative_luminance(left) {
        right
    } else {
        left
    }
}

pub(super) fn relative_luminance(color: crate::material::Color) -> f32 {
    color
        .r
        .mul_add(0.2126, color.g.mul_add(0.7152, color.b * 0.0722))
}

pub(super) fn material_transmission(
    material: &MaterialDesc,
    transmission_texture: f32,
    thickness_texture: f32,
) -> Option<PreparedPhysicalTransmission> {
    if material.kind() != MaterialKind::PbrMetallicRoughness {
        return None;
    }
    PreparedPhysicalTransmission::new(PreparedPhysicalTransmissionInput {
        transmission: material.transmission_factor(),
        transmission_texture,
        ior: material.ior(),
        thickness: material.thickness_factor(),
        thickness_texture,
        attenuation_color: material.attenuation_color(),
        attenuation_distance: material.attenuation_distance(),
        roughness: material.roughness_factor(),
    })
}

pub(super) fn average_texture_sample(
    corners: &[CpuBakeCorner; 3],
    mut sample: impl FnMut([f32; 2]) -> f32,
) -> f32 {
    (sample(corners[0].uv) + sample(corners[1].uv) + sample(corners[2].uv)) / 3.0
}

pub(super) fn structural_vertex_tint(
    tint: Option<crate::material::Color>,
) -> Option<crate::material::Color> {
    tint.filter(|tint| tint.a < 1.0)
}

pub(in crate::render) fn draw_uniform_tint(
    tint: Option<crate::material::Color>,
) -> crate::material::Color {
    tint.filter(|tint| tint.a >= 1.0)
        .unwrap_or(crate::material::Color::WHITE)
}

pub(super) fn tinted_vertex_color(
    color: crate::material::Color,
    tint: Option<crate::material::Color>,
) -> crate::material::Color {
    tint.map_or(color, |tint| multiply_color(color, tint))
}
