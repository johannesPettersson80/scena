use serde_json::Value as JsonValue;

/// Phase 2B step 1: parse the top-level `KHR_materials_variants.variants`
/// array into an ordered list of variant names. Returns an empty vector
/// when the extension is absent or the entries do not declare a `name`.
/// Anonymous entries (no `name` string) are skipped so the returned
/// indices stay in sync with the on-disk variant order — every per-
/// primitive `mappings[].variants[i]` lookup that step 2 introduces will
/// resolve to the same `material_variants[i]` slot.
pub(super) fn parse_material_variant_names(json: &JsonValue) -> Vec<String> {
    let Some(extension) = json
        .get("extensions")
        .and_then(|extensions| extensions.get("KHR_materials_variants"))
    else {
        return Vec::new();
    };
    extension
        .get("variants")
        .and_then(JsonValue::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    entry
                        .get("name")
                        .and_then(JsonValue::as_str)
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::parse_material_variant_names;
    use serde_json::json;

    #[test]
    fn parser_returns_empty_when_extension_absent() {
        let asset = json!({
            "asset": { "version": "2.0" },
        });
        assert!(parse_material_variant_names(&asset).is_empty());
    }

    #[test]
    fn parser_reads_variant_names_in_declaration_order() {
        // Phase 2B step 1: KHR_materials_variants.variants is an ordered
        // array of {name: String} objects. The parser must return the
        // names in declaration order so per-primitive
        // `mappings[].variants[i]` lookups in step 2 resolve to
        // `material_variants[i]`.
        let asset = json!({
            "asset": { "version": "2.0" },
            "extensions": {
                "KHR_materials_variants": {
                    "variants": [
                        { "name": "midnight" },
                        { "name": "noon" },
                        { "name": "twilight" },
                    ],
                },
            },
        });
        assert_eq!(
            parse_material_variant_names(&asset),
            vec![
                "midnight".to_string(),
                "noon".to_string(),
                "twilight".to_string(),
            ],
        );
    }

    #[test]
    fn parser_skips_anonymous_entries_so_named_indices_stay_dense() {
        // glTF requires variant entries to declare a `name`, but a future
        // tool that emits anonymous entries should not silently drift the
        // `material_variants[i]` indices the per-primitive bindings will
        // reference. Drop anonymous entries.
        let asset = json!({
            "asset": { "version": "2.0" },
            "extensions": {
                "KHR_materials_variants": {
                    "variants": [
                        { "name": "first" },
                        { "missing": "no_name_here" },
                        { "name": "third" },
                    ],
                },
            },
        });
        assert_eq!(
            parse_material_variant_names(&asset),
            vec!["first".to_string(), "third".to_string()],
        );
    }

    #[test]
    fn parser_returns_empty_for_malformed_variants_array() {
        let asset = json!({
            "asset": { "version": "2.0" },
            "extensions": {
                "KHR_materials_variants": { "variants": "not an array" },
            },
        });
        assert!(parse_material_variant_names(&asset).is_empty());
    }
}
