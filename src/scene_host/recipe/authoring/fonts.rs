use std::collections::BTreeMap;
use std::fs;

use crate::scene::recipe::{
    RecipeBuildPolicy, SceneRecipeBuildResourceV1, SceneRecipeBuildSkippedV1,
    SceneRecipeDiagnosticV1, SceneRecipeFontV1, build_diagnostic,
};
use crate::{LabelFontError, LabelFontFace};

use super::super::error_diagnostic;

pub(in crate::scene_host::recipe) fn build_authored_fonts(
    policy: &RecipeBuildPolicy,
    recipe_path: &str,
    recipes: &[SceneRecipeFontV1],
    manifest: &mut Vec<SceneRecipeBuildResourceV1>,
    skipped: &mut Vec<SceneRecipeBuildSkippedV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> BTreeMap<String, LabelFontFace> {
    let mut fonts = BTreeMap::new();
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.fonts[{index}]");
        let resolved =
            match policy.resolve_import_uri(recipe_path, &recipe.uri, format!("{path}.uri")) {
                Ok(uri) => uri,
                Err(diagnostic) => {
                    diagnostics.push(*diagnostic);
                    continue;
                }
            };
        let bytes = match fs::read(&resolved) {
            Ok(bytes) => bytes,
            Err(error) if recipe.optional => {
                diagnostics.push(build_diagnostic(
                    "optional_font_skipped",
                    "warning",
                    &path,
                    format!("optional font '{}' could not be loaded: {error}", recipe.id),
                    "the font was marked optional, so the build continues without it",
                    None,
                    false,
                ));
                skipped.push(SceneRecipeBuildSkippedV1 {
                    path,
                    id: recipe.id.clone(),
                    kind: "font".to_owned(),
                    reason: error.to_string(),
                });
                continue;
            }
            Err(error) => {
                diagnostics.push(error_diagnostic(
                    &path,
                    "font_load_failed",
                    format!("required font '{}' could not be loaded: {error}", recipe.id),
                    "fix the font uri or mark it optional only when no label references it",
                ));
                continue;
            }
        };
        if bytes.len() > policy.fetch_byte_limit() {
            diagnostics.push(error_diagnostic(
                format!("{path}.uri"),
                "policy_violation",
                format!(
                    "font fetched {} bytes, exceeding RecipeBuildPolicy fetch_byte_limit {}",
                    bytes.len(),
                    policy.fetch_byte_limit()
                ),
                "use a smaller font or raise the operator-owned fetch_byte_limit policy",
            ));
            continue;
        }
        let font = match LabelFontFace::from_truetype_bytes(&bytes) {
            Ok(font) => font,
            Err(LabelFontError::InvalidFont { reason }) if recipe.optional => {
                diagnostics.push(build_diagnostic(
                    "optional_font_skipped",
                    "warning",
                    &path,
                    format!(
                        "optional font '{}' could not be parsed: {reason}",
                        recipe.id
                    ),
                    "the font was marked optional, so the build continues without it",
                    None,
                    false,
                ));
                skipped.push(SceneRecipeBuildSkippedV1 {
                    path,
                    id: recipe.id.clone(),
                    kind: "font".to_owned(),
                    reason,
                });
                continue;
            }
            Err(error) => {
                diagnostics.push(error_diagnostic(
                    &path,
                    "font_load_failed",
                    format!("required font '{}' could not be parsed: {error}", recipe.id),
                    "use a valid TrueType/OpenType font file",
                ));
                continue;
            }
        };
        fonts.insert(recipe.id.clone(), font);
        manifest.push(SceneRecipeBuildResourceV1 {
            id: recipe.id.clone(),
            kind: "truetype_font".to_owned(),
            vertex_count: None,
            index_count: None,
        });
    }
    fonts
}
