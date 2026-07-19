use super::*;

pub(super) struct RecipeBuildFailure {
    pub(super) manifest: SceneRecipeBuildV1,
    pub(super) asset_fetches: u64,
}

impl RecipeBuildFailure {
    pub(super) fn before_host(manifest: SceneRecipeBuildV1) -> Self {
        Self {
            manifest,
            asset_fetches: 0,
        }
    }

    pub(super) fn with_host(
        manifest: SceneRecipeBuildV1,
        host: &SceneHostCore<DefaultAssetFetcher>,
    ) -> Self {
        Self {
            manifest,
            asset_fetches: host.assets.fetch_attempts(),
        }
    }
}

impl SceneHostCore<DefaultAssetFetcher> {
    /// Loads, validates, and executes recipe authoring into SceneHost build
    /// state without constructing renderer, GPU, prepare, render, or capture
    /// state. The returned manifest uses the same handle tables as a host build.
    pub async fn build_recipe_manifest_json(
        recipe_path: impl AsRef<str>,
        text: &str,
        policy: RecipeBuildPolicy,
    ) -> RecipeBuildResultV1 {
        let policy_report = policy.to_schema_report();
        let result = Self::build_recipe_json_with_mode(
            recipe_path,
            text,
            policy,
            RecipeBuildMode::ManifestOnly,
        )
        .await;
        let (manifest, asset_fetches) = match result {
            Ok(build) => {
                let asset_fetches = build.host.assets.fetch_attempts();
                (build.manifest, asset_fetches)
            }
            Err(failure) => (failure.manifest, failure.asset_fetches),
        };
        RecipeBuildResultV1::manifest_only(manifest, policy_report, asset_fetches)
    }
}

pub(super) fn build_manifest(
    diagnostics: Vec<SceneRecipeDiagnosticV1>,
    skipped: Vec<SceneRecipeBuildSkippedV1>,
) -> SceneRecipeBuildV1 {
    SceneRecipeBuildV1 {
        schema: SCENE_RECIPE_BUILD_SCHEMA_V1.to_owned(),
        ok: !has_errors(&diagnostics),
        imports: Vec::new(),
        nodes: Vec::<SceneRecipeBuildTargetV1>::new(),
        instances: Vec::new(),
        cameras: Vec::new(),
        lights: Vec::new(),
        animations: Vec::new(),
        anchors: Vec::new(),
        connectors: Vec::new(),
        connections: Vec::new(),
        bounds: Vec::new(),
        named_states: Vec::new(),
        geometries: Vec::<SceneRecipeBuildResourceV1>::new(),
        materials: Vec::new(),
        fonts: Vec::new(),
        diagnostics,
        skipped,
    }
}

pub(super) fn has_errors(diagnostics: &[SceneRecipeDiagnosticV1]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == "error")
}
