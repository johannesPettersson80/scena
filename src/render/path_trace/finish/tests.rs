use glam::{Affine3A, Quat};

use super::*;

fn placed(finish: SurfaceFinish) -> TraceFinish {
    TraceFinish::new(finish, Affine3A::IDENTITY)
}

fn surface(base: Vec3, metallic: f32, roughness: f32) -> Surface {
    Surface {
        base,
        metallic,
        alpha: roughness * roughness,
        f0: Vec3::splat(0.04).lerp(base, metallic),
        clearcoat: 0.5,
        clearcoat_alpha: 0.01,
        sheen: Vec3::ZERO,
        sheen_alpha: 0.25,
        transmission: 0.0,
        ior: 1.5,
        thin_walled: false,
        emission: Vec3::ZERO,
        normal_map: None,
        unlit: false,
    }
}

/// Points spread over the plane `y = 0`, a few centimetres across.
fn plane_points() -> impl Iterator<Item = Vec3> {
    (0..3000).map(|index| {
        let t = index as f32;
        Vec3::new(
            (t * 0.618_034).fract() * 0.08,
            0.0,
            (t * 0.414_214).fract() * 0.08,
        )
    })
}

/// Root-mean-square slope along the plane `y = 0`, and the mean slope
/// variance the finish reports as too fine to see.
fn slope_statistics(finish: &TraceFinish, footprint: f32) -> (f32, f32) {
    let mut rng = Rng::new(1, 1);
    let (mut squared, mut unresolved, mut count) = (0.0_f64, 0.0_f64, 0.0_f64);
    for point in plane_points() {
        let relief = finish.relief(point, Vec3::Y, footprint, &mut rng);
        let along = relief.slope - Vec3::Y * relief.slope.y;
        squared += f64::from(along.length_squared());
        unresolved += f64::from(relief.unresolved);
        count += 1.0;
    }
    ((squared / count).sqrt() as f32, (unresolved / count) as f32)
}

#[test]
fn resolved_relief_has_the_slope_its_recipe_states() {
    let finish = placed(SurfaceFinish::new(Kind::CastMetal));
    let stated = (0.24_f32.powi(2) + 0.04_f32.powi(2)).sqrt();
    let (slope, unresolved) = slope_statistics(&finish, 1.0e-7);
    assert!(
        (slope / stated - 1.0).abs() < 0.2,
        "slope {slope} for {stated}"
    );
    assert!(unresolved < 1.0e-6);
    let (doubled, _) = slope_statistics(
        &placed(SurfaceFinish::new(Kind::CastMetal).with_relief(2.0)),
        1.0e-7,
    );
    assert!((doubled / slope - 2.0).abs() < 1.0e-3);
}

#[test]
fn relief_too_fine_to_see_widens_the_highlight_instead() {
    let finish = placed(SurfaceFinish::new(Kind::CastMetal));
    let stated = 0.24_f32.powi(2) + 0.04_f32.powi(2);
    let (slope, unresolved) = slope_statistics(&finish, 0.5);
    assert!(slope < 1.0e-3, "nothing of the relief is drawn: {slope}");
    assert!((unresolved / stated - 1.0).abs() < 0.02);
    // In between, what is not drawn is accounted for as roughness.
    let (slope, unresolved) = slope_statistics(&finish, 0.000_6);
    assert!(slope > 0.02 && unresolved > 0.01);
    let total = slope * slope + unresolved;
    assert!((total / stated - 1.0).abs() < 0.35, "total {total}");
    let mut rough = surface(Vec3::splat(0.5), 1.0, 0.3);
    let mut rng = Rng::new(1, 1);
    let relief = finish.relief(Vec3::new(0.01, 0.0, 0.02), Vec3::Y, 0.5, &mut rng);
    finish.apply(&mut rough, &relief, 0.0, false);
    assert!(
        rough.alpha > 0.2,
        "alpha {} holds the hidden grain",
        rough.alpha
    );
}

#[test]
fn the_pattern_belongs_to_the_object_and_grows_with_its_scale() {
    let cast = SurfaceFinish::new(Kind::CastMetal);
    let home = placed(cast);
    let rotation = Quat::from_rotation_z(0.7) * Quat::from_rotation_x(-0.4);
    let world = Affine3A::from_scale_rotation_translation(
        Vec3::new(3.0, 3.0, 3.0),
        rotation,
        Vec3::new(0.4, -0.2, 0.9),
    );
    let moved = TraceFinish::new(cast, world);
    let enlarged = placed(cast.with_scale(2.0));
    let mut rng = Rng::new(1, 1);
    for point in plane_points().take(200) {
        let here = home.relief(point, Vec3::Y, 1.0e-4, &mut rng);
        // The same spot on the part, wherever the part stands. The mesh's
        // scale stretches the part and not its grain, so the spot lies
        // three times as far from the origin in mesh units.
        let there = moved.relief(
            world.transform_point3(point / 3.0),
            rotation * Vec3::Y,
            1.0e-4,
            &mut rng,
        );
        assert!((there.slope - rotation * here.slope).length() < 6.0e-3);
        assert!((there.gloss - here.gloss).abs() < 1.0e-3);
        assert!((there.tone - here.tone).abs() < 1.0e-3);
        let larger = enlarged.relief(point * 2.0, Vec3::Y, 2.0e-4, &mut rng);
        assert!((larger.slope - here.slope).length() < 2.0e-3);
    }
    assert!(enlarged.edge_radius > 1.9 * home.edge_radius);
}

#[test]
fn turning_leaves_rings_on_a_face_and_hoops_on_a_cylinder() {
    let finish = placed(SurfaceFinish::new(Kind::MachinedMetal));
    let mut rng = Rng::new(1, 1);
    let (mut on_face, mut on_wall) = (0.0, 0.0);
    for point in plane_points().take(400) {
        let point = point + Vec3::new(0.01, 0.0, 0.01);
        let radial = Vec3::new(point.x, 0.0, point.z).normalize();
        // A face square to the axis: the grooves are circles about it.
        let face = finish.relief(point, Vec3::Y, 1.0e-6, &mut rng).slope;
        let along = face - Vec3::Y * face.y;
        assert!(
            along.cross(radial).length() < 1.0e-4,
            "the slope runs outward"
        );
        on_face += along.length();
        // The wall of a cylinder about the axis: the grooves are hoops.
        let wall = finish.relief(point, radial, 1.0e-6, &mut rng).slope;
        let along = wall - radial * wall.dot(radial);
        assert!(
            along.cross(Vec3::Y).length() < 1.0e-4,
            "the slope runs along the axis"
        );
        on_wall += along.length();
    }
    assert!(on_face / 400.0 > 0.03 && on_wall / 400.0 > 0.03);
}

#[test]
fn brushing_finer_than_a_pixel_spreads_the_highlight_across_the_streaks_only() {
    let brushed = SurfaceFinish::new(Kind::BrushedMetal).with_axis(Vec3::X);
    let finish = placed(brushed);
    let mut rng = Rng::new(1, 1);
    let (mut across, mut along, mut unresolved) = (0.0_f64, 0.0_f64, 0.0_f64);
    for point in plane_points() {
        let relief = finish.relief(point, Vec3::Y, 1.0, &mut rng);
        across += f64::from(relief.slope.z * relief.slope.z);
        along += f64::from(relief.slope.x * relief.slope.x);
        unresolved += f64::from(relief.unresolved);
    }
    let across = (across / 3000.0).sqrt() as f32;
    let stated = 0.16 * (0.92_f32.powi(2) + 0.38_f32.powi(2)).sqrt();
    assert!(
        (across / stated - 1.0).abs() < 0.12,
        "across {across} for {stated}"
    );
    assert!(along == 0.0, "nothing tilts along the brushing");
    // The broad waviness of the sheet is all that turns into roughness.
    assert!(unresolved / 3000.0 < 1.0e-4);
}

#[test]
fn resolved_brushing_runs_along_its_axis() {
    let finish = placed(SurfaceFinish::new(Kind::BrushedMetal).with_axis(Vec3::X));
    let mut rng = Rng::new(1, 1);
    let (mut across, mut along) = (0.0_f64, 0.0_f64);
    for point in plane_points() {
        let slope = finish.relief(point, Vec3::Y, 1.0e-7, &mut rng).slope;
        across += f64::from(slope.z * slope.z);
        along += f64::from(slope.x * slope.x);
    }
    assert!(
        across > 400.0 * along,
        "streaks: {across} across, {along} along"
    );
    let across = (across / 3000.0).sqrt() as f32;
    assert!((across / 0.16 - 1.0).abs() < 0.25, "across {across}");
}

#[test]
fn wear_bares_metal_on_edges_and_grime_darkens_sheltered_places() {
    let red = Vec3::new(0.6, 0.05, 0.04);
    let worn = placed(SurfaceFinish::new(Kind::PaintedMetal).with_wear(1.0));
    let soiled = placed(SurfaceFinish::new(Kind::PaintedMetal).with_grime(1.0));
    assert!(worn.grime_reach == 0.0 && soiled.grime_reach > 0.0);
    let mut rng = Rng::new(1, 1);
    let (mut clean_luma, mut dirty_luma) = (0.0, 0.0);
    for point in plane_points().take(500) {
        let relief = worn.relief(point, Vec3::Y, 1.0e-4, &mut rng);
        let mut flat = surface(red, 0.0, 0.4);
        worn.apply(&mut flat, &relief, 0.0, false);
        assert_eq!(flat.metallic, 0.0, "paint stays on a flat face");
        assert!((flat.base.x / red.x - 1.0).abs() < 0.1);
        let mut edge = surface(red, 0.0, 0.4);
        worn.apply(&mut edge, &relief, SQUARE_EDGE, false);
        assert!(edge.metallic > 0.95 && edge.clearcoat < 0.03);
        assert!((edge.base - BARE_METAL).length() < 0.05);
        assert!(
            (edge.f0 - edge.base).length() < 0.05,
            "bare metal reflects its colour"
        );

        let relief = soiled.relief(point, Vec3::Y, 1.0e-4, &mut rng);
        let (mut open, mut sheltered) = (surface(red, 0.0, 0.4), surface(red, 0.0, 0.4));
        soiled.apply(&mut open, &relief, 0.0, false);
        soiled.apply(&mut sheltered, &relief, 0.0, true);
        clean_luma += open.base.element_sum();
        dirty_luma += sheltered.base.element_sum();
        assert!(sheltered.alpha >= open.alpha, "dirt is dull");
    }
    assert!(
        dirty_luma < 0.6 * clean_luma,
        "{dirty_luma} against {clean_luma}"
    );
    let scuffed = placed(SurfaceFinish::new(Kind::Rubber).with_wear(1.0));
    let relief = scuffed.relief(Vec3::ZERO, Vec3::Y, 1.0e-4, &mut rng);
    let mut edge = surface(Vec3::splat(0.02), 0.0, 0.8);
    scuffed.apply(&mut edge, &relief, SQUARE_EDGE, false);
    assert!(
        edge.metallic == 0.0 && edge.base.x > 0.05,
        "rubber scuffs paler"
    );
}

#[test]
fn every_finish_leaves_a_valid_surface_wherever_it_is_asked() {
    for name in Kind::NAMES {
        let kind = Kind::from_name(name).expect("a listed kind");
        let finish = TraceFinish::new(
            SurfaceFinish::new(kind)
                .with_wear(0.6)
                .with_grime(0.6)
                .with_relief(SurfaceFinish::MAX_RELIEF),
            Affine3A::from_scale(Vec3::new(0.0, 2.0, -1.0)),
        );
        assert!(finish.edge_radius > 0.0 && finish.edge_radius < 0.01);
        let mut rng = Rng::new(3, 9);
        for (index, point) in plane_points().take(300).enumerate() {
            let footprint = 10.0_f32.powi(index as i32 % 9 - 8);
            let normal = Vec3::new(point.x, 0.3, point.z - 0.04).normalize();
            let relief = finish.relief(point * 40.0 - 1.5, normal, footprint, &mut rng);
            assert!(
                relief.slope.is_finite() && relief.slope.length() < 12.0,
                "{name}"
            );
            let mut lit = surface(Vec3::new(0.9, 0.5, 0.1), 0.5, 0.03);
            finish.apply(&mut lit, &relief, rng.next(), index % 2 == 0);
            assert!(
                (MIN_ROUGHNESS * MIN_ROUGHNESS..=1.0).contains(&lit.alpha),
                "{name}"
            );
            assert!(lit.base.min_element() >= 0.0 && lit.base.max_element() <= 1.0);
            assert!((0.0..=1.0).contains(&lit.metallic) && lit.f0.is_finite());
            assert!((0.0..=1.0).contains(&lit.clearcoat));
        }
    }
}
