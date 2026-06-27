use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScenaViewerAnnotationLayoutOptions {
    viewport_width: f32,
    viewport_height: f32,
    viewport_padding: f32,
    viewport_clamping: bool,
    overlap_avoidance: bool,
    occlusion_hiding: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenaViewerAnnotationLayoutInput {
    id: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    priority: i32,
    visible: bool,
    behind_camera: bool,
    occluded: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenaViewerAnnotationLayoutReport {
    coordinate_space: String,
    viewport_width: f32,
    viewport_height: f32,
    entries: Vec<ScenaViewerAnnotationLayoutEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenaViewerAnnotationLayoutEntry {
    id: String,
    original_x: f32,
    original_y: f32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    priority: i32,
    visible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hidden_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PlacedAnnotation {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl ScenaViewerAnnotationLayoutOptions {
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        Self {
            viewport_width: sanitize_extent(viewport_width),
            viewport_height: sanitize_extent(viewport_height),
            viewport_padding: 0.0,
            viewport_clamping: true,
            overlap_avoidance: false,
            occlusion_hiding: false,
        }
    }

    pub const fn with_viewport_clamping(mut self, enabled: bool) -> Self {
        self.viewport_clamping = enabled;
        self
    }

    pub const fn with_overlap_avoidance(mut self, enabled: bool) -> Self {
        self.overlap_avoidance = enabled;
        self
    }

    pub const fn with_occlusion_hiding(mut self, enabled: bool) -> Self {
        self.occlusion_hiding = enabled;
        self
    }

    pub fn with_viewport_padding(mut self, padding: f32) -> Self {
        self.viewport_padding = if padding.is_finite() {
            padding.max(0.0)
        } else {
            0.0
        };
        self
    }

    pub const fn viewport_width(&self) -> f32 {
        self.viewport_width
    }

    pub const fn viewport_height(&self) -> f32 {
        self.viewport_height
    }
}

impl Default for ScenaViewerAnnotationLayoutOptions {
    fn default() -> Self {
        Self::new(1.0, 1.0)
    }
}

impl ScenaViewerAnnotationLayoutInput {
    pub fn new(id: impl Into<String>, x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            id: id.into(),
            x: sanitize_coordinate(x),
            y: sanitize_coordinate(y),
            width: sanitize_extent(width),
            height: sanitize_extent(height),
            priority: 0,
            visible: true,
            behind_camera: false,
            occluded: false,
        }
    }

    pub const fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub const fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub const fn behind_camera(mut self, behind_camera: bool) -> Self {
        self.behind_camera = behind_camera;
        self
    }

    pub const fn occluded(mut self, occluded: bool) -> Self {
        self.occluded = occluded;
        self
    }
}

impl ScenaViewerAnnotationLayoutReport {
    pub fn entry(&self, id: &str) -> Option<&ScenaViewerAnnotationLayoutEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    pub fn entries(&self) -> &[ScenaViewerAnnotationLayoutEntry] {
        &self.entries
    }

    pub fn coordinate_space(&self) -> &str {
        &self.coordinate_space
    }

    pub const fn viewport_width(&self) -> f32 {
        self.viewport_width
    }

    pub const fn viewport_height(&self) -> f32 {
        self.viewport_height
    }
}

impl ScenaViewerAnnotationLayoutEntry {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub const fn original_x(&self) -> f32 {
        self.original_x
    }

    pub const fn original_y(&self) -> f32 {
        self.original_y
    }

    pub const fn x(&self) -> f32 {
        self.x
    }

    pub const fn y(&self) -> f32 {
        self.y
    }

    pub const fn visible(&self) -> bool {
        self.visible
    }

    pub fn hidden_reason(&self) -> Option<&str> {
        self.hidden_reason.as_deref()
    }
}

pub fn layout_scena_viewer_annotations<I>(
    inputs: I,
    options: ScenaViewerAnnotationLayoutOptions,
) -> ScenaViewerAnnotationLayoutReport
where
    I: IntoIterator<Item = ScenaViewerAnnotationLayoutInput>,
{
    let inputs = inputs.into_iter().collect::<Vec<_>>();
    let mut entries = inputs
        .iter()
        .map(|input| layout_entry(input, options))
        .collect::<Vec<_>>();

    if options.overlap_avoidance {
        let mut order = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (index, entry.priority, entry.id.clone()))
            .collect::<Vec<_>>();
        order.sort_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| left.2.cmp(&right.2))
                .then_with(|| left.0.cmp(&right.0))
        });

        let mut accepted = Vec::new();
        for (index, _, _) in order {
            if !entries[index].visible {
                continue;
            }
            let candidate = entries[index].placed();
            if accepted.iter().any(|placed| overlaps(candidate, *placed)) {
                entries[index].visible = false;
                entries[index].hidden_reason = Some("overlap".to_owned());
            } else {
                accepted.push(candidate);
            }
        }
    }

    ScenaViewerAnnotationLayoutReport {
        coordinate_space: "css_pixels".to_owned(),
        viewport_width: options.viewport_width,
        viewport_height: options.viewport_height,
        entries,
    }
}

impl ScenaViewerAnnotationLayoutEntry {
    const fn placed(&self) -> PlacedAnnotation {
        PlacedAnnotation {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        }
    }
}

fn layout_entry(
    input: &ScenaViewerAnnotationLayoutInput,
    options: ScenaViewerAnnotationLayoutOptions,
) -> ScenaViewerAnnotationLayoutEntry {
    let mut x = input.x;
    let mut y = input.y;
    if options.viewport_clamping {
        x = clamp_axis(
            x,
            input.width,
            options.viewport_width,
            options.viewport_padding,
        );
        y = clamp_axis(
            y,
            input.height,
            options.viewport_height,
            options.viewport_padding,
        );
    }

    let hidden_reason = if !input.visible {
        Some("hidden".to_owned())
    } else if input.behind_camera {
        Some("behind_camera".to_owned())
    } else if options.occlusion_hiding && input.occluded {
        Some("occluded".to_owned())
    } else {
        None
    };

    ScenaViewerAnnotationLayoutEntry {
        id: input.id.clone(),
        original_x: input.x,
        original_y: input.y,
        x,
        y,
        width: input.width,
        height: input.height,
        priority: input.priority,
        visible: hidden_reason.is_none(),
        hidden_reason,
    }
}

fn clamp_axis(value: f32, size: f32, viewport: f32, padding: f32) -> f32 {
    let min = padding.min(viewport);
    let max = (viewport - size - padding).max(min);
    value.clamp(min, max)
}

fn overlaps(a: PlacedAnnotation, b: PlacedAnnotation) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}

fn sanitize_coordinate(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn sanitize_extent(value: f32) -> f32 {
    if value.is_finite() {
        value.max(1.0)
    } else {
        1.0
    }
}
