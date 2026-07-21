use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::SCENE_RECIPE_SCHEMA_V1;

pub const FIELD_MODEL_SCHEMA_V1: &str = "scena.field_model.v1";

pub(super) const ROOT_FIELDS: &[&str] = &[
    "schema",
    "imports",
    "colors",
    "geometries",
    "morphs",
    "skins",
    "materials",
    "nodes",
    "anchors",
    "connectors",
    "bounds",
    "named_states",
    "instance_sets",
    "particles",
    "fonts",
    "labels",
    "clipping_planes",
    "animations",
    "cameras",
    "lights",
    "scene",
    "render",
    "expect",
    "section_box",
    "measurements",
    "callouts",
    "exploded_view",
    "capture",
    "metadata",
];
pub(super) const IMPORT_FIELDS: &[&str] = &[
    "id",
    "uri",
    "optional",
    "transform",
    "expected_extent",
    "material",
    "edge_emphasis",
];
pub(super) const IMPORT_TRANSFORM_KINDS: &[&str] = &["raw", "trs"];
pub(super) const AUTHORING_TRANSFORM_KINDS: &[&str] = &[
    "raw",
    "trs",
    "look_at",
    "center",
    "ground",
    "fit_to_size",
    "place_on",
    "align_to_anchor",
];
pub(super) const CAPTURE_FIELDS: &[&str] = &["width", "height"];
pub(super) const PRIMITIVE_KINDS: &[&str] = &[
    "arrow", "axes", "box", "cone", "cylinder", "disc", "grid", "line", "plane", "polyline",
    "sphere", "torus", "wedge",
];
pub(super) const RENDER_PROFILES: &[&str] =
    &["auto", "quality", "balanced", "compatibility", "industrial"];
pub(super) const RENDER_QUALITIES: &[&str] = &["low", "medium", "high"];
pub(super) const ANTI_ALIASING_MODES: &[&str] = &["none", "fxaa", "msaa4", "msaa8"];
pub(super) const RECONSTRUCTION_FILTERS: &[&str] = &["box", "tent", "gaussian"];
pub(super) const TONEMAPPERS: &[&str] = &["standard", "aces", "pbr_neutral"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaFieldModelV1 {
    pub schema: String,
    pub contract: String,
    pub fields: Vec<SchemaFieldV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaFieldV1 {
    pub path: String,
    #[serde(rename = "type")]
    pub value_type: String,
    pub required: bool,
    #[serde(rename = "enum", default, skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    pub deprecated: bool,
    pub examples: Vec<Value>,
}

pub fn scene_recipe_field_model_v1() -> SchemaFieldModelV1 {
    let mut fields = vec![
        field("$.schema", "string", true, json!(SCENE_RECIPE_SCHEMA_V1))
            .with_enum_strings(&[SCENE_RECIPE_SCHEMA_V1]),
    ];
    for (name, value_type, default) in [
        ("imports", "array", json!([])),
        ("colors", "object", json!({})),
        ("geometries", "array", json!([])),
        ("morphs", "array", json!([])),
        ("skins", "array", json!([])),
        ("materials", "array", json!([])),
        ("nodes", "array", json!([])),
        ("anchors", "array", json!([])),
        ("connectors", "array", json!([])),
        ("bounds", "array", json!([])),
        ("named_states", "array", json!([])),
        ("instance_sets", "array", json!([])),
        ("particles", "array", json!([])),
        ("fonts", "array", json!([])),
        ("labels", "array", json!([])),
        ("clipping_planes", "array", json!([])),
        ("animations", "array", json!([])),
        ("cameras", "array", json!([])),
        ("lights", "array", json!([])),
        ("scene", "object", Value::Null),
        ("render", "object", Value::Null),
        ("expect", "object", Value::Null),
        ("section_box", "object", Value::Null),
        ("measurements", "array", json!([])),
        ("callouts", "array", json!([])),
        ("exploded_view", "object", Value::Null),
        ("capture", "object", Value::Null),
        ("metadata", "object", json!({})),
    ] {
        fields.push(
            field(&format!("$.{name}"), value_type, false, default.clone()).with_default(default),
        );
    }
    fields.extend([
        field("$.imports[].id", "string", true, json!("part")),
        field("$.imports[].uri", "string", true, json!("assets/part.glb")),
        field("$.imports[].optional", "boolean", false, json!(false)).with_default(json!(false)),
        field(
            "$.imports[].transform",
            "object",
            false,
            json!({"kind":"trs"}),
        ),
        field("$.imports[].transform.kind", "string", true, json!("trs"))
            .with_enum_strings(IMPORT_TRANSFORM_KINDS),
        field("$.geometries[].id", "string", true, json!("box_geo")),
        field(
            "$.geometries[].primitive",
            "object",
            false,
            json!({"kind":"box","size":[1.0,1.0,1.0]}),
        ),
        field(
            "$.geometries[].mesh",
            "object",
            false,
            json!({"topology":"triangles","positions":[],"indices":[]}),
        ),
        field(
            "$.geometries[].primitive.kind",
            "string",
            true,
            json!("box"),
        )
        .with_enum_strings(PRIMITIVE_KINDS),
        field(
            "$.geometries[].primitive.size",
            "array",
            false,
            json!([1.0, 1.0, 1.0]),
        ),
        field(
            "$.geometries[].primitive.radius",
            "number",
            false,
            json!(0.5),
        )
        .with_minimum(0.0),
        field(
            "$.geometries[].primitive.height",
            "number",
            false,
            json!(1.0),
        )
        .with_minimum(0.0),
        field(
            "$.geometries[].primitive.segments",
            "integer",
            false,
            json!(32),
        )
        .with_minimum(3.0),
        field("$.materials[].id", "string", true, json!("paint")),
        field(
            "$.materials[].kind",
            "string",
            false,
            json!("pbr_metallic_roughness"),
        )
        .with_enum_strings(&[
            "unlit",
            "pbr_metallic_roughness",
            "line",
            "wireframe",
            "edge",
        ]),
        field(
            "$.materials[].preset",
            "string",
            false,
            json!("brushed_steel"),
        ),
        field(
            "$.materials[].base_color",
            "string",
            false,
            json!("#D8C69A"),
        ),
        field("$.materials[].metallic", "number", false, json!(0.0)).with_range(0.0, 1.0),
        field("$.materials[].roughness", "number", false, json!(0.5)).with_range(0.0, 1.0),
        field("$.materials[].double_sided", "boolean", false, json!(false))
            .with_default(json!(false)),
        field(
            "$.materials[].alpha_mode.kind",
            "string",
            false,
            json!("opaque"),
        )
        .with_enum_strings(&["opaque", "mask", "blend"]),
        field("$.nodes[].id", "string", true, json!("part")),
        field("$.nodes[].geometry", "string", false, json!("box_geo")),
        field("$.nodes[].material", "string", false, json!("paint")),
        field(
            "$.nodes[].transform",
            "object",
            false,
            json!({"kind":"trs"}),
        ),
        field("$.nodes[].transform.kind", "string", true, json!("trs"))
            .with_enum_strings(AUTHORING_TRANSFORM_KINDS),
        field("$.anchors[].id", "string", true, json!("mount")),
        field("$.anchors[].source.kind", "string", true, json!("authored"))
            .with_enum_strings(&["authored", "import"]),
        field(
            "$.anchors[].source.target.kind",
            "string",
            false,
            json!("node"),
        )
        .with_enum_strings(&["node", "import_root", "import_node"]),
        field("$.connectors[].id", "string", true, json!("plug")),
        field(
            "$.connectors[].source.kind",
            "string",
            true,
            json!("authored"),
        )
        .with_enum_strings(&["authored", "import"]),
        field("$.connectors[].polarity", "string", false, json!("plug"))
            .with_enum_strings(&["plug", "socket", "neutral"]),
        field(
            "$.connectors[].roll_policy",
            "string",
            false,
            json!("preserve"),
        )
        .with_enum_strings(&["preserve", "choose_nearest"]),
        field(
            "$.connectors[].mate.target",
            "string",
            false,
            json!("socket"),
        ),
        field("$.bounds[].id", "string", true, json!("work_zone")),
        field("$.bounds[].source", "string", true, json!("computed"))
            .with_enum_strings(&["computed", "imported", "authored"]),
        field("$.bounds[].target.kind", "string", true, json!("node")).with_enum_strings(&[
            "node",
            "import_root",
            "import_node",
        ]),
        field("$.bounds[].min", "array", false, json!([-0.5, -0.5, -0.5])),
        field("$.bounds[].max", "array", false, json!([0.5, 0.5, 0.5])),
        field("$.named_states[].id", "string", true, json!("inspection")),
        field("$.named_states[].inherits", "string", false, json!("base")),
        field("$.named_states[].active", "boolean", false, json!(false)).with_default(json!(false)),
        field("$.named_states[].transforms", "array", false, json!([])).with_default(json!([])),
        field("$.named_states[].tints", "array", false, json!([])).with_default(json!([])),
        field("$.named_states[].visibility", "array", false, json!([])).with_default(json!([])),
        field("$.cameras[].id", "string", true, json!("main")),
        field("$.cameras[].kind", "string", true, json!("perspective"))
            .with_enum_strings(&["perspective", "orthographic"]),
        field("$.cameras[].active", "boolean", false, json!(true)).with_default(json!(false)),
        field("$.lights[].id", "string", true, json!("key")),
        field("$.lights[].kind", "string", true, json!("directional")).with_enum_strings(&[
            "directional",
            "point",
            "spot",
            "area",
            "ambient",
            "hemisphere",
            "studio_rig",
        ]),
        field(
            "$.scene.environment.kind",
            "string",
            false,
            json!("default"),
        )
        .with_enum_strings(&["default", "uri", "none"]),
        field(
            "$.scene.environment.uri",
            "string",
            false,
            json!("assets/studio.hdr"),
        ),
        field(
            "$.scene.environment.optional",
            "boolean",
            false,
            json!(false),
        )
        .with_default(json!(false)),
        field("$.render.profile", "string", false, json!("quality"))
            .with_enum_strings(RENDER_PROFILES),
        field("$.render.quality", "string", false, json!("high"))
            .with_enum_strings(RENDER_QUALITIES),
        field("$.render.anti_aliasing", "string", false, json!("fxaa"))
            .with_enum_strings(ANTI_ALIASING_MODES),
        field("$.render.supersample", "integer", false, json!(2))
            .with_enum_values(&[1, 2, 3, 4, 8]),
        field("$.render.reconstruction", "string", false, json!("tent"))
            .with_enum_strings(RECONSTRUCTION_FILTERS),
        field("$.render.tonemapper", "string", false, json!("pbr_neutral"))
            .with_enum_strings(TONEMAPPERS),
        field("$.capture.width", "integer", true, json!(800)).with_range(1.0, f64::from(u32::MAX)),
        field("$.capture.height", "integer", true, json!(600)).with_range(1.0, f64::from(u32::MAX)),
    ]);
    SchemaFieldModelV1 {
        schema: FIELD_MODEL_SCHEMA_V1.to_owned(),
        contract: SCENE_RECIPE_SCHEMA_V1.to_owned(),
        fields,
    }
}

fn field(path: &str, value_type: &str, required: bool, example: Value) -> SchemaFieldV1 {
    SchemaFieldV1 {
        path: path.to_owned(),
        value_type: value_type.to_owned(),
        required,
        enum_values: Vec::new(),
        minimum: None,
        maximum: None,
        default: None,
        deprecated: false,
        examples: vec![example],
    }
}

impl SchemaFieldV1 {
    fn with_enum_strings(mut self, values: &[&str]) -> Self {
        self.enum_values = values.iter().map(|value| json!(value)).collect();
        self
    }

    fn with_enum_values(mut self, values: &[u64]) -> Self {
        self.enum_values = values.iter().map(|value| json!(value)).collect();
        self
    }

    fn with_minimum(mut self, minimum: f64) -> Self {
        self.minimum = Some(minimum);
        self
    }

    fn with_range(mut self, minimum: f64, maximum: f64) -> Self {
        self.minimum = Some(minimum);
        self.maximum = Some(maximum);
        self
    }

    fn with_default(mut self, default: Value) -> Self {
        self.default = Some(default);
        self
    }
}
