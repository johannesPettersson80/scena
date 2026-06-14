use std::fs;

#[test]
fn placement_result_golden_fixture_matches_live_schema() {
    let text = fs::read_to_string("tests/assets/stable-contracts/placement_result.v1.json")
        .expect("placement fixture reads");
    let result: scena::ScenePlacementResultV1 =
        serde_json::from_str(&text).expect("placement fixture deserializes");

    assert_eq!(result.schema, scena::SCENE_PLACEMENT_RESULT_SCHEMA_V1);
    assert!(result.ok);
    assert_eq!(
        serde_json::to_value(&result).expect("placement result serializes"),
        serde_json::from_str::<serde_json::Value>(&text).expect("fixture parses")
    );
}
