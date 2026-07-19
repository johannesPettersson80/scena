pub(super) struct SchemaEntryRow {
    pub(super) schema: &'static str,
    pub(super) owner_module: &'static str,
    pub(super) summary: &'static str,
    pub(super) feature_flag: Option<&'static str>,
    pub(super) fixture_path: Option<&'static str>,
}
