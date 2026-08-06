#![cfg(feature = "scene-host")]

use scena::scene::recipe::{SceneRecipeImportEdgeRoundingV1, SceneRecipeV1};
use scena::{GeometryDesc, Vec3};

#[test]
fn import_edge_rounding_contract_round_trips_with_explicit_controls() {
    let recipe: SceneRecipeV1 = serde_json::from_value(serde_json::json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "product",
            "uri": "product.glb",
            "edge_rounding": {
                "enabled": true,
                "radius_fraction": 0.0025,
                "segments": 3,
                "edge_angle_threshold_degrees": 30.0,
                "max_derived_triangles": 250000
            }
        }],
        "scene": {},
        "render": {}
    }))
    .expect("edge_rounding is part of the typed recipe");

    assert_eq!(
        recipe.imports[0].edge_rounding,
        Some(SceneRecipeImportEdgeRoundingV1::default().with_max_derived_triangles(250_000))
    );
}

#[test]
fn import_edge_rounding_rejects_unsafe_or_ambiguous_controls() {
    let validation = scena::validate_scene_recipe_value(serde_json::json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "product",
            "uri": "product.glb",
            "edge_rounding": {
                "enabled": true,
                "radius_fraction": 0.0,
                "segments": 0,
                "edge_angle_threshold_degrees": 180.0,
                "max_derived_triangles": 0
            }
        }],
        "scene": {},
        "render": {}
    }));
    let diagnostics = validation.diagnostics;

    for path in [
        "$.imports[0].edge_rounding.radius_fraction",
        "$.imports[0].edge_rounding.segments",
        "$.imports[0].edge_rounding.edge_angle_threshold_degrees",
        "$.imports[0].edge_rounding.max_derived_triangles",
    ] {
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.path == path),
            "missing structured edge-rounding diagnostic for {path}: {diagnostics:#?}"
        );
    }
}

#[test]
fn frozen_handmade_glb_receives_reported_render_only_curve_refinement() {
    let recipe = serde_json::json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "speaker",
            "uri": "subjects/dark_metal_speaker.glb",
            "edge_rounding": {
                "enabled": true,
                "radius_fraction": 0.0025,
                "segments": 3,
                "edge_angle_threshold_degrees": 30.0,
                "max_derived_triangles": 250000
            }
        }],
        "scene": {},
        "render": {},
        "capture": { "width": 320, "height": 210 }
    });
    let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/photo/final/edge-rounding.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("the frozen static GLB supports opt-in edge rounding");

    let report = build.manifest.imports[0]
        .edge_rounding
        .as_ref()
        .expect("build manifest reports derived edge geometry");
    assert!(report.inspected_meshes > 0);
    assert!(report.rounded_meshes > 0, "{report:#?}");
    assert_eq!(report.rounded_edges, report.eligible_edges);
    assert!(report.derived_triangles > report.source_triangles);
}

#[test]
fn all_frozen_photo_subjects_receive_budgeted_render_only_curve_refinement() {
    let policy = scena::RecipeBuildPolicy::testing().with_allowed_root(
        std::path::Path::new(".")
            .canonicalize()
            .expect("repository root canonicalizes"),
    );
    for (name, uri) in [
        (
            "dark_metal_speaker",
            "tests/assets/photo/final/subjects/dark_metal_speaker.glb",
        ),
        (
            "colored_travel_mug",
            "tests/assets/photo/final/subjects/colored_travel_mug.glb",
        ),
        (
            "valve_manifold",
            "tests/assets/photo/final/subjects/valve_manifold.glb",
        ),
        (
            "demo_hero",
            "demo/samples/connector-snap/connector_snap_assembly.glb",
        ),
    ] {
        let recipe_path = format!("{name}.curve-refinement.recipe.json");
        let recipe_text = serde_json::to_string_pretty(&serde_json::json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{
                "id": "subject",
                "uri": uri,
                "edge_rounding": {
                    "enabled": true,
                    "radius_fraction": 0.0025,
                    "segments": 3,
                    "edge_angle_threshold_degrees": 30.0,
                    "max_derived_triangles": 250000
                }
            }],
            "scene": {},
            "render": {},
            "capture": { "width": 3840, "height": 2520 }
        }))
        .expect("focused curve-refinement recipe serializes");
        let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
            &recipe_path,
            &recipe_text,
            policy.clone(),
        ))
        .unwrap_or_else(|manifest| {
            panic!("{name} final recipe builds with edge rounding: {manifest:#?}")
        });
        let report = build.manifest.imports[0]
            .edge_rounding
            .as_ref()
            .unwrap_or_else(|| panic!("{name} build reports edge rounding"));
        eprintln!(
            "{name}: source_triangles={} derived_triangles={} rounded_meshes={} skipped_meshes={}",
            report.source_triangles,
            report.derived_triangles,
            report.rounded_meshes,
            report.skipped_meshes
        );
        if name == "valve_manifold" {
            for draw in build
                .host
                .scene()
                .inspect_with_assets(build.host.assets())
                .draw_list()
            {
                let half_extent = draw.local_bounds().half_extent();
                if !(0.035..0.037).contains(&half_extent.x)
                    || !(0.035..0.037).contains(&half_extent.z)
                    || half_extent.y >= 0.007
                {
                    continue;
                }
                let geometry = build
                    .host
                    .assets()
                    .try_geometry(draw.geometry())
                    .expect("derived flange geometry resolves");
                let maximum_radius = geometry
                    .vertices()
                    .iter()
                    .map(|vertex| vertex.position.x.hypot(vertex.position.z))
                    .fold(0.0_f32, f32::max);
                let outer_angles = geometry
                    .vertices()
                    .iter()
                    .filter(|vertex| {
                        vertex.position.x.hypot(vertex.position.z) >= maximum_radius - 0.000_2
                    })
                    .map(|vertex| {
                        (vertex.position.z.atan2(vertex.position.x) * 1_000_000.0).round() as i32
                    })
                    .collect::<std::collections::BTreeSet<_>>();
                let mut coincident =
                    std::collections::BTreeMap::<[i32; 3], Vec<(scena::Vec3, [f32; 2])>>::new();
                for (index, vertex) in geometry.vertices().iter().enumerate() {
                    let key = [
                        (vertex.position.x * 10_000_000.0).round() as i32,
                        (vertex.position.y * 10_000_000.0).round() as i32,
                        (vertex.position.z * 10_000_000.0).round() as i32,
                    ];
                    coincident.entry(key).or_default().push((
                        vertex.normal,
                        geometry
                            .tex_coords0()
                            .get(index)
                            .copied()
                            .unwrap_or_default(),
                    ));
                }
                let mut discontinuous_normal_groups = 0;
                let mut discontinuous_uv_groups = 0;
                let mut maximum_normal_angle = 0.0_f32;
                for group in coincident.values().filter(|group| group.len() > 1) {
                    let (first_normal, first_uv) = group[0];
                    for (normal, uv) in &group[1..] {
                        let angle = first_normal
                            .normalize_or_zero()
                            .dot(normal.normalize_or_zero())
                            .clamp(-1.0, 1.0)
                            .acos();
                        maximum_normal_angle = maximum_normal_angle.max(angle);
                        if angle.to_degrees() > 0.1 {
                            discontinuous_normal_groups += 1;
                            break;
                        }
                        if (uv[0] - first_uv[0]).abs() > 1.0e-5
                            || (uv[1] - first_uv[1]).abs() > 1.0e-5
                        {
                            discontinuous_uv_groups += 1;
                            break;
                        }
                    }
                }
                eprintln!(
                    "valve flange: triangles={} maximum_radius={} outer_angles={} \
                     coincident_groups={} discontinuous_normal_groups={} \
                     maximum_normal_angle={} discontinuous_uv_groups={}",
                    draw.primitive_count(),
                    maximum_radius,
                    outer_angles.len(),
                    coincident.values().filter(|group| group.len() > 1).count(),
                    discontinuous_normal_groups,
                    maximum_normal_angle.to_degrees(),
                    discontinuous_uv_groups
                );
            }
        }

        assert!(report.enabled, "{name}: {report:#?}");
        assert!(report.inspected_meshes > 0, "{name}: {report:#?}");
        assert!(report.rounded_meshes > 0, "{name}: {report:#?}");
        assert_eq!(
            report.rounded_edges, report.eligible_edges,
            "{name}: {report:#?}"
        );
        assert_eq!(report.rejected_edges, 0, "{name}: {report:#?}");
        assert!(report.derived_triangles <= 250_000, "{name}: {report:#?}");
        assert!(
            report.derived_triangles > report.source_triangles,
            "{name}: {report:#?}"
        );
    }
}

#[test]
fn hero_shaft_curve_refinement_stays_inside_the_source_geometry_envelope() {
    let source = build_hero_shaft_geometry(false);
    let refined = build_hero_shaft_geometry(true);
    let mut maximum_displacement_fraction = 0.0_f32;

    for vertex in refined.vertices() {
        let (distance, local_edge) = nearest_source_triangle_distance(&source, vertex.position);
        maximum_displacement_fraction =
            maximum_displacement_fraction.max(distance / local_edge.max(1.0e-8));
    }

    let canonical_view = Vec3::new(1.0, 0.35, 1.0).normalize();
    let canonical_right = canonical_view.cross(Vec3::Y).normalize();
    let canonical_up = canonical_right.cross(canonical_view).normalize();
    let source_silhouette = projected_bounds(&source, canonical_right, canonical_up);
    let refined_silhouette = projected_bounds(&refined, canonical_right, canonical_up);
    let source_span = (source_silhouette[1] - source_silhouette[0])
        .max(source_silhouette[3] - source_silhouette[2]);
    let silhouette_deviation = source_silhouette
        .into_iter()
        .zip(refined_silhouette)
        .map(|(source, refined)| (source - refined).abs())
        .fold(0.0_f32, f32::max)
        / source_span.max(1.0e-8);

    eprintln!(
        "hero shaft source_triangles={} refined_triangles={} \
         maximum_displacement_fraction={maximum_displacement_fraction:.6} \
         silhouette_deviation={silhouette_deviation:.6} \
         source_bounds={:?} refined_bounds={:?}",
        source.indices().len() / 3,
        refined.indices().len() / 3,
        source.bounds(),
        refined.bounds(),
    );
    assert_eq!(
        refined.indices().len(),
        source.indices().len(),
        "the over-envelope hero shaft must fall back to its source geometry"
    );
    assert_eq!(
        refined.vertices(),
        source.vertices(),
        "the fallback must preserve the exact source shaft positions and normals, not \
         return an already rounded intermediate"
    );
    assert_eq!(
        refined.indices(),
        source.indices(),
        "the fallback must preserve the exact source shaft topology"
    );
    assert!(
        maximum_displacement_fraction <= 0.05 + 1.0e-4,
        "hero shaft derived vertices must remain within 5% of their nearest local \
         source edge; measured {maximum_displacement_fraction:.6}"
    );
    assert!(
        silhouette_deviation <= 0.005,
        "hero shaft canonical-view silhouette must remain straight and source-faithful; \
         normalized deviation {silhouette_deviation:.6}"
    );
}

fn build_hero_shaft_geometry(edge_rounding: bool) -> GeometryDesc {
    let policy = scena::RecipeBuildPolicy::testing().with_allowed_root(
        std::path::Path::new(".")
            .canonicalize()
            .expect("repository root canonicalizes"),
    );
    let recipe_path = if edge_rounding {
        "hero-shaft-refined.recipe.json"
    } else {
        "hero-shaft-source.recipe.json"
    };
    let mut recipe = serde_json::json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "machine",
            "uri": "demo/samples/connector-snap/connector_snap_assembly.glb"
        }],
        "scene": {},
        "render": {},
        "capture": { "width": 3840, "height": 2520 }
    });
    if edge_rounding {
        recipe["imports"][0]["edge_rounding"] = serde_json::json!({
            "enabled": true,
            "radius_fraction": 0.0025,
            "segments": 3,
            "edge_angle_threshold_degrees": 30.0,
            "max_derived_triangles": 250000
        });
    }
    let recipe_text =
        serde_json::to_string_pretty(&recipe).expect("hero shaft protection recipe serializes");
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        recipe_path,
        &recipe_text,
        policy,
    ))
    .expect("frozen hero GLB builds for shaft protection proof");
    let shaft_handle = build.manifest.imports[0].nodes_by_path["machine:/drive_unit/drive shaft"];
    let inspection = build.host.scene().inspect_with_assets(build.host.assets());
    let schema: scena::SceneInspectionReportV1 = serde_json::from_str(
        &build
            .host
            .inspect_json()
            .expect("hero scene inspection serializes"),
    )
    .expect("hero scene inspection has the stable schema");
    let draw = inspection
        .draw_list()
        .iter()
        .zip(&schema.draw_list)
        .find(|(_, schema_draw)| schema_draw.node == shaft_handle)
        .map(|(draw, _)| draw)
        .expect("hero shaft remains identifiable by its exact manifest import-node path");
    assert_eq!(
        draw.material_preview()
            .and_then(|material| material.source())
            .and_then(|source| source.material_index()),
        Some(10),
        "the exact drive-shaft node retains manifest-declared source material 10"
    );
    build
        .host
        .assets()
        .try_geometry(draw.geometry())
        .expect("hero shaft geometry resolves")
        .clone()
}

fn nearest_source_triangle_distance(source: &GeometryDesc, point: Vec3) -> (f32, f32) {
    source
        .indices()
        .chunks_exact(3)
        .map(|triangle| {
            let a = source.vertices()[triangle[0] as usize].position;
            let b = source.vertices()[triangle[1] as usize].position;
            let c = source.vertices()[triangle[2] as usize].position;
            let distance = point.distance(closest_point_on_triangle(point, a, b, c));
            let local_edge = a
                .distance(b)
                .min(b.distance(c))
                .min(c.distance(a))
                .max(1.0e-8);
            (distance, local_edge)
        })
        .min_by(|left, right| left.0.total_cmp(&right.0))
        .expect("hero shaft source contains triangles")
}

fn closest_point_on_triangle(point: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let ab = b - a;
    let ac = c - a;
    let ap = point - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = point - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = point - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denominator = 1.0 / (va + vb + vc);
    a + ab * (vb * denominator) + ac * (vc * denominator)
}

fn projected_bounds(geometry: &GeometryDesc, right: Vec3, up: Vec3) -> [f32; 4] {
    geometry.vertices().iter().fold(
        [
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ],
        |mut bounds, vertex| {
            let x = vertex.position.dot(right);
            let y = vertex.position.dot(up);
            bounds[0] = bounds[0].min(x);
            bounds[1] = bounds[1].max(x);
            bounds[2] = bounds[2].min(y);
            bounds[3] = bounds[3].max(y);
            bounds
        },
    )
}
