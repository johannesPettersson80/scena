use crate::Tonemapper;
use crate::assets::AssetLoadProgress;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenaViewerAttributes {
    src: Option<String>,
    environment: Option<String>,
    tonemapper: Tonemapper,
    camera_controls: bool,
    auto_rotate: bool,
    ar: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScenaViewerAccessibilityDefaults {
    host_role: &'static str,
    host_label: &'static str,
    canvas_label: &'static str,
    min_width_px: u32,
    min_height_px: u32,
    touch_action: &'static str,
    host_keyboard_focusable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenaViewerKeyboardAction {
    OrbitLeft,
    OrbitRight,
    OrbitUp,
    OrbitDown,
    ZoomIn,
    ZoomOut,
    ResetView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenaViewerGestureAction {
    Orbit,
    PinchZoom,
    WheelZoom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenaViewerDropKind {
    Glb,
    Gltf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenaViewerDroppedFile {
    name: String,
    kind: ScenaViewerDropKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScenaViewerDropDecision {
    accepted: Vec<ScenaViewerDroppedFile>,
    rejected: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenaViewerVariantOption {
    name: String,
    label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScenaViewerVariantSelection {
    options: Vec<ScenaViewerVariantOption>,
    active: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenaViewerProgressPhase {
    Idle,
    Loading,
    Fetching,
    Parsing,
    Caching,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenaViewerProgress {
    phase: ScenaViewerProgressPhase,
    path: Option<String>,
    loaded_bytes: Option<usize>,
    external_buffer_index: Option<usize>,
    nodes: Option<usize>,
    meshes: Option<usize>,
}

impl Default for ScenaViewerAttributes {
    fn default() -> Self {
        Self {
            src: None,
            environment: None,
            tonemapper: Tonemapper::PbrNeutral,
            camera_controls: false,
            auto_rotate: false,
            ar: false,
        }
    }
}

impl ScenaViewerAttributes {
    pub fn from_pairs<I, K, V>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut attributes = Self::default();
        for (name, value) in pairs {
            attributes.set_attribute(name.as_ref(), value.as_ref());
        }
        attributes
    }

    pub fn set_attribute(&mut self, name: &str, value: &str) {
        match name {
            "src" => self.src = non_empty_string(value),
            "environment" => self.environment = non_empty_string(value),
            "tone-mapping" | "tone_mapping" => self.tonemapper = parse_tonemapper(value),
            "camera-controls" | "camera_controls" => {
                self.camera_controls = parse_boolean_attribute(name, value);
            }
            "auto-rotate" | "auto_rotate" => {
                self.auto_rotate = parse_boolean_attribute(name, value);
            }
            "ar" => self.ar = parse_boolean_attribute(name, value),
            _ => {}
        }
    }

    pub fn src(&self) -> Option<&str> {
        self.src.as_deref()
    }

    pub fn environment(&self) -> Option<&str> {
        self.environment.as_deref()
    }

    pub const fn tonemapper(&self) -> Tonemapper {
        self.tonemapper
    }

    pub const fn camera_controls(&self) -> bool {
        self.camera_controls
    }

    pub const fn auto_rotate(&self) -> bool {
        self.auto_rotate
    }

    pub const fn ar(&self) -> bool {
        self.ar
    }
}

impl Default for ScenaViewerAccessibilityDefaults {
    fn default() -> Self {
        Self {
            host_role: "img",
            host_label: "3D model viewer",
            canvas_label: "scena 3D viewer canvas",
            min_width_px: 160,
            min_height_px: 120,
            touch_action: "none",
            host_keyboard_focusable: true,
        }
    }
}

impl ScenaViewerAccessibilityDefaults {
    pub const fn host_role(&self) -> &'static str {
        self.host_role
    }

    pub const fn host_label(&self) -> &'static str {
        self.host_label
    }

    pub const fn canvas_label(&self) -> &'static str {
        self.canvas_label
    }

    pub const fn min_width_px(&self) -> u32 {
        self.min_width_px
    }

    pub const fn min_height_px(&self) -> u32 {
        self.min_height_px
    }

    pub const fn touch_action(&self) -> &'static str {
        self.touch_action
    }

    pub const fn host_is_keyboard_focusable(&self) -> bool {
        self.host_keyboard_focusable
    }
}

impl ScenaViewerKeyboardAction {
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "ArrowLeft" => Some(Self::OrbitLeft),
            "ArrowRight" => Some(Self::OrbitRight),
            "ArrowUp" => Some(Self::OrbitUp),
            "ArrowDown" => Some(Self::OrbitDown),
            "+" | "=" => Some(Self::ZoomIn),
            "-" | "_" => Some(Self::ZoomOut),
            "Escape" | "Home" => Some(Self::ResetView),
            _ => None,
        }
    }

    pub const fn event_action(self) -> &'static str {
        match self {
            Self::OrbitLeft => "orbit-left",
            Self::OrbitRight => "orbit-right",
            Self::OrbitUp => "orbit-up",
            Self::OrbitDown => "orbit-down",
            Self::ZoomIn => "zoom-in",
            Self::ZoomOut => "zoom-out",
            Self::ResetView => "reset-view",
        }
    }
}

impl ScenaViewerGestureAction {
    pub const fn event_action(self) -> &'static str {
        match self {
            Self::Orbit => "orbit",
            Self::PinchZoom => "pinch-zoom",
            Self::WheelZoom => "wheel-zoom",
        }
    }
}

impl ScenaViewerDropDecision {
    pub fn from_file_names<I, N>(names: I) -> Self
    where
        I: IntoIterator<Item = N>,
        N: AsRef<str>,
    {
        let mut decision = Self::default();
        for name in names {
            let trimmed = name.as_ref().trim();
            if trimmed.is_empty() {
                continue;
            }
            match ScenaViewerDropKind::from_file_name(trimmed) {
                Some(kind) => decision.accepted.push(ScenaViewerDroppedFile {
                    name: trimmed.to_string(),
                    kind,
                }),
                None => decision.rejected.push(trimmed.to_string()),
            }
        }
        decision
    }

    pub fn accepted(&self) -> &[ScenaViewerDroppedFile] {
        &self.accepted
    }

    pub fn rejected(&self) -> &[String] {
        &self.rejected
    }

    pub fn has_accepted_files(&self) -> bool {
        !self.accepted.is_empty()
    }

    pub fn has_rejections(&self) -> bool {
        !self.rejected.is_empty()
    }

    pub fn status_text(&self) -> String {
        match (self.accepted.len(), self.rejected.is_empty()) {
            (0, true) => "Drop a .glb or .gltf file".to_string(),
            (0, false) => format!(
                "Rejected {}; expected .glb or .gltf",
                self.rejected.join(", ")
            ),
            (1, true) => "Accepted 1 glTF file".to_string(),
            (count, true) => format!("Accepted {count} glTF files"),
            (1, false) => format!(
                "Accepted 1 glTF file; rejected {}",
                self.rejected.join(", ")
            ),
            (count, false) => {
                format!(
                    "Accepted {count} glTF files; rejected {}",
                    self.rejected.join(", ")
                )
            }
        }
    }
}

impl ScenaViewerDroppedFile {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn kind(&self) -> ScenaViewerDropKind {
        self.kind
    }
}

impl ScenaViewerDropKind {
    pub fn from_file_name(name: &str) -> Option<Self> {
        let lower = name.trim().to_ascii_lowercase();
        if lower.ends_with(".glb") {
            Some(Self::Glb)
        } else if lower.ends_with(".gltf") {
            Some(Self::Gltf)
        } else {
            None
        }
    }

    pub const fn extension(self) -> &'static str {
        match self {
            Self::Glb => "glb",
            Self::Gltf => "gltf",
        }
    }
}

impl ScenaViewerVariantSelection {
    pub fn from_names<I, N>(names: I) -> Self
    where
        I: IntoIterator<Item = N>,
        N: AsRef<str>,
    {
        let options = names
            .into_iter()
            .filter_map(|name| {
                let trimmed = name.as_ref().trim();
                (!trimmed.is_empty()).then(|| ScenaViewerVariantOption {
                    name: trimmed.to_string(),
                    label: trimmed.to_string(),
                })
            })
            .collect();
        Self {
            options,
            active: None,
        }
    }

    pub fn with_active(mut self, name: &str) -> Self {
        let requested = name.trim();
        self.active = self
            .options
            .iter()
            .any(|option| option.name == requested)
            .then(|| requested.to_string());
        self
    }

    pub fn options(&self) -> &[ScenaViewerVariantOption] {
        &self.options
    }

    pub fn active(&self) -> Option<&str> {
        self.active.as_deref()
    }

    pub fn has_active_variant(&self) -> bool {
        self.active.is_some()
    }

    pub fn status_text(&self) -> String {
        match (self.options.len(), self.active()) {
            (0, _) => "No material variants".to_string(),
            (1, Some(active)) => format!("1 material variant; active {active}"),
            (count, Some(active)) => format!("{count} material variants; active {active}"),
            (1, None) => "1 material variant".to_string(),
            (count, None) => format!("{count} material variants"),
        }
    }
}

impl ScenaViewerVariantOption {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

impl Default for ScenaViewerProgress {
    fn default() -> Self {
        Self {
            phase: ScenaViewerProgressPhase::Idle,
            path: None,
            loaded_bytes: None,
            external_buffer_index: None,
            nodes: None,
            meshes: None,
        }
    }
}

impl ScenaViewerProgress {
    pub fn from_asset_event(event: &AssetLoadProgress) -> Self {
        match event {
            AssetLoadProgress::LoadStarted { path } => Self::for_path(
                ScenaViewerProgressPhase::Loading,
                Some(path.as_str().to_string()),
            ),
            AssetLoadProgress::CacheHit { path } => Self::for_path(
                ScenaViewerProgressPhase::Complete,
                Some(path.as_str().to_string()),
            ),
            AssetLoadProgress::AssetFetched { path, bytes } => {
                let mut progress = Self::for_path(
                    ScenaViewerProgressPhase::Fetching,
                    Some(path.as_str().to_string()),
                );
                progress.loaded_bytes = Some(*bytes);
                progress
            }
            AssetLoadProgress::ExternalBufferFetched { path, index, bytes } => {
                let mut progress = Self::for_path(
                    ScenaViewerProgressPhase::Fetching,
                    Some(path.as_str().to_string()),
                );
                progress.external_buffer_index = Some(*index);
                progress.loaded_bytes = Some(*bytes);
                progress
            }
            AssetLoadProgress::Parsed {
                path,
                nodes,
                meshes,
            } => {
                let mut progress = Self::for_path(
                    ScenaViewerProgressPhase::Parsing,
                    Some(path.as_str().to_string()),
                );
                progress.nodes = Some(*nodes);
                progress.meshes = Some(*meshes);
                progress
            }
            AssetLoadProgress::Cached { path } => Self::for_path(
                ScenaViewerProgressPhase::Complete,
                Some(path.as_str().to_string()),
            ),
        }
    }

    fn for_path(phase: ScenaViewerProgressPhase, path: Option<String>) -> Self {
        Self {
            phase,
            path,
            ..Self::default()
        }
    }

    pub const fn phase(&self) -> ScenaViewerProgressPhase {
        self.phase
    }

    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    pub const fn loaded_bytes(&self) -> Option<usize> {
        self.loaded_bytes
    }

    pub const fn external_buffer_index(&self) -> Option<usize> {
        self.external_buffer_index
    }

    pub const fn nodes(&self) -> Option<usize> {
        self.nodes
    }

    pub const fn meshes(&self) -> Option<usize> {
        self.meshes
    }

    pub const fn is_complete(&self) -> bool {
        matches!(self.phase, ScenaViewerProgressPhase::Complete)
    }

    pub fn aria_text(&self) -> String {
        let path = self.path.as_deref().unwrap_or("asset");
        match self.phase {
            ScenaViewerProgressPhase::Idle => "Ready".to_string(),
            ScenaViewerProgressPhase::Loading => format!("Loading {path}"),
            ScenaViewerProgressPhase::Fetching => {
                match (self.external_buffer_index, self.loaded_bytes) {
                    (Some(index), Some(bytes)) => {
                        format!("Fetched external buffer {index} with {bytes} bytes from {path}")
                    }
                    (_, Some(bytes)) => format!("Fetched {bytes} bytes from {path}"),
                    _ => format!("Fetching {path}"),
                }
            }
            ScenaViewerProgressPhase::Parsing => match (self.nodes, self.meshes) {
                (Some(nodes), Some(meshes)) => {
                    format!("Parsed {path} with {nodes} nodes and {meshes} meshes")
                }
                _ => format!("Parsing {path}"),
            },
            ScenaViewerProgressPhase::Caching => format!("Caching {path}"),
            ScenaViewerProgressPhase::Complete => format!("Loaded {path}"),
        }
    }
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn parse_tonemapper(value: &str) -> Tonemapper {
    match value.trim().to_ascii_lowercase().as_str() {
        "aces" => Tonemapper::Aces,
        "standard" => Tonemapper::Standard,
        "neutral" | "pbr-neutral" | "pbr_neutral" => Tonemapper::PbrNeutral,
        _ => Tonemapper::PbrNeutral,
    }
}

fn parse_boolean_attribute(name: &str, value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    normalized.is_empty()
        || normalized == "true"
        || normalized == "1"
        || normalized == name
        || normalized == name.replace('_', "-")
}
