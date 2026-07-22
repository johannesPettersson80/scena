use crate::scene::recipe::{SchemaFieldModelV1, scene_recipe_field_model_v1};
use serde_json::json;

use super::{
    SCHEMA_CATALOG_SCHEMA_V1, SCHEMA_ENTRY_SCHEMA_V1, SchemaCatalogEntryV1, SchemaCatalogV1,
    SchemaEntryReportV1, entries, fixtures, schema_entry_rows,
};

fn schema_catalog_entries() -> Vec<SchemaCatalogEntryV1> {
    schema_entry_rows()
        .iter()
        .chain(entries::operational_schema_entry_rows())
        .map(|row| SchemaCatalogEntryV1 {
            schema: row.schema.to_owned(),
            owner_module: row.owner_module.to_owned(),
            summary: row.summary.to_owned(),
            feature_flag: row.feature_flag.map(str::to_owned),
            fixture_path: row.fixture_path.map(str::to_owned),
        })
        .collect()
}

pub fn schema_catalog_v1() -> SchemaCatalogV1 {
    SchemaCatalogV1 {
        schema: SCHEMA_CATALOG_SCHEMA_V1.to_owned(),
        entries: schema_catalog_entries(),
    }
}

pub fn schema_catalog_entry(schema: &str) -> Option<SchemaCatalogEntryV1> {
    schema_catalog_entries()
        .into_iter()
        .find(|entry| entry.schema == schema)
}

pub fn schema_entry_report_v1(schema: &str) -> Option<SchemaEntryReportV1> {
    let entry = schema_catalog_entry(schema)?;
    let example = fixtures::schema_fixture_json(schema)
        .and_then(|fixture| serde_json::from_str(fixture).ok())
        .unwrap_or(serde_json::Value::Null);
    Some(SchemaEntryReportV1 {
        schema: SCHEMA_ENTRY_SCHEMA_V1.to_owned(),
        entry,
        example,
        invalid_example: invalid_example_for_schema(schema),
        field_model: field_model_for_schema(schema),
    })
}

pub(super) fn field_model_for_schema(schema: &str) -> Option<SchemaFieldModelV1> {
    (schema == crate::SCENE_RECIPE_SCHEMA_V1).then(scene_recipe_field_model_v1)
}

pub(super) fn invalid_example_for_schema(schema: &str) -> Option<serde_json::Value> {
    match schema {
        "scena.scene_recipe.v1" => Some(json!({
            "schema": "scena.scene_recipe.v1",
            "importe": [{
                "id": "part",
                "uri": "tests/assets/gltf/mesh_material_vertex_color_scene.gltf"
            }]
        })),
        _ => None,
    }
}
