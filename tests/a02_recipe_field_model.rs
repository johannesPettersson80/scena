use std::collections::BTreeSet;

#[test]
fn recipe_field_model_covers_authoring_and_rendering_surface() {
    let model = scena::scene_recipe_field_model_v1();
    let paths = model
        .fields
        .iter()
        .map(|field| field.path.as_str())
        .collect::<BTreeSet<_>>();
    for required in [
        "$.imports[].transform.translation",
        "$.materials[].base_color_texture.uri",
        "$.materials[].normal_texture.color_space",
        "$.nodes[].lods[].max_screen_fraction",
        "$.animations[].channels[].interpolation",
        "$.cameras[].lens",
        "$.cameras[].framing.preset",
        "$.lights[].illuminance_lux",
        "$.scene.background.color",
        "$.scene.environment.preset",
        "$.render.bloom.threshold_srgb",
        "$.render.ssao.radius_px",
        "$.render.screen_space_reflections.strength",
        "$.render.depth_of_field.focus_distance",
        "$.render.depth_of_field.focus.mode",
        "$.render.depth_of_field.focus.target.kind",
        "$.render.depth_of_field.focus.target.id",
        "$.render.depth_of_field.coverage",
        "$.render.depth_of_field.strength",
        "$.render.auto_exposure.preset",
        "$.render.exposure_compensation_ev",
        "$.render.metering.mode",
        "$.render.metering.target.kind",
        "$.render.metering.target.id",
        "$.render.metering.fallback",
        "$.render.metering.rect.x",
        "$.render.metering.rect.y",
        "$.render.metering.rect.width",
        "$.render.metering.rect.height",
        "$.render.metering.surround_weight",
        "$.photo.intent",
        "$.photo.subject.kind",
        "$.photo.subject.id",
        "$.photo.subject.target.kind",
        "$.photo.subject.target.id",
        "$.photo.subject.fallback",
        "$.capture.width",
        "$.capture.height",
    ] {
        assert!(paths.contains(required), "field model omitted {required}");
    }
    assert!(
        model.fields.len() >= 350,
        "the authoritative recipe surface unexpectedly collapsed to {} fields",
        model.fields.len()
    );
}

#[test]
fn photo_ground_field_model_advertises_the_general_bounded_intents() {
    let model = scena::scene_recipe_field_model_v1();
    let ground = model
        .fields
        .iter()
        .find(|field| field.path == "$.photo.staging.ground")
        .expect("photo ground field remains discoverable");
    assert_eq!(
        ground.enum_values,
        [serde_json::json!("matte"), serde_json::json!("reflective")]
    );
    assert_eq!(ground.default, Some(serde_json::json!("matte")));
}

#[test]
fn recipe_json_schema_and_field_model_have_bidirectional_path_parity() {
    let model = scena::scene_recipe_field_model_v1();
    let schema = scena::scene_recipe_json_schema_v1();
    let schema_paths = scena::scene_recipe_json_schema_paths_v1(&schema);
    let model_paths = model
        .fields
        .iter()
        .map(|field| field.path.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(model_paths, schema_paths);
}

#[test]
fn recipe_field_model_parity_rejects_an_omitted_promoted_field() {
    let model = scena::scene_recipe_field_model_v1();
    let mut schema = scena::scene_recipe_json_schema_v1();
    assert!(remove_property(&mut schema, "screen_space_reflections"));
    let schema_paths = scena::scene_recipe_json_schema_paths_v1(&schema);
    let model_paths = model
        .fields
        .iter()
        .map(|field| field.path.clone())
        .collect::<BTreeSet<_>>();
    assert_ne!(model_paths, schema_paths);
    assert!(model_paths.contains("$.render.screen_space_reflections"));
    assert!(!schema_paths.contains("$.render.screen_space_reflections"));
}

fn remove_property(value: &mut serde_json::Value, name: &str) -> bool {
    match value {
        serde_json::Value::Object(object) => {
            if object
                .get_mut("properties")
                .and_then(serde_json::Value::as_object_mut)
                .is_some_and(|properties| properties.remove(name).is_some())
            {
                return true;
            }
            object
                .values_mut()
                .any(|value| remove_property(value, name))
        }
        serde_json::Value::Array(values) => {
            values.iter_mut().any(|value| remove_property(value, name))
        }
        _ => false,
    }
}
