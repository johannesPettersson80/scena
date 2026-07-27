use super::*;

pub(super) fn material_values_are_physical(material: &MaterialDesc) -> bool {
    let color_is_finite = |color: crate::Color| {
        [color.r, color.g, color.b, color.a]
            .into_iter()
            .all(f32::is_finite)
    };
    color_is_finite(material.base_color())
        && color_is_finite(material.emissive())
        && material.emissive_strength().is_finite()
        && material.emissive_strength() >= 0.0
        && (0.0..=1.0).contains(&material.metallic_factor())
        && (0.0..=1.0).contains(&material.roughness_factor())
        && material.normal_scale().is_finite()
        && material.occlusion_strength().is_finite()
        && (0.0..=1.0).contains(&material.occlusion_strength())
        && material.photographic_micro_surface().is_none_or(|surface| {
            surface.strength().is_finite()
                && surface.strength() >= 0.0
                && surface.scale_m().is_finite()
                && surface.scale_m() > 0.0
        })
}

pub(super) fn transform_bounds(bounds: Aabb, transform: Transform) -> Aabb {
    let mut minimum = Vec3::splat(f32::INFINITY);
    let mut maximum = Vec3::splat(f32::NEG_INFINITY);
    for x in [bounds.min.x, bounds.max.x] {
        for y in [bounds.min.y, bounds.max.y] {
            for z in [bounds.min.z, bounds.max.z] {
                let point = transform.translation
                    + transform.rotation * (Vec3::new(x, y, z) * transform.scale);
                minimum = minimum.min(point);
                maximum = maximum.max(point);
            }
        }
    }
    Aabb::new(minimum, maximum)
}

pub(super) fn append_component_layout_issues(
    report: &mut PhotographicSurfaceReportV1,
    components: &[(u64, Aabb)],
) {
    let Some((_, primary)) = components.iter().max_by(|left, right| {
        left.1
            .bounding_sphere_radius()
            .total_cmp(&right.1.bounding_sphere_radius())
    }) else {
        return;
    };
    let primary_radius = primary.bounding_sphere_radius().max(1.0e-6);
    for (node, bounds) in components {
        if bounds == primary {
            continue;
        }
        let radius = bounds.bounding_sphere_radius();
        let separation = bounds.center().distance(primary.center());
        if separation > primary_radius * 4.0 && radius < primary_radius * 0.25 {
            report.issues.push(asset_issue(
                PhotographicAssetIssueClassV1::AppearanceChangeRequired,
                "outlier_subject_component",
                Some(*node),
                "small subject component is positioned far outside the primary component",
                Some("confirm the component transform and scene units"),
            ));
        } else if !aabbs_overlap(*primary, *bounds) {
            report.issues.push(asset_issue(
                PhotographicAssetIssueClassV1::AppearanceChangeRequired,
                "detached_subject_component",
                Some(*node),
                "subject component is spatially detached from the primary component",
                Some("confirm that the detached component belongs to the requested photograph"),
            ));
        }
    }
}

pub(super) fn aabbs_overlap(left: Aabb, right: Aabb) -> bool {
    left.min.x <= right.max.x
        && left.max.x >= right.min.x
        && left.min.y <= right.max.y
        && left.max.y >= right.min.y
        && left.min.z <= right.max.z
        && left.max.z >= right.min.z
}
