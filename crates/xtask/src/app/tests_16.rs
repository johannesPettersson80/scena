use crate::app::prelude::*;
use crate::app::tests_12::{VALID_GUIDE, write_easy_scene_fixture};

pub(crate) fn write_scena_viewer_element_easy_scene_fixture(fixture_root: &Path) {
    fs::write(
        fixture_root.join("Cargo.toml"),
        format!(
            "{}\nviewer-element = []",
            fs::read_to_string(fixture_root.join("Cargo.toml")).expect("manifest fixture")
        ),
    )
    .expect("manifest viewer-element fixture");
    fs::write(
        fixture_root.join("src/lib.rs"),
        format!(
            "{} pub mod viewer_element; SCENA_VIEWER_TAG ScenaViewerAttributes define_scena_viewer",
            fs::read_to_string(fixture_root.join("src/lib.rs")).expect("lib fixture")
        ),
    )
    .expect("lib viewer-element fixture");
    fs::write(
        fixture_root.join("src/viewer_element.rs"),
        "pub const SCENA_VIEWER_TAG pub struct ScenaViewerAttributes from_pairs defineScenaViewerElement customElements.define attachShadow observedAttributes scena-viewer-ready",
    )
    .expect("viewer element fixture");
    fs::write(
        fixture_root.join("tests/scena_viewer_element.rs"),
        "scena_viewer_attributes_parse_model_viewer_style_surface scena_viewer_attributes_default_to_safe_drop_in_viewer_values camera-controls tone-mapping",
    )
    .expect("viewer element test fixture");
    fs::write(
        fixture_root.join("docs/browser.md"),
        "<scena-viewer defineScenaViewer viewer-element shadow DOM canvas",
    )
    .expect("browser docs fixture");
    let checklist_path =
        fixture_root.join("docs/checklists/next-release-easy-use-and-state-of-the-art.md");
    let mut checklist =
        fs::read_to_string(&checklist_path).expect("next release checklist fixture");
    checklist.push_str(
        " custom-element\nfoundation **[shipped]** src/viewer_element.rs SCENA-VIEWER-ELEMENT Full\n  asset loading/rendering parity remains open under bet 1.1",
    );
    fs::write(checklist_path, checklist).expect("next release checklist viewer element fixture");
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_scena_viewer_element_foundation() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/missing-scena-viewer-element");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    let _ = fs::remove_file(fixture_root.join("src/viewer_element.rs"));
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "SCENA-VIEWER-ELEMENT"),
        "doctor must reject <scena-viewer> foundation claims without the source, docs, and tests: {findings:?}",
    );
}
