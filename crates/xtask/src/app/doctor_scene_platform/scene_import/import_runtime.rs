use crate::app::prelude::*;

pub(crate) fn check_m3a_import_runtime_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/import/load.rs",
        &[
            "pub async fn import<",
            "pub async fn import_with<",
            "pub fn replace_import(",
            "mark_stale",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/import.rs",
        &[
            "pub struct ImportOptions",
            "pub enum SourceUnits",
            "pub enum SourceCoordinateSystem",
            "Centimeters",
            "Inches",
            "Feet",
            "ZUpRightHanded",
            "pub struct SceneImport",
            "pub struct ImportAnchor",
            "pub struct ImportConnector",
            "pub struct ImportClip",
            "pub struct ImportPivot",
            "node_bounds",
            "source_node.meshes()",
            "scene_asset: &SceneAsset",
            "InvalidAnchorExtras",
            "convert_marker_units(",
            "placement_node",
            "placement_transform",
            "root_from_node",
            "convert_transform(source_node.transform())",
            "ImportDiagnosticOverlayKind::Origin",
            "ImportDiagnosticOverlayKind::Axes",
            "ImportDiagnosticOverlayKind::Bounds",
            "ImportDiagnosticOverlayKind::Anchor",
            "ImportDiagnosticOverlayKind::Connector",
            "ImportDiagnosticOverlayKind::Pivot",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/import/instantiate.rs",
        &[
            "pub fn instantiate(",
            "pub fn instantiate_with(",
            "pub fn instantiate_under(",
            "self.instantiate_with_parent(",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/import/handedness.rs",
        &[
            "reject_unproven_left_handed_mesh_import",
            "has_negative_determinant",
            "UnsupportedCoordinateSystem",
            "left-handed mesh imports require explicit winding and normal correction proof",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/import/types.rs",
        &["ImportBuild", "NodeKind::Mesh", "mesh_node_kind"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/import/options.rs",
        &[
            "pub const fn gltf_default() -> Self",
            "pub const fn with_source_units",
            "pub const fn with_source_coordinate_system",
            "pub(super) fn convert_transform",
            "convert_connector_transform(transform)",
            "AnimationTarget::Translation",
            "AnimationTarget::Rotation",
            "AnimationTarget::Scale",
            "AnimationTarget::Weights",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/import/accessors.rs",
        &[
            "pub const fn placement_node",
            "self.placement_transform",
            "pub fn channels(&self)",
            "pub const fn duration_seconds",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/import/lookups.rs",
        &[
            "LookupError::StaleImport",
            "pub fn node(&self, name: &str)",
            "pub fn first_node(&self, name: &str)",
            "pub fn nodes_named",
            "pub fn path(&self, path: &str)",
            "pub fn bounds_local",
            "pub fn bounds_world",
            "scene.world_transform(record.node)",
            "pub fn pivot(&self",
            "pub fn diagnostic_overlays",
            "pub fn anchor(&self",
            "pub fn replacement_anchor",
            "pub fn connector(&self",
            "pub fn replacement_connector",
            "pub fn connectors_named",
            "pub fn first_anchor",
            "pub fn anchors_named",
            "pub fn clip(&self",
            "pub fn first_clip",
            "pub fn clips_named",
            "AmbiguousAnchorName",
            "AmbiguousClipName",
            "fn path_segments(path: &str)",
        ],
    );
    check_scene_import_transaction_contracts(root, findings);
}

fn check_scene_import_transaction_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "SCENE-IMPORT-TRANSACTION",
        "src/scene/transaction.rs",
        &[
            "pub(super) struct SceneTransaction",
            "snapshot: Option<Scene>",
            "fn transaction_snapshot",
            "self.scene.structure_revision",
            "self.scene.transform_revision",
            "self.scene.appearance_revision",
            "self.scene.visibility_revision",
            "*self.scene = snapshot",
            "nodes: self.nodes.clone()",
            "lights: self.lights.clone()",
            "instance_sets: self.instance_sets.clone()",
            "animation_mixers: self.animation_mixers.clone()",
            "anchors: self.anchors.clone()",
            "retired_anchors: self.retired_anchors.clone()",
            "connectors: self.connectors.clone()",
            "retired_connectors: self.retired_connectors.clone()",
            "node_bounds: self.node_bounds.clone()",
            "morph_weights: self.morph_weights.clone()",
            "skin_bindings: self.skin_bindings.clone()",
        ],
    );
    require_contains(
        root,
        findings,
        "SCENE-IMPORT-TRANSACTION",
        "src/scene/import/prevalidation.rs",
        &[
            "validate_scene_asset_for_instantiation",
            "InvalidChildIndex",
            "MultipleNodeParents",
            "CyclicNodeGraph",
            "InvalidAnchorExtras",
            "InvalidConnectorExtras",
            "InvalidSkinIndex",
            "InvalidSkinJointIndex",
        ],
    );
    require_contains(
        root,
        findings,
        "SCENE-IMPORT-TRANSACTION",
        "src/scene/import/load.rs",
        &[
            "ForeignReplacementImport",
            "SceneTransaction::new(self)",
            "remove_nodes_unchecked",
            "transaction.commit()",
            "import.mark_stale()",
        ],
    );
    forbid_contains(
        root,
        findings,
        "SCENE-IMPORT-TRANSACTION",
        "src/scene/import/load.rs",
        &["import.mark_stale();\n        self.instantiate_with"],
    );
    require_replace_import_lifecycle_order(root, findings);
    require_contains(
        root,
        findings,
        "SCENE-IMPORT-TRANSACTION",
        "src/scene/removal.rs",
        &[
            "ImportFromDifferentScene",
            "SceneTransaction::new(self)",
            "transaction.commit()",
            "import.mark_stale()",
            "frame.import_retirement_name()",
            "self.retired_anchors",
            "self.retired_connectors",
        ],
    );
    require_contains(
        root,
        findings,
        "SCENE-IMPORT-TRANSACTION",
        "src/scene/import/transaction_tests.rs",
        &[
            "repeated_replace_import_is_bounded_and_removes_every_old_root",
            "failed_create_is_exact_noop_at_every_late_failure_family",
            "failed_replace_is_exact_noop_at_every_late_failure_family",
            "prevalidation_rejects_cycles_and_multiple_parents_without_scene_mutation",
            "remove_import_is_atomic_for_multiple_roots_and_rejects_foreign_scene",
            "replacement_uses_fresh_runtime_overrides_and_commits_one_revision_boundary",
            "removed_import_anchor_and_connector_handles_remain_stale_without_live_registry_growth",
            "direct_anchor_and_connector_removal_remains_missing_not_import_stale",
        ],
    );
}

fn require_replace_import_lifecycle_order(root: &Path, findings: &mut Vec<Finding>) {
    const REL: &str = "src/scene/import/load.rs";
    const ORDERED_STEPS: &[&str] = &[
        "let mut transaction = SceneTransaction::new(self);",
        ".instantiate_with(scene_asset, options)?",
        "remove_nodes_unchecked",
        "transaction.commit();",
        "import.mark_stale();",
        "Ok(replacement)",
    ];

    let Ok(text) = fs::read_to_string(root.join(REL)) else {
        return;
    };
    let Some(function_start) = text.find("pub fn replace_import(") else {
        return;
    };
    let mut cursor = function_start;
    for step in ORDERED_STEPS {
        let Some(offset) = text[cursor..].find(step) else {
            findings.push(Finding::new(
                "SCENE-IMPORT-TRANSACTION",
                format!(
                    "{REL} replace_import lifecycle order is invalid: expected '{step}' after the prior transaction step"
                ),
            ));
            return;
        };
        cursor += offset + step.len();
    }
}
