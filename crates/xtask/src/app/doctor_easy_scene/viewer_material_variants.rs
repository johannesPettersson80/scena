use crate::app::prelude::*;

pub(super) fn check_viewer_material_variants(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "VIEWER-MATERIAL-VARIANTS",
        "src/viewer.rs",
        &["mod material_variants;"],
    );
    require_contains(
        root,
        findings,
        "VIEWER-MATERIAL-VARIANTS",
        "src/viewer/material_variants.rs",
        &[
            "pub fn material_variants(&self) -> &[String]",
            "pub fn active_material_variant(&self) -> Option<String>",
            "pub fn set_active_material_variant(&mut self, name: Option<&str>) -> crate::Result<()>",
            "self.scene.set_active_variant(&self.import, name)?",
            "self.prepare()",
        ],
    );
    require_contains(
        root,
        findings,
        "VIEWER-MATERIAL-VARIANTS",
        "tests/assets/gltf/material_variants_scene.gltf",
        &["KHR_materials_variants", "\"midnight\"", "\"noon\""],
    );
    require_contains(
        root,
        findings,
        "VIEWER-MATERIAL-VARIANTS",
        "tests/first_render_api.rs",
        &[
            "headless_gltf_viewer_switches_material_variants_and_reprepares",
            "material_variants_scene.gltf",
            "viewer.material_variants()",
            "set_active_material_variant(Some(\"midnight\"))",
            "viewer.active_material_variant()",
        ],
    );
    require_contains(
        root,
        findings,
        "VIEWER-MATERIAL-VARIANTS",
        "tests/m7_interactive_viewer.rs",
        &[
            "interactive_gltf_viewer_switches_material_variants_and_reprepares",
            "material_variants_scene.gltf",
            "viewer.material_variants()",
            "set_active_material_variant(Some(\"noon\"))",
            "viewer.active_material_variant()",
        ],
    );
    require_contains(
        root,
        findings,
        "VIEWER-MATERIAL-VARIANTS",
        "tests/examples_visual_proof.rs",
        &[
            "viewer_material_variant_reference_docs_image",
            "viewer-material-variant-reference-docs-image",
            "reference-image+docs-image",
        ],
    );
    require_contains(
        root,
        findings,
        "VIEWER-MATERIAL-VARIANTS",
        "docs/guides/easy-scene-setup.md",
        &[
            "Material variants",
            "viewer.material_variants()",
            "viewer.set_active_material_variant(Some(\"blue\"))?",
            "viewer.set_active_material_variant(None)?",
        ],
    );
}
