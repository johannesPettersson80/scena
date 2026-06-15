use std::f32::consts::FRAC_PI_2;

use scena::{
    Aabb, Assets, LabelKey, MeasurementAxis, MeasurementKind, MeasurementOverlay, NodeKind,
    UnitFormat, Vec3,
};

#[test]
fn measurement_reports_distance_angle_and_bounds_dimensions() {
    let distance =
        MeasurementOverlay::distance("shaft-offset", Vec3::ZERO, Vec3::new(0.0, 3.0, 4.0))
            .with_units(UnitFormat::millimeters().with_precision(1))
            .measure()
            .expect("distance measures");
    assert_eq!(distance.kind, MeasurementKind::Distance);
    assert_eq!(distance.value, 5.0);
    assert_eq!(distance.formatted_value, "5000.0 mm");

    let angle = MeasurementOverlay::angle(
        "elbow",
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::ZERO,
        Vec3::new(0.0, 1.0, 0.0),
    )
    .with_units(UnitFormat::degrees().with_precision(0))
    .measure()
    .expect("angle measures");
    assert_eq!(angle.kind, MeasurementKind::Angle);
    assert!((angle.value - FRAC_PI_2).abs() < 1.0e-6);
    assert_eq!(angle.formatted_value, "90 deg");

    let bounds = Aabb {
        min: Vec3::new(-1.0, 2.0, -3.0),
        max: Vec3::new(3.0, 7.5, 4.0),
    };
    let height = MeasurementOverlay::bounds_dimension("height", bounds, MeasurementAxis::Y)
        .with_units(UnitFormat::custom(100.0, "cm", 1))
        .measure()
        .expect("bounds dimension measures");
    assert_eq!(height.kind, MeasurementKind::BoundsDimension);
    assert_eq!(height.value, 5.5);
    assert_eq!(height.formatted_value, "550.0 cm");
}

#[test]
fn measurement_overlay_adds_line_and_label_visuals() {
    let assets = Assets::new();
    let mut scene = scena::Scene::new();
    let overlay =
        MeasurementOverlay::distance("shaft-offset", Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0))
            .with_label("shaft offset")
            .with_units(UnitFormat::millimeters().with_precision(0));

    let report = scene
        .add_measurement_overlay(&assets, overlay)
        .expect("measurement overlay inserts");

    assert_eq!(report.id, "shaft-offset");
    assert_eq!(report.formatted_value, "1000 mm");
    assert!(matches!(
        scene
            .node(report.line_node)
            .expect("line node exists")
            .kind(),
        NodeKind::Mesh(_)
    ));
    let label = report.label.expect("label key is reported");
    assert_label_text(&scene, label, "shaft offset: 1000 mm");
}

fn assert_label_text(scene: &scena::Scene, label: LabelKey, expected: &str) {
    assert_eq!(scene.label(label).expect("label exists").text(), expected);
}
