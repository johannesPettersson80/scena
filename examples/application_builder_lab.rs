use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};

use scena::{
    APPEARANCE_EXPECTATION_SCHEMA_V1, AppearanceExpectationV1, AppearanceIntrospectionOptions,
    AssetPath, Assets, Callout, CaptureOptions, CaptureRgba8, Color, GeometryDesc, LabelDesc,
    MaterialDesc, MeasurementOverlay, PRODUCT_OPTIONS_SCHEMA_V1, PresentationTimelineActionKindV1,
    PresentationTimelineV1, ProductOptionGroupV1, ProductOptionV1, ProductOptionsV1, Profile,
    RENDER_INTROSPECTION_SCHEMA_V1, RenderIntrospectionOptions, Renderer, RendererOptions,
    SCENE_RECIPE_SCHEMA_V1, Scene, SceneHostAnimationLoopMode, SceneHostAnimationPlayOptions,
    SceneHostCameraState, SceneHostCore, SceneHostVisualStateV1, SceneInspectionReportV1,
    SceneRecipeCaptureV1, SceneRecipeExpectedExtentV1, SceneRecipeImportV1, SceneRecipeV1,
    Transform, UnitFormat, Vec3, ViewerProfile, VisualPatchLabelTargetV1, VisualPatchLabelV1,
    VisualPatchMaterialVariantV1, VisualPatchSectionBoxV1, VisualPatchSelectionV1,
    VisualPatchTintV1, VisualPatchTransformV1, VisualPatchV1, VisualPatchVisibilityV1,
    headless_gltf_viewer, validate_scene_recipe_json_with_assets,
};

const WIDTH: u32 = 320;
const HEIGHT: u32 = 220;
const OVERLAY_SUBJECT_MIN_FIT_FRACTION: f64 = 0.4;

#[derive(Debug, Serialize)]
struct LabIndex {
    schema: &'static str,
    output_dir: String,
    applications: Vec<ApplicationFinding>,
}

#[derive(Debug, Clone, Serialize)]
struct ApplicationFinding {
    application: &'static str,
    status: &'static str,
    artifacts: Vec<String>,
    worked_well: Vec<&'static str>,
    missing_or_awkward: Vec<&'static str>,
}

struct RenderedScene {
    artifacts: Vec<String>,
    capture: CaptureRgba8,
    inspection: SceneInspectionReportV1,
    renderer: Renderer,
}

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/gate-artifacts/application-builder-lab"));
    fs::create_dir_all(&out_dir)?;

    let findings = vec![
        model_viewer(&out_dir)?,
        cad_builder_and_inspection(&out_dir)?,
        digital_twin_state_viewer(&out_dir)?,
        product_configurator(&out_dir)?,
        industrial_dashboard(&out_dir)?,
        headless_documentation(&out_dir)?,
        agent_render_loop(&out_dir)?,
        data_visualization(&out_dir)?,
        animated_viewer(&out_dir)?,
        interaction_proof_viewer(&out_dir)?,
        browser_viewer_contract(&out_dir)?,
        guided_tour(&out_dir)?,
        scene_host_loop_template(&out_dir)?,
    ];

    let index = LabIndex {
        schema: "scena.application_builder_lab.v1",
        output_dir: out_dir.display().to_string(),
        applications: findings.clone(),
    };
    let index_path = out_dir.join("application-builder-lab.index.json");
    write_json(&index_path, &index)?;

    let findings_path = out_dir.join("application-builder-lab.findings.md");
    write_findings_markdown(&findings_path, &findings)?;

    println!("{}", index_path.display());
    println!("{}", findings_path.display());
    Ok(())
}

fn model_viewer(root: &Path) -> Result<ApplicationFinding, Box<dyn Error>> {
    let dir = create_app_dir(root, "model-viewer")?;
    let png = dir.join("model-viewer.png");
    let descriptor = dir.join("model-viewer.capture.json");
    let introspection = dir.join("model-viewer.render-introspection.json");

    let first = pollster::block_on(
        headless_gltf_viewer("tests/assets/gltf/khronos/UnlitTest/UnlitTest.gltf")
            .size(WIDTH, HEIGHT)
            .with_viewer_profile(ViewerProfile::model_viewer())
            .render(),
    )?;
    let capture = first.capture()?;
    capture.write_png(&png)?;
    write_json(&descriptor, &capture.descriptor)?;
    let report = first.render_introspection(RenderIntrospectionOptions::default())?;
    if report.schema != RENDER_INTROSPECTION_SCHEMA_V1 || !report.ok {
        return Err(format!(
            "render introspection failed for model-viewer: {}",
            serde_json::to_string_pretty(&report)?
        )
        .into());
    }
    write_json(&introspection, &report)?;

    Ok(ApplicationFinding {
        application: "model viewer",
        status: "runnable",
        artifacts: paths(&[png, descriptor, introspection]),
        worked_well: vec![
            "ViewerProfile::model_viewer plus headless_gltf_viewer is a compact first render path.",
            "Capture descriptors bind the PNG to renderer, camera, revision, and pixel metadata.",
            "The high-level viewer path now exposes the same render-introspection contract used by SceneHost examples.",
        ],
        missing_or_awkward: vec![
            "The high-level viewer path gives a screenshot and descriptor, but not the richer SceneHost event/patch loop.",
        ],
    })
}

fn cad_builder_and_inspection(root: &Path) -> Result<ApplicationFinding, Box<dyn Error>> {
    let dir = create_app_dir(root, "cad-builder-inspection")?;
    let mut host = SceneHostCore::headless(WIDTH, HEIGHT)?;
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/cad_plate_drawing_scene.gltf",
    )))?;
    let plate = host.node_handle(import, "CADPlate120x60mm")?;
    host.frame_node(plate)?;

    let bounds = host
        .node_world_bounds(plate)?
        .ok_or_else(|| std::io::Error::other("CAD plate should have inspectable bounds"))?;
    let section_report = host.set_section_box_json(bounds, 0.01, false, true)?;
    let measurement_report = host.add_distance_measurement_json(
        "plate-width",
        Vec3::new(-0.06, 0.0, 0.0),
        Vec3::new(0.06, 0.0, 0.0),
        Some("plate width"),
        "mm",
        1,
    )?;
    let callout = host.add_node_callout(
        "datum-callout",
        plate,
        [0.0, 0.02, 0.0],
        [0.06, 0.05, 0.0],
        "120 x 60 mm plate",
    )?;
    host.frame_all_with_overlays()?;
    host.apply_patch(&VisualPatchV1 {
        selection: Some(VisualPatchSelectionV1 { node: Some(plate) }),
        labels: vec![VisualPatchLabelV1 {
            id: "cad-plate-label".to_owned(),
            target: VisualPatchLabelTargetV1::Node {
                node: plate,
                local_offset: [0.0, 0.025, 0.0],
            },
        }],
        ..VisualPatchV1::default()
    })?;

    let mut artifacts = render_host(&mut host, &dir, "cad-builder-inspection")?;
    artifacts.push(write_json_value(
        &dir.join("section-box.json"),
        &section_report,
    )?);
    artifacts.push(write_json_value(
        &dir.join("measurement-overlay.json"),
        &measurement_report,
    )?);
    artifacts.push(write_json_path(&dir.join("callout.json"), &callout)?);

    Ok(ApplicationFinding {
        application: "CAD builder / inspection",
        status: "runnable native workflow",
        artifacts,
        worked_well: vec![
            "Authored glTF CAD-like assets can be framed, sectioned, measured, selected, and annotated from SceneHost.",
            "Measurement, section-box, label, callout, and selection all compose through normal handles.",
            "frame_all_with_overlays keeps generated measurement and callout labels inside documentation-style captures.",
        ],
        missing_or_awkward: vec![
            "This is a visual CAD inspection workflow, not a CAD kernel or drawing parser; DXF/PDF-to-solid construction remains host-side.",
            "The CLI agent template can now express measurement, section-box, callout, and exploded-view overlay directives through scene_recipe.v1; CAD kernels and drawing import remain host-side.",
        ],
    })
}

fn digital_twin_state_viewer(root: &Path) -> Result<ApplicationFinding, Box<dyn Error>> {
    let dir = create_app_dir(root, "digital-twin-state-viewer")?;
    let mut host = SceneHostCore::headless(WIDTH, HEIGHT)?;
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
    )))?;
    let node = host.node_handle(import, "ColoredTriangle")?;
    host.frame_node(node)?;

    host.store_visual_state(SceneHostVisualStateV1::new(
        "normal",
        VisualPatchV1 {
            tints: vec![VisualPatchTintV1 {
                node,
                tint: Some(Color::from_srgb_u8(70, 180, 255)),
            }],
            metadata: Some(json!({ "process_state": "normal" })),
            echo_metadata: true,
            ..VisualPatchV1::default()
        },
    ))?;
    host.store_visual_state(SceneHostVisualStateV1::new(
        "alarm",
        VisualPatchV1 {
            tints: vec![VisualPatchTintV1 {
                node,
                tint: Some(Color::from_srgb_u8(255, 80, 40)),
            }],
            labels: vec![VisualPatchLabelV1 {
                id: "alarm-label".to_owned(),
                target: VisualPatchLabelTargetV1::Node {
                    node,
                    local_offset: [0.0, 0.0, 0.0],
                },
            }],
            metadata: Some(json!({ "process_state": "alarm", "temperature_c": 92.0 })),
            echo_metadata: true,
            ..VisualPatchV1::default()
        },
    ))?;

    let timeline = PresentationTimelineV1::new()
        .at(0.0, PresentationTimelineActionKindV1::apply_state("normal"))
        .at(1.0, PresentationTimelineActionKindV1::apply_state("alarm"));
    let timeline_json = serde_json::to_string_pretty(&timeline)?;
    let seek_result = host.seek_timeline(&timeline, 1.0)?;

    let mut artifacts = render_host(&mut host, &dir, "digital-twin-alarm")?;
    artifacts.push(write_json_value(
        &dir.join("visual-states.json"),
        &host.visual_states_json()?,
    )?);
    artifacts.push(write_json_value(
        &dir.join("timeline.json"),
        &timeline_json,
    )?);
    artifacts.push(write_json_path(
        &dir.join("timeline-seek-result.json"),
        &seek_result,
    )?);

    Ok(ApplicationFinding {
        application: "digital twin state viewer",
        status: "runnable",
        artifacts,
        worked_well: vec![
            "Visual states and host-ticked timelines model live-state playback without adding a hidden render loop.",
            "Metadata echo makes it easy to correlate visual state with the host process state.",
        ],
        missing_or_awkward: vec![
            "A real twin still needs the host to own telemetry ingestion, history, units, and alarm semantics.",
        ],
    })
}

fn product_configurator(root: &Path) -> Result<ApplicationFinding, Box<dyn Error>> {
    let dir = create_app_dir(root, "product-configurator")?;
    let mut host = SceneHostCore::headless(WIDTH, HEIGHT)?;
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/material_variants_scene.gltf",
    )))?;
    let mesh = host.node_handle(import, "VariantTriangle")?;
    let accessory = host.add_empty(
        Some(host.root_handle()),
        Transform::at(Vec3::new(0.35, 0.0, 0.0)),
        Some("optional-accessory"),
    )?;

    host.store_product_options(ProductOptionsV1 {
        schema: PRODUCT_OPTIONS_SCHEMA_V1.to_owned(),
        groups: vec![
            ProductOptionGroupV1 {
                id: "finish".to_owned(),
                label: "Finish".to_owned(),
                active: None,
                options: vec![ProductOptionV1 {
                    id: "noon-green".to_owned(),
                    label: "Noon green".to_owned(),
                    patch: VisualPatchV1 {
                        material_variants: vec![VisualPatchMaterialVariantV1 {
                            import,
                            variant: Some("noon".to_owned()),
                        }],
                        tints: vec![VisualPatchTintV1 {
                            node: mesh,
                            tint: Some(Color::from_srgb_u8(50, 220, 120)),
                        }],
                        ..VisualPatchV1::default()
                    },
                    metadata: Some(json!({ "sku_suffix": "GREEN" })),
                }],
            },
            ProductOptionGroupV1 {
                id: "accessory".to_owned(),
                label: "Accessory".to_owned(),
                active: None,
                options: vec![ProductOptionV1 {
                    id: "hidden".to_owned(),
                    label: "Hidden".to_owned(),
                    patch: VisualPatchV1 {
                        visibility: vec![VisualPatchVisibilityV1 {
                            node: accessory,
                            visible: false,
                        }],
                        ..VisualPatchV1::default()
                    },
                    metadata: None,
                }],
            },
        ],
    })?;
    let finish = host.apply_product_option_json("finish", "noon-green")?;
    let accessory_result = host.apply_product_option_json("accessory", "hidden")?;
    host.frame_node(mesh)?;

    let mut artifacts = render_host(&mut host, &dir, "product-configurator")?;
    artifacts.push(write_json_value(
        &dir.join("product-options.json"),
        &host.product_options_json()?,
    )?);
    artifacts.push(write_json_value(&dir.join("finish-result.json"), &finish)?);
    artifacts.push(write_json_value(
        &dir.join("accessory-result.json"),
        &accessory_result,
    )?);

    Ok(ApplicationFinding {
        application: "product configurator",
        status: "runnable",
        artifacts,
        worked_well: vec![
            "Product options are thin visual patch groups, so variants, tints, visibility, and camera moves compose cleanly.",
            "The same patch/result contract is suitable for Rust, browser, and agent surfaces.",
        ],
        missing_or_awkward: vec![
            "Business rules, pricing, incompatible options, and SKU persistence correctly remain outside scena.",
        ],
    })
}

fn industrial_dashboard(root: &Path) -> Result<ApplicationFinding, Box<dyn Error>> {
    let dir = create_app_dir(root, "industrial-dashboard")?;
    let assets = Assets::new();
    let floor = assets.create_geometry(GeometryDesc::grid(3.0, 12));
    let body = assets.create_geometry(GeometryDesc::box_xyz(0.34, 0.22, 0.18));
    let pipe = assets.create_geometry(GeometryDesc::box_xyz(0.07, 0.07, 0.72));
    let floor_material =
        assets.create_material(MaterialDesc::line(Color::from_srgb_u8(90, 110, 130), 1.0));
    let body_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(55, 150, 220)));
    let alarm_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(235, 90, 60)));
    let pipe_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(205, 210, 220)));

    let mut scene = Scene::new();
    scene
        .mesh(floor, floor_material)
        .transform(Transform::at(Vec3::new(0.0, -0.35, 0.0)))
        .add()?;
    for (index, x) in [-0.55, 0.0, 0.55].into_iter().enumerate() {
        let material = if index == 1 {
            alarm_material
        } else {
            body_material
        };
        scene
            .mesh(body, material)
            .transform(Transform::at(Vec3::new(x, 0.0, 0.0)))
            .add()?;
        scene
            .mesh(pipe, pipe_material)
            .transform(Transform::at(Vec3::new(x, -0.18, 0.0)))
            .add()?;
    }
    scene.add_label(
        scene.root(),
        LabelDesc::new("Line A"),
        Transform::at(Vec3::new(0.0, 0.34, 0.0)),
    )?;
    let camera = scene.add_default_camera()?;
    scene.frame_all_with_assets(camera, &assets)?;

    let rendered = render_scene(
        &mut scene,
        &assets,
        &dir,
        "industrial-dashboard",
        RendererOptions::default().with_profile(Profile::Industrial),
    )?;

    Ok(ApplicationFinding {
        application: "industrial dashboard",
        status: "runnable",
        artifacts: rendered.artifacts,
        worked_well: vec![
            "The industrial profile, grid, labels, and simple visual alarm coloring are enough for a static dashboard scene.",
            "The host can keep process logic external and push only visual state.",
        ],
        missing_or_awkward: vec![
            "There is no built-in telemetry binding or dashboard widget layer, which is the right boundary but must be supplied by the app.",
        ],
    })
}

fn headless_documentation(root: &Path) -> Result<ApplicationFinding, Box<dyn Error>> {
    let dir = create_app_dir(root, "headless-documentation")?;
    let assets = Assets::new();
    let body_geometry = assets.create_geometry(GeometryDesc::box_xyz(0.8, 0.42, 0.28));
    let body_material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::BLUE, 0.0, 0.5));

    let mut scene = Scene::new();
    let body = scene
        .mesh(body_geometry, body_material)
        .transform(Transform::IDENTITY)
        .add()?;
    scene.add_callout(
        &assets,
        Callout::node(
            "primary-callout",
            body,
            Vec3::new(0.32, 0.16, 0.0),
            "Service panel",
        )
        .with_label_offset(Vec3::new(0.28, 0.24, 0.0))
        .with_color(Color::YELLOW),
    )?;
    scene.add_measurement_overlay(
        &assets,
        MeasurementOverlay::distance(
            "doc-width",
            Vec3::new(-0.4, -0.26, 0.0),
            Vec3::new(0.4, -0.26, 0.0),
        )
        .with_label("body width")
        .with_units(UnitFormat::millimeters()),
    )?;

    let camera = scene.add_default_camera()?;
    scene.frame_all_with_overlays(camera, &assets, WIDTH, HEIGHT)?;

    let rendered = render_scene(
        &mut scene,
        &assets,
        &dir,
        "headless-documentation",
        RendererOptions::default().with_profile(Profile::Quality),
    )?;

    Ok(ApplicationFinding {
        application: "headless documentation renderer",
        status: "runnable native workflow",
        artifacts: rendered.artifacts,
        worked_well: vec![
            "PNG export, capture descriptors, callouts, and measurement overlays make documentation generation straightforward.",
            "The workflow is deterministic enough for CI-produced docs images.",
            "frame_all_with_overlays reserves label margin so generated annotations are not truncated by geometry-only framing.",
        ],
        missing_or_awkward: vec![
            "The CLI agent template can now express documentation overlays through scene_recipe.v1; page layout and prose generation remain host-side.",
        ],
    })
}

fn agent_render_loop(root: &Path) -> Result<ApplicationFinding, Box<dyn Error>> {
    let dir = create_app_dir(root, "agent-render-loop")?;
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "purpose".to_owned(),
        json!("agent emits recipe, validates it, renders, and reads introspection"),
    );
    let recipe = SceneRecipeV1 {
        schema: SCENE_RECIPE_SCHEMA_V1.to_owned(),
        imports: vec![SceneRecipeImportV1 {
            id: "asset".to_owned(),
            uri: "tests/assets/gltf/mesh_material_vertex_color_scene.gltf".to_owned(),
            optional: false,
            transform: None,
            expected_extent: Some(SceneRecipeExpectedExtentV1 {
                min: 0.1,
                max: 2.0,
                unit: Some("scene units".to_owned()),
            }),
            material: None,
            edge_emphasis: None,
        }],
        colors: BTreeMap::new(),
        geometries: Vec::new(),
        morphs: Vec::new(),
        skins: Vec::new(),
        materials: Vec::new(),
        nodes: Vec::new(),
        anchors: Vec::new(),
        connectors: Vec::new(),
        bounds: Vec::new(),
        named_states: Vec::new(),
        instance_sets: Vec::new(),
        particles: Vec::new(),
        fonts: Vec::new(),
        labels: Vec::new(),
        clipping_planes: Vec::new(),
        animations: Vec::new(),
        cameras: Vec::new(),
        lights: Vec::new(),
        scene: None,
        render: None,
        photo: None,
        expect: None,
        capture: Some(SceneRecipeCaptureV1 {
            width: 160,
            height: 120,
        }),
        section_box: None,
        measurements: Vec::new(),
        callouts: Vec::new(),
        exploded_view: None,
        metadata,
    };
    let recipe_path = dir.join("agent-loop.recipe.json");
    write_json(&recipe_path, &recipe)?;
    let recipe_text = fs::read_to_string(&recipe_path)?;
    let validation = pollster::block_on(validate_scene_recipe_json_with_assets(
        recipe_path.display().to_string(),
        &recipe_text,
        &Assets::new(),
    ));
    let validation_path = dir.join("agent-loop.validation.json");
    write_json(&validation_path, &validation)?;
    if !validation.ok {
        return Err(format!("agent-loop recipe should validate: {validation:#?}").into());
    }

    let mut host = SceneHostCore::headless(160, 120)?;
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
    )))?;
    let mesh = host.node_handle(import, "ColoredTriangle")?;
    host.frame_node(mesh)?;
    let mut artifacts = render_host(&mut host, &dir, "agent-loop-render")?;
    artifacts.push(recipe_path.display().to_string());
    artifacts.push(validation_path.display().to_string());

    Ok(ApplicationFinding {
        application: "agent render loop",
        status: "runnable contract workflow",
        artifacts,
        worked_well: vec![
            "The scene recipe validator, explicit render, PNG, and render-introspection JSON form the basic act-see-diagnose loop.",
            "The recipe is a snapshot, not a script, which keeps the renderer boundary clean.",
        ],
        missing_or_awkward: vec![
            "The Rust lab can call the validator directly, but a pure shell agent should use the CLI commands and smoke templates.",
        ],
    })
}

fn data_visualization(root: &Path) -> Result<ApplicationFinding, Box<dyn Error>> {
    let dir = create_app_dir(root, "data-visualization")?;
    let assets = Assets::new();
    let bar = assets.create_geometry(GeometryDesc::box_xyz(0.18, 1.0, 0.18));
    let grid = assets.create_geometry(GeometryDesc::grid(2.5, 10));
    let grid_material =
        assets.create_material(MaterialDesc::line(Color::from_srgb_u8(70, 80, 90), 1.0));
    let mut scene = Scene::new();
    scene
        .mesh(grid, grid_material)
        .transform(Transform::at(Vec3::new(0.0, -0.55, 0.0)))
        .add()?;
    for (index, value) in [0.25_f32, 0.55, 0.9, 0.4, 0.72].into_iter().enumerate() {
        let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(
            (60.0 + value * 140.0) as u8,
            (110.0 + value * 90.0) as u8,
            220,
        )));
        let node = scene
            .mesh(bar, material)
            .transform(Transform {
                scale: Vec3::new(1.0, value, 1.0),
                ..Transform::at(Vec3::new(
                    index as f32 * 0.32 - 0.64,
                    value * 0.5 - 0.5,
                    0.0,
                ))
            })
            .add()?;
        if index == 2 {
            scene.add_tag(node, "data-mark-peak")?;
        }
    }
    scene.add_label(
        scene.root(),
        LabelDesc::new("Throughput"),
        Transform::at(Vec3::new(0.0, 0.55, 0.0)),
    )?;
    let camera = scene.add_default_camera()?;
    scene.frame_all_with_assets(camera, &assets)?;
    let mut rendered = render_scene(
        &mut scene,
        &assets,
        &dir,
        "data-visualization",
        RendererOptions::default().with_profile(Profile::Balanced),
    )?;
    let appearance_expectation: AppearanceExpectationV1 = serde_json::from_value(json!({
        "schema": APPEARANCE_EXPECTATION_SCHEMA_V1,
        "targets": [
            {
                "id": "peak-throughput-bar",
                "tag": "data-mark-peak",
                "swatch_srgb8": [186, 191, 220],
                "swatch_tolerance": 0.25,
                "color_family": "blue",
                "alpha_mode": "opaque"
            }
        ]
    }))?;
    let appearance_report = rendered.renderer.introspect_appearance(
        &rendered.capture,
        &rendered.inspection,
        &appearance_expectation,
        AppearanceIntrospectionOptions::summary(),
    );
    if !appearance_report.ok
        || appearance_report.targets.first().is_none_or(|target| {
            target.sampled_region.kind != "node_bbox" || target.sampled_region.sampled_pixels == 0
        })
    {
        return Err(format!(
            "data-visualization appearance verification failed: {}",
            serde_json::to_string_pretty(&appearance_report)?
        )
        .into());
    }
    rendered.artifacts.push(write_json_path(
        &dir.join("data-visualization.appearance-expectation.json"),
        &appearance_expectation,
    )?);
    rendered.artifacts.push(write_json_path(
        &dir.join("data-visualization.appearance-introspection.json"),
        &appearance_report,
    )?);

    Ok(ApplicationFinding {
        application: "scientific / data visualization",
        status: "runnable",
        artifacts: rendered.artifacts,
        worked_well: vec![
            "Procedural geometry plus labels is enough for small 3D data displays.",
            "Capture artifacts let an agent or CI verify that a generated visualization produced output.",
            "A tagged peak bar is verified through appearance introspection using a node-bbox sample, not a whole-frame average.",
        ],
        missing_or_awkward: vec![
            "There is no chart grammar, axis/tick layout, or data binding layer; applications still need to map data to geometry.",
        ],
    })
}

fn animated_viewer(root: &Path) -> Result<ApplicationFinding, Box<dyn Error>> {
    let dir = create_app_dir(root, "animated-viewer")?;
    let mut host = SceneHostCore::headless(WIDTH, HEIGHT)?;
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/animated_triangle_scene.glb",
    )))?;
    let triangle = host.node_handle(import, "AnimatedTriangle")?;
    let inventory = host.animation_inventory_json(import)?;
    let mixer = host.play_animation(
        import,
        "MoveTriangle",
        SceneHostAnimationPlayOptions {
            loop_mode: SceneHostAnimationLoopMode::Repeat,
            speed: 1.0,
        },
    )?;
    host.seek_animation(mixer, 0.5)?;
    host.frame_node(triangle)?;

    let mut artifacts = render_host(&mut host, &dir, "animated-viewer")?;
    artifacts.push(write_json_value(
        &dir.join("animation-inventory.json"),
        &inventory,
    )?);

    Ok(ApplicationFinding {
        application: "animated viewer",
        status: "runnable",
        artifacts,
        worked_well: vec![
            "SceneHost can inventory clips, play/seek mixers, and render a sampled pose without owning the app clock.",
            "Animation state composes with the same render/capture/introspection path as static scenes.",
        ],
        missing_or_awkward: vec![
            "The host must still decide semantic expectations for a specific moving node when verifying animations.",
        ],
    })
}

fn interaction_proof_viewer(root: &Path) -> Result<ApplicationFinding, Box<dyn Error>> {
    let dir = create_app_dir(root, "interaction-proof-viewer")?;
    let mut host = SceneHostCore::headless(128, 128)?;
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
    )))?;
    let mesh = host.node_handle(import, "ColoredTriangle")?;
    host.frame_node(mesh)?;
    host.prepare()?;
    host.render()?;
    host.drain_events();

    let hover = host.hover(64.0, 64.0)?;
    let select = host.select(64.0, 64.0)?;
    if hover != Some(mesh) || select != Some(mesh) {
        return Err(format!(
            "synthetic interaction should hit {mesh}, got hover={hover:?} select={select:?}"
        )
        .into());
    }
    let events = host.drain_events_json()?;
    let mut artifacts = render_host(&mut host, &dir, "interaction-proof-viewer")?;
    artifacts.push(write_json_value(
        &dir.join("interaction-events.json"),
        &events,
    )?);

    Ok(ApplicationFinding {
        application: "interaction proof viewer",
        status: "runnable",
        artifacts,
        worked_well: vec![
            "Synthetic hover/select runs against the same SceneHost pick path as the browser bindings.",
            "The host-event batch gives an LLM a machine-readable proof that interaction hit the intended handle.",
        ],
        missing_or_awkward: vec![
            "Run-stable handles are excellent inside one session, but not portable identifiers across independent app runs.",
        ],
    })
}

fn browser_viewer_contract(root: &Path) -> Result<ApplicationFinding, Box<dyn Error>> {
    let dir = create_app_dir(root, "browser-viewer-contract")?;
    let html = dir.join("scena-viewer-contract.html");
    let contract = dir.join("browser-contract.json");
    fs::write(
        &html,
        r#"<!doctype html>
<meta charset="utf-8">
<scena-viewer id="viewer" src="../../tests/assets/gltf/material_variants_scene.gltf"></scena-viewer>
<script type="module">
  import init, { SceneHost } from "../../target/m6-browser-pkg/scena.js";
  await init();
  const host = new SceneHost(320, 220);
  const importHandle = await host.instantiateUrl("tests/assets/gltf/material_variants_scene.gltf");
  host.frameAll();
  host.prepare();
  host.render();
  console.log(host.renderIntrospectionJson(false));
</script>
"#,
    )?;
    write_json(
        &contract,
        &json!({
            "schema": "scena.browser_viewer_contract_note.v1",
            "normal_path": "scena browser-proof scene-host or scena browser-proof m6",
            "uses": [
                "SceneHost browser API",
                "renderIntrospectionJson",
                "capturePng",
                "host event JSON"
            ]
        }),
    )?;

    Ok(ApplicationFinding {
        application: "<scena-viewer> browser app",
        status: "contract artifact only in this native lab",
        artifacts: paths(&[html, contract]),
        worked_well: vec![
            "The browser surface uses the same SceneHost contracts as the native host.",
            "scena browser-proof wraps the Playwright/browser proof lanes for actual canvas readback.",
        ],
        missing_or_awkward: vec![
            "A native Rust example cannot prove browser pixels; browser-rendered output still requires the wasm-pack and Playwright environment.",
        ],
    })
}

fn guided_tour(root: &Path) -> Result<ApplicationFinding, Box<dyn Error>> {
    let dir = create_app_dir(root, "guided-tour")?;
    let mut host = SceneHostCore::headless(WIDTH, HEIGHT)?;
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/exploded_view_assembly.gltf",
    )))?;
    let assembly = host.node_handle(import, "Assembly")?;
    host.frame_node(assembly)?;

    let overview = SceneHostCameraState {
        target: Vec3::ZERO,
        distance: 2.4,
        yaw_radians: 0.7,
        pitch_radians: 0.35,
    };
    let detail = SceneHostCameraState {
        target: Vec3::new(0.2, 0.0, 0.0),
        distance: 1.6,
        yaw_radians: 0.95,
        pitch_radians: 0.25,
    };
    let explode_patch = host.exploded_view_patch(
        assembly,
        scena::SceneHostExplodedViewOptionsV1 {
            mode: scena::SceneHostExplodedViewModeV1::Axis,
            axis: Some([1.0, 0.0, 0.0]),
            factor: 1.0,
            distance: 0.35,
            duration_seconds: None,
            easing: Default::default(),
        },
    )?;
    let timeline = PresentationTimelineV1::new()
        .with_camera_bookmark("overview", overview)
        .with_camera_bookmark("detail", detail)
        .at(
            0.0,
            PresentationTimelineActionKindV1::camera_bookmark("overview"),
        )
        .at(
            1.0,
            PresentationTimelineActionKindV1::apply_patch(explode_patch),
        )
        .at(
            2.0,
            PresentationTimelineActionKindV1::camera_bookmark("detail"),
        );
    host.seek_timeline(&timeline, 2.0)?;
    let mut artifacts = render_host(&mut host, &dir, "guided-tour")?;
    artifacts.push(write_json_path(
        &dir.join("presentation-timeline.json"),
        &timeline,
    )?);

    Ok(ApplicationFinding {
        application: "guided tour / exploded assembly",
        status: "runnable",
        artifacts,
        worked_well: vec![
            "Presentation timelines compose camera bookmarks, exploded-view patches, and normal VisualPatch actions.",
            "The host controls time explicitly, so guided tours do not introduce a hidden player loop.",
            "PresentationTimelineV1::new().with_camera_bookmark().at() keeps guided-tour authoring compact.",
        ],
        missing_or_awkward: vec![],
    })
}

fn scene_host_loop_template(root: &Path) -> Result<ApplicationFinding, Box<dyn Error>> {
    let dir = create_app_dir(root, "scenehost-loop-template")?;
    let mut host = SceneHostCore::headless(WIDTH, HEIGHT)?;
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/material_variants_scene.gltf",
    )))?;
    let mesh = host.node_handle(import, "VariantTriangle")?;
    let patch = VisualPatchV1 {
        transforms: vec![VisualPatchTransformV1 {
            node: mesh,
            transform: Transform::at(Vec3::new(0.0, 0.05, 0.0)),
        }],
        tints: vec![VisualPatchTintV1 {
            node: mesh,
            tint: Some(Color::from_srgb_u8(120, 210, 255)),
        }],
        material_variants: vec![VisualPatchMaterialVariantV1 {
            import,
            variant: Some("midnight".to_owned()),
        }],
        camera: Some(SceneHostCameraState {
            target: Vec3::ZERO,
            distance: 2.2,
            yaw_radians: 0.5,
            pitch_radians: 0.25,
        }),
        section_box: Some(VisualPatchSectionBoxV1::Disable),
        metadata: Some(json!({ "host_tick": 1 })),
        echo_metadata: true,
        ..VisualPatchV1::default()
    };
    let result = host.apply_patch(&patch)?;
    let mut artifacts = render_host(&mut host, &dir, "scenehost-loop-template")?;
    artifacts.push(write_json_path(&dir.join("visual-patch.json"), &patch)?);
    artifacts.push(write_json_path(
        &dir.join("visual-patch-result.json"),
        &result,
    )?);
    artifacts.push(write_json_value(
        &dir.join("host-events.json"),
        &host.drain_events_json()?,
    )?);

    Ok(ApplicationFinding {
        application: "SceneHost host-loop template",
        status: "runnable",
        artifacts,
        worked_well: vec![
            "Patch in, prepare/render/capture, events out is a small and repeatable host loop.",
            "The same loop supports native, browser, and agent consumers because the contracts are JSON-shaped.",
        ],
        missing_or_awkward: vec![
            "Hosts still need their own app-state model and persistence; SceneHost deliberately stays visual-only.",
        ],
    })
}

fn render_host(
    host: &mut SceneHostCore,
    dir: &Path,
    stem: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    host.prepare()?;
    host.render()?;
    let png = dir.join(format!("{stem}.png"));
    let capture = dir.join(format!("{stem}.capture.json"));
    let introspection = dir.join(format!("{stem}.render-introspection.json"));
    fs::write(&png, host.capture_png_bytes()?)?;
    write_json_value(&capture, &host.capture_json()?)?;
    let report_text = host.render_introspection_json(false)?;
    let report: Value = serde_json::from_str(&report_text)?;
    if report["schema"] != RENDER_INTROSPECTION_SCHEMA_V1 || report["ok"] != true {
        return Err(format!("render introspection failed for {stem}: {report:#}").into());
    }
    assert_overlay_subject_not_tiny(stem, &report)?;
    write_json(&introspection, &report)?;
    Ok(paths(&[png, capture, introspection]))
}

fn render_scene(
    scene: &mut Scene,
    assets: &Assets,
    dir: &Path,
    stem: &str,
    options: RendererOptions,
) -> Result<RenderedScene, Box<dyn Error>> {
    let mut renderer = Renderer::headless_with_options(WIDTH, HEIGHT, options)?;
    renderer.prepare_with_assets(scene, assets)?;
    renderer.render_active(scene)?;
    let png = dir.join(format!("{stem}.png"));
    let descriptor = dir.join(format!("{stem}.capture.json"));
    let capture = renderer.capture_rgba8(scene, CaptureOptions::default())?;
    capture.write_png(&png)?;
    write_json(&descriptor, &capture.descriptor)?;

    // Prove the frame rendered real non-background content through the same
    // render-introspection contract the SceneHost path uses, instead of trusting
    // the capture buffer length, which is a fixed size and is never empty.
    let inspection = scene.inspect_with_assets(assets).to_schema_report();
    let report =
        renderer.introspect_capture(&capture, &inspection, RenderIntrospectionOptions::default());
    let introspection = dir.join(format!("{stem}.render-introspection.json"));
    write_json(&introspection, &report)?;
    if report.schema != RENDER_INTROSPECTION_SCHEMA_V1 || !report.ok {
        return Err(format!(
            "render introspection failed for {stem}: {}",
            serde_json::to_string_pretty(&report)?
        )
        .into());
    }
    assert_overlay_subject_not_tiny_typed(stem, &report)?;
    Ok(RenderedScene {
        artifacts: paths(&[png, descriptor, introspection]),
        capture,
        inspection,
        renderer,
    })
}

fn assert_overlay_subject_not_tiny(stem: &str, report: &Value) -> Result<(), Box<dyn Error>> {
    if !requires_overlay_subject_fill_guard(stem) {
        return Ok(());
    }
    let fit_fraction = report["framing"]["fit_fraction"]
        .as_f64()
        .ok_or_else(|| format!("render introspection for {stem} omitted framing.fit_fraction"))?;
    let tiny_in_frame = report["framing"]["tiny_in_frame"]
        .as_bool()
        .ok_or_else(|| format!("render introspection for {stem} omitted framing.tiny_in_frame"))?;
    if tiny_in_frame || fit_fraction < OVERLAY_SUBJECT_MIN_FIT_FRACTION {
        return Err(format!(
            "{stem} overlay framing produced an unreadably small subject: fit_fraction={fit_fraction:.3}, tiny_in_frame={tiny_in_frame}"
        )
        .into());
    }
    Ok(())
}

fn assert_overlay_subject_not_tiny_typed(
    stem: &str,
    report: &scena::RenderIntrospectionReportV1,
) -> Result<(), Box<dyn Error>> {
    if !requires_overlay_subject_fill_guard(stem) {
        return Ok(());
    }
    let fit_fraction = f64::from(report.framing.fit_fraction);
    let tiny_in_frame = report.framing.tiny_in_frame;
    if tiny_in_frame || fit_fraction < OVERLAY_SUBJECT_MIN_FIT_FRACTION {
        return Err(format!(
            "{stem} overlay framing produced an unreadably small subject: fit_fraction={fit_fraction:.3}, tiny_in_frame={tiny_in_frame}"
        )
        .into());
    }
    Ok(())
}

fn requires_overlay_subject_fill_guard(stem: &str) -> bool {
    matches!(stem, "cad-builder-inspection" | "headless-documentation")
}

fn create_app_dir(root: &Path, name: &str) -> Result<PathBuf, std::io::Error> {
    let dir = root.join(name);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn write_json_path<T: Serialize>(path: &Path, value: &T) -> Result<String, Box<dyn Error>> {
    write_json(path, value)?;
    Ok(path.display().to_string())
}

fn write_json_value(path: &Path, text: &str) -> Result<String, Box<dyn Error>> {
    let value: Value = serde_json::from_str(text)?;
    write_json(path, &value)?;
    Ok(path.display().to_string())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), Box<dyn Error>> {
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

fn paths(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect()
}

fn write_findings_markdown(
    path: &Path,
    findings: &[ApplicationFinding],
) -> Result<(), Box<dyn Error>> {
    let mut out = String::from("# Application Builder Lab Findings\n\n");
    out.push_str(
        "Generated by `cargo run --example application_builder_lab --features scene-host`.\n\n",
    );
    for finding in findings {
        out.push_str(&format!(
            "## {} ({})\n\n",
            finding.application, finding.status
        ));
        out.push_str("Artifacts:\n");
        for artifact in &finding.artifacts {
            out.push_str(&format!("- `{artifact}`\n"));
        }
        out.push_str("\nWorked well:\n");
        for item in &finding.worked_well {
            out.push_str(&format!("- {item}\n"));
        }
        out.push_str("\nMissing or awkward:\n");
        for item in &finding.missing_or_awkward {
            out.push_str(&format!("- {item}\n"));
        }
        out.push('\n');
    }
    fs::write(path, out)?;
    Ok(())
}
