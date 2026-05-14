use crate::app::prelude::*;

pub(crate) fn check_architecture_contract(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-CONTRACT",
        "docs/api.md",
        &[
            "Scene",
            "Assets",
            "Renderer",
            "SceneImport",
            "Renderer lifecycle",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CONTRACT",
        "README.md",
        &["Scene", "Assets", "Renderer", "Host application"],
    );
}
