use crate::scene::recipe::{SchemaFieldModelV1, scene_recipe_field_model_v1};
use serde_json::json;

use super::{
    SCHEMA_CATALOG_SCHEMA_V1, SchemaCatalogEntryV1, SchemaCatalogV1, schema_catalog_entries,
};

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
