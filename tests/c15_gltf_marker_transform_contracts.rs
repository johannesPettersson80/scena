use scena::{AssetError, AssetFetcher, AssetPath, Assets, Scene, SourceUnits, Vec3};
use std::collections::BTreeMap;
use std::future::{Ready, ready};
use std::sync::Arc;

fn marker_document(kind: &str, marker: &str) -> String {
    format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "nodes": [{{
                "name": "Fixture",
                "extras": {{ "scena": {{ "{kind}": [{marker}] }} }}
            }}]
        }}"#
    )
}

fn load_marker_error(kind: &str, marker: &str) -> (String, String) {
    let path = format!("memory://c15-{kind}.gltf");
    let assets = Assets::with_fetcher(MemoryFetcher::new(
        path.as_str(),
        marker_document(kind, marker),
    ));
    let error = pollster::block_on(assets.load_scene(path.as_str()))
        .expect_err("invalid marker transform must abort the asset transaction");
    match error {
        AssetError::Parse { path, reason } => (path, reason),
        other => panic!("expected path-qualified parse error, got {other:?}"),
    }
}

#[test]
fn anchor_and_connector_basis_vectors_fail_closed() {
    for (kind, singular, marker, field) in [
        (
            "anchors",
            "anchor",
            r#"{ "name": "zero", "forward": [0.0, 0.0, 0.0], "up": [0.0, 1.0, 0.0] }"#,
            "forward",
        ),
        (
            "connectors",
            "connector",
            r#"{ "name": "parallel", "forward": [0.0, 1.0, 0.0], "up": [0.0, 2.0, 0.0] }"#,
            "up",
        ),
        (
            "anchors",
            "anchor",
            r#"{ "name": "unpaired-forward", "forward": [1.0, 0.0, 0.0] }"#,
            "up",
        ),
        (
            "connectors",
            "connector",
            r#"{ "name": "unpaired-up", "up": [0.0, 1.0, 0.0] }"#,
            "forward",
        ),
        (
            "anchors",
            "anchor",
            r#"{ "name": "overflow", "forward": [3.5e38, 0.0, 0.0], "up": [0.0, 1.0, 0.0] }"#,
            "forward",
        ),
    ] {
        let (path, reason) = load_marker_error(kind, marker);
        assert_eq!(path, format!("memory://c15-{kind}.gltf"));
        assert!(
            reason.contains(&format!("nodes[0].extras.scena.{kind}[0].{field}")),
            "{singular} error must identify the exact authored field: {reason}"
        );
    }
}

#[test]
fn anchor_and_connector_trs_fail_closed() {
    for (kind, marker, field) in [
        (
            "anchors",
            r#"{ "name": "zero-scale", "scale": [1.0, 0.0, 1.0] }"#,
            "scale",
        ),
        (
            "connectors",
            r#"{ "name": "zero-quaternion", "rotation": [0.0, 0.0, 0.0, 0.0] }"#,
            "rotation",
        ),
        (
            "connectors",
            r#"{ "name": "overflow-quaternion", "rotation": [0.0, 0.0, 0.0, 3.5e38] }"#,
            "rotation",
        ),
        (
            "anchors",
            r#"{ "name": "overflow-translation", "translation": [0.0, 3.5e38, 0.0] }"#,
            "translation",
        ),
    ] {
        let (_, reason) = load_marker_error(kind, marker);
        assert!(
            reason.contains(&format!("nodes[0].extras.scena.{kind}[0].{field}")),
            "TRS error must identify the exact authored field: {reason}"
        );
    }
}

#[test]
fn anchor_and_connector_matrices_fail_closed() {
    for (name, matrix, detail) in [
        ("short", "[1.0, 0.0, 0.0]", "16"),
        (
            "overflow",
            "[1.0,0.0,0.0,0.0, 0.0,3.5e38,0.0,0.0, 0.0,0.0,1.0,0.0, 0.0,0.0,0.0,1.0]",
            "finite",
        ),
        (
            "shear",
            "[1.0,0.0,0.0,0.0, 0.5,1.0,0.0,0.0, 0.0,0.0,1.0,0.0, 0.0,0.0,0.0,1.0]",
            "decomposable",
        ),
        (
            "projective",
            "[1.0,0.0,0.0,0.1, 0.0,1.0,0.0,0.0, 0.0,0.0,1.0,0.0, 0.0,0.0,0.0,1.0]",
            "affine",
        ),
    ] {
        for kind in ["anchors", "connectors"] {
            let marker = format!(r#"{{ "name": "{name}", "matrix": {matrix} }}"#);
            let (_, reason) = load_marker_error(kind, &marker);
            assert!(
                reason.contains(&format!("nodes[0].extras.scena.{kind}[0].matrix")),
                "matrix error must identify the exact authored field: {reason}"
            );
            assert!(reason.contains(detail), "missing matrix remedy: {reason}");
        }
    }
}

#[test]
fn valid_marker_basis_loads_without_changing_authored_units() {
    let source = marker_document(
        "anchors",
        r#"{
            "name": "basis",
            "translation": [100.0, 0.0, 0.0],
            "forward": [0.0, 1.0, 0.0],
            "up": [-1.0, 0.0, 0.0],
            "units": "centimeters"
        }"#,
    );
    let assets = Assets::with_fetcher(MemoryFetcher::new("memory://c15-valid.gltf", source));
    let asset = pollster::block_on(assets.load_scene("memory://c15-valid.gltf"))
        .expect("valid basis marker loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&asset)
        .expect("valid basis marker instantiates");
    let anchor = import.anchor("basis").expect("basis anchor resolves");

    assert_eq!(anchor.source_units(), SourceUnits::Centimeters);
    assert!(
        anchor.transform().translation.abs_diff_eq(Vec3::X, 1.0e-5),
        "marker translation must retain the established explicit-unit conversion"
    );
}

#[test]
fn valid_marker_matrix_preserves_translation_rotation_and_scale() {
    let source = marker_document(
        "connectors",
        r#"{
            "name": "matrix-mount",
            "kind": "mount",
            "matrix": [
                2.0, 0.0, 0.0, 0.0,
                0.0, 3.0, 0.0, 0.0,
                0.0, 0.0, 4.0, 0.0,
                5.0, 6.0, 7.0, 1.0
            ]
        }"#,
    );
    let assets = Assets::with_fetcher(MemoryFetcher::new("memory://c15-matrix.gltf", source));
    let asset = pollster::block_on(assets.load_scene("memory://c15-matrix.gltf"))
        .expect("valid matrix marker loads");
    let marker = &asset.nodes()[0].connectors()[0];
    let transform = marker.transform();

    assert!(
        transform
            .translation
            .abs_diff_eq(Vec3::new(5.0, 6.0, 7.0), 1.0e-5)
    );
    assert!(
        transform
            .scale
            .abs_diff_eq(Vec3::new(2.0, 3.0, 4.0), 1.0e-5)
    );
    assert!((transform.rotation.length() - 1.0).abs() <= 1.0e-5);
}

#[derive(Clone)]
struct MemoryFetcher {
    sources: Arc<BTreeMap<AssetPath, Vec<u8>>>,
}

impl MemoryFetcher {
    fn new(path: impl Into<AssetPath>, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            sources: Arc::new(BTreeMap::from([(path.into(), bytes.into())])),
        }
    }
}

impl AssetFetcher for MemoryFetcher {
    type Future<'a> = Ready<Result<Vec<u8>, AssetError>>;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        ready(
            self.sources
                .get(path)
                .cloned()
                .ok_or_else(|| AssetError::NotFound {
                    path: path.as_str().to_owned(),
                }),
        )
    }
}
