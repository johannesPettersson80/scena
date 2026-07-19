use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use super::types::SceneRecipeDiagnosticV1;

mod report;
mod sandbox;
pub use report::{
    RecipeBuildPolicyBoolV1, RecipeBuildPolicyLimitV1, RecipeBuildPolicyReportV1,
    RecipeBuildPolicyRootV1, RecipeBuildPolicyStringV1,
};

const DEFAULT_MAX_IMPORTS: usize = 64;
const DEFAULT_MAX_NODES: usize = 10_000;
const DEFAULT_MAX_VERTICES: usize = 2_000_000;
const DEFAULT_MAX_INDICES: usize = 6_000_000;
const DEFAULT_MAX_MATERIALS: usize = 2_000;
const DEFAULT_MAX_TEXTURES: usize = 256;
const DEFAULT_MAX_TEXTURE_BYTES: usize = 64 * 1024 * 1024;
const DEFAULT_MAX_IMAGE_DIMENSION: u32 = 8192;
const DEFAULT_MAX_INSTANCES: usize = 100_000;
const DEFAULT_MAX_PARTICLES: usize = 100_000;
const DEFAULT_MAX_ANIMATIONS: usize = 4_000;
const DEFAULT_MAX_ANIMATION_CHANNELS: usize = 100_000;
const DEFAULT_MAX_ANIMATION_KEYFRAMES: usize = 2_000_000;
const DEFAULT_MAX_OUTPUT_PIXELS: u64 = 4096 * 4096;
const DEFAULT_FETCH_BYTE_LIMIT: usize = 64 * 1024 * 1024;
const DEFAULT_MAX_RECIPE_BYTES: usize = 8 * 1024 * 1024;
const OVERRIDE_MAX_IMPORTS: u32 = 1 << 0;
const OVERRIDE_MAX_NODES: u32 = 1 << 1;
const OVERRIDE_MAX_VERTICES: u32 = 1 << 2;
const OVERRIDE_MAX_INDICES: u32 = 1 << 3;
const OVERRIDE_MAX_MATERIALS: u32 = 1 << 4;
const OVERRIDE_MAX_TEXTURES: u32 = 1 << 5;
const OVERRIDE_MAX_TEXTURE_BYTES: u32 = 1 << 6;
const OVERRIDE_MAX_IMAGE_DIMENSION: u32 = 1 << 7;
const OVERRIDE_MAX_INSTANCES: u32 = 1 << 8;
const OVERRIDE_MAX_PARTICLES: u32 = 1 << 9;
const OVERRIDE_MAX_ANIMATIONS: u32 = 1 << 10;
const OVERRIDE_MAX_ANIMATION_CHANNELS: u32 = 1 << 11;
const OVERRIDE_MAX_ANIMATION_KEYFRAMES: u32 = 1 << 12;
const OVERRIDE_MAX_OUTPUT_PIXELS: u32 = 1 << 13;
const OVERRIDE_FETCH_BYTE_LIMIT: u32 = 1 << 14;
const OVERRIDE_MAX_RECIPE_BYTES: u32 = 1 << 15;
const OVERRIDE_NETWORK: u32 = 1 << 16;
const OVERRIDE_URI_SCHEMES: u32 = 1 << 17;
const OVERRIDE_ROOTS: u32 = 1 << 18;

pub const RECIPE_POLICY_SCHEMA_V1: &str = "scena.recipe_policy.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeBuildPolicy {
    max_imports: usize,
    max_nodes: usize,
    max_vertices: usize,
    max_indices: usize,
    max_materials: usize,
    max_textures: usize,
    max_texture_bytes: usize,
    max_image_dimension: u32,
    max_instances: usize,
    max_particles: usize,
    max_animations: usize,
    max_animation_channels: usize,
    max_animation_keyframes: usize,
    max_output_pixels: u64,
    fetch_byte_limit: usize,
    max_recipe_bytes: usize,
    allow_network: bool,
    allowed_uri_schemes: BTreeSet<String>,
    allowed_roots: Vec<PathBuf>,
    override_mask: u32,
}

impl Default for RecipeBuildPolicy {
    fn default() -> Self {
        let mut allowed_uri_schemes = BTreeSet::new();
        allowed_uri_schemes.insert("file".to_owned());
        Self {
            max_imports: DEFAULT_MAX_IMPORTS,
            max_nodes: DEFAULT_MAX_NODES,
            max_vertices: DEFAULT_MAX_VERTICES,
            max_indices: DEFAULT_MAX_INDICES,
            max_materials: DEFAULT_MAX_MATERIALS,
            max_textures: DEFAULT_MAX_TEXTURES,
            max_texture_bytes: DEFAULT_MAX_TEXTURE_BYTES,
            max_image_dimension: DEFAULT_MAX_IMAGE_DIMENSION,
            max_instances: DEFAULT_MAX_INSTANCES,
            max_particles: DEFAULT_MAX_PARTICLES,
            max_animations: DEFAULT_MAX_ANIMATIONS,
            max_animation_channels: DEFAULT_MAX_ANIMATION_CHANNELS,
            max_animation_keyframes: DEFAULT_MAX_ANIMATION_KEYFRAMES,
            max_output_pixels: DEFAULT_MAX_OUTPUT_PIXELS,
            fetch_byte_limit: DEFAULT_FETCH_BYTE_LIMIT,
            max_recipe_bytes: DEFAULT_MAX_RECIPE_BYTES,
            allow_network: false,
            allowed_uri_schemes,
            allowed_roots: default_allowed_roots(),
            override_mask: 0,
        }
    }
}

impl RecipeBuildPolicy {
    /// Returns the default deny-network, local-root sandbox used by tests and examples.
    ///
    /// The policy is operator-owned: recipes cannot raise these limits. `testing()` is an
    /// alias for `default()` so test and CLI paths exercise the same sandbox defaults.
    pub fn testing() -> Self {
        Self::default()
    }

    fn source_for(&self, bit: u32) -> &'static str {
        if self.override_mask & bit == 0 {
            "compiled_default"
        } else {
            "operator_override"
        }
    }

    /// Maximum number of imports allowed in one recipe.
    pub const fn max_imports(&self) -> usize {
        self.max_imports
    }

    /// Maximum aggregate scene nodes allowed after imports and authored nodes are built.
    pub const fn max_nodes(&self) -> usize {
        self.max_nodes
    }

    /// Maximum aggregate vertex count allowed across imported and authored geometry.
    pub const fn max_vertices(&self) -> usize {
        self.max_vertices
    }

    /// Maximum aggregate index count allowed across imported and authored geometry.
    pub const fn max_indices(&self) -> usize {
        self.max_indices
    }

    /// Maximum authored/imported material count allowed in one build.
    pub const fn max_materials(&self) -> usize {
        self.max_materials
    }

    /// Maximum texture count allowed in one build.
    pub const fn max_textures(&self) -> usize {
        self.max_textures
    }

    /// Maximum decoded texture memory budget in bytes.
    pub const fn max_texture_bytes(&self) -> usize {
        self.max_texture_bytes
    }

    /// Maximum allowed width or height for a decoded image.
    pub const fn max_image_dimension(&self) -> u32 {
        self.max_image_dimension
    }

    /// Maximum authored/imported instance count allowed in one build.
    pub const fn max_instances(&self) -> usize {
        self.max_instances
    }

    /// Maximum authored particle count allowed in one build.
    pub const fn max_particles(&self) -> usize {
        self.max_particles
    }

    /// Maximum authored animation clips allowed in one build.
    pub const fn max_animations(&self) -> usize {
        self.max_animations
    }

    /// Maximum aggregate authored animation channels allowed in one build.
    pub const fn max_animation_channels(&self) -> usize {
        self.max_animation_channels
    }

    /// Maximum aggregate authored animation keyframes allowed in one build.
    pub const fn max_animation_keyframes(&self) -> usize {
        self.max_animation_keyframes
    }

    /// Maximum requested capture size, in pixels.
    pub const fn max_output_pixels(&self) -> u64 {
        self.max_output_pixels
    }

    /// Maximum source bytes fetched for one local resource.
    pub const fn fetch_byte_limit(&self) -> usize {
        self.fetch_byte_limit
    }

    /// Maximum scene recipe JSON document size, in bytes.
    pub const fn max_recipe_bytes(&self) -> usize {
        self.max_recipe_bytes
    }

    /// Whether HTTP(S) resources are allowed.
    pub const fn allow_network(&self) -> bool {
        self.allow_network
    }

    /// URI schemes accepted by the recipe executor.
    pub fn allowed_uri_schemes(&self) -> &BTreeSet<String> {
        &self.allowed_uri_schemes
    }

    /// Canonical local roots that local file resources must stay under.
    ///
    /// Empty or uncanonicalizable root sets deny local file resources. There is no implicit
    /// unsandboxed mode.
    pub fn allowed_roots(&self) -> &[PathBuf] {
        &self.allowed_roots
    }

    pub const fn with_max_imports(mut self, max_imports: usize) -> Self {
        self.max_imports = max_imports;
        self.override_mask |= OVERRIDE_MAX_IMPORTS;
        self
    }

    pub const fn with_max_nodes(mut self, max_nodes: usize) -> Self {
        self.max_nodes = max_nodes;
        self.override_mask |= OVERRIDE_MAX_NODES;
        self
    }

    pub const fn with_max_vertices(mut self, max_vertices: usize) -> Self {
        self.max_vertices = max_vertices;
        self.override_mask |= OVERRIDE_MAX_VERTICES;
        self
    }

    pub const fn with_max_indices(mut self, max_indices: usize) -> Self {
        self.max_indices = max_indices;
        self.override_mask |= OVERRIDE_MAX_INDICES;
        self
    }

    pub const fn with_max_materials(mut self, max_materials: usize) -> Self {
        self.max_materials = max_materials;
        self.override_mask |= OVERRIDE_MAX_MATERIALS;
        self
    }

    pub const fn with_max_textures(mut self, max_textures: usize) -> Self {
        self.max_textures = max_textures;
        self.override_mask |= OVERRIDE_MAX_TEXTURES;
        self
    }

    pub const fn with_max_texture_bytes(mut self, max_texture_bytes: usize) -> Self {
        self.max_texture_bytes = max_texture_bytes;
        self.override_mask |= OVERRIDE_MAX_TEXTURE_BYTES;
        self
    }

    pub const fn with_max_image_dimension(mut self, max_image_dimension: u32) -> Self {
        self.max_image_dimension = max_image_dimension;
        self.override_mask |= OVERRIDE_MAX_IMAGE_DIMENSION;
        self
    }

    pub const fn with_max_instances(mut self, max_instances: usize) -> Self {
        self.max_instances = max_instances;
        self.override_mask |= OVERRIDE_MAX_INSTANCES;
        self
    }

    pub const fn with_max_particles(mut self, max_particles: usize) -> Self {
        self.max_particles = max_particles;
        self.override_mask |= OVERRIDE_MAX_PARTICLES;
        self
    }

    pub const fn with_max_animations(mut self, max_animations: usize) -> Self {
        self.max_animations = max_animations;
        self.override_mask |= OVERRIDE_MAX_ANIMATIONS;
        self
    }

    pub const fn with_max_animation_channels(mut self, max_animation_channels: usize) -> Self {
        self.max_animation_channels = max_animation_channels;
        self.override_mask |= OVERRIDE_MAX_ANIMATION_CHANNELS;
        self
    }

    pub const fn with_max_animation_keyframes(mut self, max_animation_keyframes: usize) -> Self {
        self.max_animation_keyframes = max_animation_keyframes;
        self.override_mask |= OVERRIDE_MAX_ANIMATION_KEYFRAMES;
        self
    }

    pub const fn with_max_output_pixels(mut self, max_output_pixels: u64) -> Self {
        self.max_output_pixels = max_output_pixels;
        self.override_mask |= OVERRIDE_MAX_OUTPUT_PIXELS;
        self
    }

    pub const fn with_fetch_byte_limit(mut self, fetch_byte_limit: usize) -> Self {
        self.fetch_byte_limit = fetch_byte_limit;
        self.override_mask |= OVERRIDE_FETCH_BYTE_LIMIT;
        self
    }

    pub const fn with_max_recipe_bytes(mut self, max_recipe_bytes: usize) -> Self {
        self.max_recipe_bytes = max_recipe_bytes;
        self.override_mask |= OVERRIDE_MAX_RECIPE_BYTES;
        self
    }

    pub const fn with_allow_network(mut self, allow_network: bool) -> Self {
        self.allow_network = allow_network;
        self.override_mask |= OVERRIDE_NETWORK;
        self
    }

    pub fn with_allowed_uri_schemes<I, S>(mut self, schemes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_uri_schemes = schemes
            .into_iter()
            .map(Into::into)
            .map(|scheme| scheme.to_ascii_lowercase())
            .collect();
        self.override_mask |= OVERRIDE_URI_SCHEMES;
        self
    }

    pub fn with_allowed_roots(mut self, roots: impl IntoIterator<Item = PathBuf>) -> Self {
        self.allowed_roots = roots.into_iter().collect();
        self.override_mask |= OVERRIDE_ROOTS;
        self
    }
}

fn strip_file_scheme(
    uri: &str,
    diagnostic_path: String,
) -> Result<&str, Box<SceneRecipeDiagnosticV1>> {
    let Some(rest) = uri
        .strip_prefix("file://")
        .or_else(|| uri.strip_prefix("FILE://"))
    else {
        return Ok(uri);
    };
    if !rest.starts_with('/') {
        return Err(Box::new(policy_error(
            diagnostic_path,
            "file URI authorities are not allowed by RecipeBuildPolicy",
            "use a local path or a file:/// absolute path without an authority",
        )));
    }
    Ok(rest)
}

pub(crate) fn resolve_recipe_asset_uri(recipe_path: &str, uri: &str) -> String {
    let uri_path = Path::new(uri);
    if uri_path.is_absolute() || uri.contains("://") || uri.starts_with("data:") {
        return uri.to_owned();
    }
    let relative_to_recipe = Path::new(recipe_path)
        .parent()
        .map(|parent| parent.join(uri));
    if let Some(path) = relative_to_recipe.filter(|path| path.exists()) {
        return path.display().to_string();
    }
    uri.to_owned()
}

pub(crate) fn build_diagnostic(
    code: impl Into<String>,
    severity: impl Into<String>,
    path: impl Into<String>,
    message: impl Into<String>,
    help: impl Into<String>,
    suggestion: Option<String>,
    auto_fixable: bool,
) -> SceneRecipeDiagnosticV1 {
    SceneRecipeDiagnosticV1 {
        code: code.into(),
        severity: severity.into(),
        path: path.into(),
        message: message.into(),
        help: help.into(),
        suggestion,
        auto_fixable,
    }
}

fn policy_error(
    path: impl Into<String>,
    message: impl Into<String>,
    help: impl Into<String>,
) -> SceneRecipeDiagnosticV1 {
    build_diagnostic(
        "policy_violation",
        "error",
        path,
        message,
        help,
        None,
        false,
    )
}

fn uri_scheme(uri: &str) -> Option<&str> {
    let colon = uri.find(':')?;
    let candidate = &uri[..colon];
    let mut chars = candidate.chars();
    if !chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
    {
        return None;
    }
    if !chars
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.'))
    {
        return None;
    }
    Some(candidate)
}

fn has_parent_dir(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::ParentDir))
}

fn default_allowed_roots() -> Vec<PathBuf> {
    std::env::current_dir().ok().into_iter().collect::<Vec<_>>()
}

fn stable_canonical_path(path: &Path) -> String {
    let current_dir = std::env::current_dir().ok();
    if let Some(current_dir) = current_dir.as_deref()
        && let Ok(relative) = path.strip_prefix(current_dir)
        && !relative.as_os_str().is_empty()
    {
        return relative.display().to_string();
    }
    path.display().to_string()
}

fn format_allowed_roots(roots: &[PathBuf]) -> String {
    if roots.is_empty() {
        return "<none>".to_owned();
    }
    roots
        .iter()
        .map(|root| root.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
