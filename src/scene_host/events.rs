use std::cell::{Cell, RefCell};
use std::fmt;

use serde::{Deserialize, Serialize};

use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{
    ASSET_LOAD_REPORT_SCHEMA_V1, AssetFetcher, AssetLoadProgressV1, AssetLoadReportV1, Backend,
    CAPABILITY_REPORT_SCHEMA_V1, CAPTURE_SCHEMA_V1, CapturePayloadKind, CaptureRgba8,
    CursorPosition, Diagnostic, DiagnosticCode, DiagnosticSeverity, Hit, HitTarget,
    SurfaceViewport, Vec3, Viewport,
};

pub const HOST_EVENT_SCHEMA_V1: &str = "scena.host_event.v1";

type HostEventSink = Box<dyn FnMut(HostEventV1)>;

pub(super) struct HostEventQueue {
    events: RefCell<Vec<HostEventV1>>,
    sink: RefCell<Option<HostEventSink>>,
    sink_revision: Cell<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostEventBatchV1 {
    pub schema: String,
    pub events: Vec<HostEventV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HostEventV1 {
    Pick {
        x_css_px: f32,
        y_css_px: f32,
        hit: Option<HostEventHitV1>,
        button: Option<HostEventButtonV1>,
        modifiers: HostEventModifiersV1,
    },
    Hover {
        x_css_px: f32,
        y_css_px: f32,
        phase: HostEventHoverPhaseV1,
        hit: Option<HostEventHitV1>,
    },
    SelectionChanged {
        previous: Option<u64>,
        current: Option<u64>,
    },
    LoadProgress {
        progress: AssetLoadProgressV1,
    },
    AssetLoaded {
        import: u64,
        asset_load_report: Box<AssetLoadReportV1>,
    },
    Diagnostic {
        code: DiagnosticCode,
        severity: DiagnosticSeverity,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        node: Option<u64>,
        message: String,
        help: Option<String>,
    },
    CaptureReady {
        capture_schema: String,
        width: u32,
        height: u32,
        pixel_format: String,
        payload_kind: CapturePayloadKind,
        payload_bytes: usize,
        payload_fnv1a64: String,
    },
    SurfaceResized {
        width_css_px: f32,
        height_css_px: f32,
        width_physical_px: u32,
        height_physical_px: u32,
        device_pixel_ratio: f32,
    },
    ContextLost {
        recoverable: bool,
    },
    ContextRestored,
    DeviceLost {
        recoverable: bool,
    },
    DeviceRecovered,
    CapabilityChanged {
        capability_schema: String,
        backend: Backend,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HostEventHitV1 {
    pub target: HostEventTargetKindV1,
    pub handle: u64,
    pub distance: f32,
    pub world_position: Vec3,
    pub normal: Option<Vec3>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostEventTargetKindV1 {
    Node,
    InstanceRoot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostEventButtonV1 {
    Primary,
    Secondary,
    Auxiliary,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostEventModifiersV1 {
    pub alt: bool,
    pub ctrl: bool,
    pub meta: bool,
    pub shift: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostEventHoverPhaseV1 {
    Entered,
    Moved,
    Left,
}

impl Default for HostEventQueue {
    fn default() -> Self {
        Self {
            events: RefCell::new(Vec::new()),
            sink: RefCell::new(None),
            sink_revision: Cell::new(0),
        }
    }
}

impl fmt::Debug for HostEventQueue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HostEventQueue")
            .field("queued", &self.events.borrow().len())
            .field("sink", &self.sink.borrow().as_ref().map(|_| "registered"))
            .finish()
    }
}

impl HostEventQueue {
    fn set_sink<S>(&self, sink: S)
    where
        S: FnMut(HostEventV1) + 'static,
    {
        *self.sink.borrow_mut() = Some(Box::new(sink));
        self.bump_sink_revision();
    }

    fn clear_sink(&self) {
        *self.sink.borrow_mut() = None;
        self.bump_sink_revision();
    }

    fn emit(&self, event: HostEventV1) {
        let restore_revision = self.sink_revision.get();
        let Some(mut sink) = self.sink.borrow_mut().take() else {
            self.events.borrow_mut().push(event);
            return;
        };
        sink(event);
        if self.sink.borrow().is_none() && self.sink_revision.get() == restore_revision {
            *self.sink.borrow_mut() = Some(sink);
        }
    }

    fn drain(&self) -> Vec<HostEventV1> {
        std::mem::take(&mut *self.events.borrow_mut())
    }

    fn bump_sink_revision(&self) {
        self.sink_revision
            .set(self.sink_revision.get().wrapping_add(1));
    }
}

impl HostEventBatchV1 {
    pub fn new(events: Vec<HostEventV1>) -> Self {
        Self {
            schema: HOST_EVENT_SCHEMA_V1.to_owned(),
            events,
        }
    }
}

impl HostEventV1 {
    fn diagnostic(diagnostic: &Diagnostic, node: Option<u64>, message: String) -> Self {
        Self::Diagnostic {
            code: diagnostic.code,
            severity: diagnostic.severity,
            node,
            message,
            help: diagnostic.help.clone(),
        }
    }

    pub(super) fn capture_ready(capture: &CaptureRgba8) -> Self {
        Self::CaptureReady {
            capture_schema: CAPTURE_SCHEMA_V1.to_owned(),
            width: capture.descriptor.width,
            height: capture.descriptor.height,
            pixel_format: capture.descriptor.pixel_format.clone(),
            payload_kind: capture.descriptor.payload.kind,
            payload_bytes: capture.rgba8.len(),
            payload_fnv1a64: capture.descriptor.payload.fnv1a64.clone(),
        }
    }

    pub(super) fn capability_changed(backend: Backend) -> Self {
        Self::CapabilityChanged {
            capability_schema: CAPABILITY_REPORT_SCHEMA_V1.to_owned(),
            backend,
        }
    }
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn pick(&mut self, x: f32, y: f32) -> Result<Option<u64>, SceneHostError> {
        let hit = self.pick_hit(x, y)?;
        let event_hit = hit.and_then(|hit| self.host_event_hit(hit));
        let handle = event_hit.as_ref().map(|hit| hit.handle);
        self.emit_event(HostEventV1::Pick {
            x_css_px: x,
            y_css_px: y,
            hit: event_hit,
            button: None,
            modifiers: HostEventModifiersV1::default(),
        });
        Ok(handle)
    }

    pub fn hover(&mut self, x: f32, y: f32) -> Result<Option<u64>, SceneHostError> {
        let previous = self.scene.interaction().hover();
        let hit = self.pick_hit(x, y)?;
        let target = hit.map(|hit| hit.target());
        let event_hit = hit.and_then(|hit| self.host_event_hit(hit));
        let handle = event_hit.as_ref().map(|hit| hit.handle);
        self.scene.set_hover_target(target);
        let phase = match (previous, target) {
            (Some(_), None) => HostEventHoverPhaseV1::Left,
            (Some(previous), Some(current)) if previous == current => HostEventHoverPhaseV1::Moved,
            (_, Some(_)) => HostEventHoverPhaseV1::Entered,
            (None, None) => HostEventHoverPhaseV1::Moved,
        };
        self.emit_event(HostEventV1::Hover {
            x_css_px: x,
            y_css_px: y,
            phase,
            hit: event_hit,
        });
        Ok(handle)
    }

    pub fn select(&mut self, x: f32, y: f32) -> Result<Option<u64>, SceneHostError> {
        let previous = self
            .scene
            .interaction()
            .primary_selection()
            .and_then(|target| self.handle_for_hit_target(target));
        let hit = self.pick_hit(x, y)?;
        let target = hit.map(|hit| hit.target());
        let event_hit = hit.and_then(|hit| self.host_event_hit(hit));
        let current = event_hit.as_ref().map(|hit| hit.handle);
        self.scene.set_hover_target(target);
        self.scene.set_primary_selection_target(target);
        if previous != current {
            self.emit_event(HostEventV1::SelectionChanged { previous, current });
        }
        Ok(current)
    }

    pub fn set_event_sink<S>(&mut self, sink: S)
    where
        S: FnMut(HostEventV1) + 'static,
    {
        self.events.set_sink(sink);
    }

    pub fn clear_event_sink(&mut self) {
        self.events.clear_sink();
    }

    pub fn drain_events(&self) -> Vec<HostEventV1> {
        self.events.drain()
    }

    pub fn drain_events_json(&self) -> Result<String, SceneHostError> {
        let batch = HostEventBatchV1::new(self.drain_events());
        serde_json::to_string(&batch).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("host event serialization failed: {error}"),
            )
        })
    }

    pub(super) fn emit_event(&self, event: HostEventV1) {
        self.events.emit(event);
    }

    pub(super) fn emit_asset_load_events(
        &self,
        import: u64,
        asset_load_report: &AssetLoadReportV1,
    ) {
        debug_assert_eq!(asset_load_report.schema, ASSET_LOAD_REPORT_SCHEMA_V1);
        for progress in &asset_load_report.progress_events {
            self.emit_event(HostEventV1::LoadProgress {
                progress: progress.clone(),
            });
        }
        self.emit_event(HostEventV1::AssetLoaded {
            import,
            asset_load_report: Box::new(asset_load_report.clone()),
        });
    }

    pub(super) fn emit_asset_progress_events(&self, asset_load_report: &AssetLoadReportV1) {
        debug_assert_eq!(asset_load_report.schema, ASSET_LOAD_REPORT_SCHEMA_V1);
        for progress in &asset_load_report.progress_events {
            self.emit_event(HostEventV1::LoadProgress {
                progress: progress.clone(),
            });
        }
    }

    pub(super) fn emit_changed_diagnostics(&mut self) {
        let diagnostics = self.renderer.diagnostics().to_vec();
        let events = diagnostics
            .iter()
            .map(|diagnostic| self.diagnostic_event(diagnostic))
            .collect::<Vec<_>>();
        if events == self.last_diagnostic_events {
            return;
        }
        self.last_diagnostic_events = events.clone();
        for event in events {
            self.emit_event(event);
        }
    }

    pub(super) fn emit_surface_resized_event(&self, viewport: SurfaceViewport) {
        let size = viewport.physical_size();
        self.emit_event(HostEventV1::SurfaceResized {
            width_css_px: viewport.logical_width(),
            height_css_px: viewport.logical_height(),
            width_physical_px: size.width,
            height_physical_px: size.height,
            device_pixel_ratio: viewport.device_pixel_ratio(),
        });
    }

    fn pick_hit(&self, x: f32, y: f32) -> Result<Option<Hit>, SceneHostError> {
        let size = self.viewport.physical_size();
        let viewport = Viewport::new(size.width, size.height, self.viewport.device_pixel_ratio())
            .ok_or_else(|| {
            SceneHostError::new(
                SceneHostErrorCode::InvalidViewport,
                format!(
                    "invalid viewport {}x{} at DPR {}",
                    size.width,
                    size.height,
                    self.viewport.device_pixel_ratio()
                ),
            )
        })?;
        Ok(self.scene.pick_with_assets(
            self.active_camera,
            CursorPosition::logical(x, y),
            viewport,
            &self.assets,
        )?)
    }

    fn host_event_hit(&mut self, hit: Hit) -> Option<HostEventHitV1> {
        let (target, handle) = match hit.target {
            HitTarget::Node(node) => (HostEventTargetKindV1::Node, self.register_node(node)),
            HitTarget::Instance { node, instance } => (
                HostEventTargetKindV1::InstanceRoot,
                *self.instance_handle_map.get(&(node, instance))?,
            ),
        };
        Some(HostEventHitV1 {
            target,
            handle,
            distance: hit.distance,
            world_position: hit.world_position,
            normal: hit.normal,
        })
    }

    fn handle_for_hit_target(&self, target: HitTarget) -> Option<u64> {
        match target {
            HitTarget::Node(node) => self.node_handle_map.get(&node).copied(),
            HitTarget::Instance { node, instance } => {
                self.instance_handle_map.get(&(node, instance)).copied()
            }
        }
    }

    fn diagnostic_event(&mut self, diagnostic: &Diagnostic) -> HostEventV1 {
        let node_handle = diagnostic.node().map(|node| self.register_node(node));
        let message = host_diagnostic_message(diagnostic, node_handle);
        HostEventV1::diagnostic(diagnostic, node_handle, message)
    }
}

fn host_diagnostic_message(diagnostic: &Diagnostic, node_handle: Option<u64>) -> String {
    let Some(handle) = node_handle else {
        return diagnostic.message.clone();
    };
    if let Some(rest) = diagnostic.message.strip_prefix("camera node ") {
        return format!("camera node handle {handle} {rest}");
    }
    if let Some(rest) = diagnostic.message.strip_prefix("node ") {
        return format!("node handle {handle} {rest}");
    }
    format!("node handle {handle}: {}", diagnostic.message)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::{HostEventQueue, HostEventV1};

    #[test]
    fn event_queue_sink_can_emit_reentrantly_without_borrow_panic() {
        let queue = Rc::new(HostEventQueue::default());
        let reentered = Rc::new(Cell::new(false));
        let queue_for_sink = Rc::clone(&queue);
        let reentered_for_sink = Rc::clone(&reentered);
        queue.set_sink(move |_| {
            if !reentered_for_sink.replace(true) {
                queue_for_sink.emit(HostEventV1::ContextRestored);
            }
        });

        queue.emit(HostEventV1::ContextLost { recoverable: true });

        assert!(reentered.get());
        assert_eq!(queue.drain(), vec![HostEventV1::ContextRestored]);
    }

    #[test]
    fn event_queue_clear_sink_inside_callback_is_not_restored() {
        let queue = Rc::new(HostEventQueue::default());
        let queue_for_sink = Rc::clone(&queue);
        queue.set_sink(move |_| {
            queue_for_sink.clear_sink();
        });

        queue.emit(HostEventV1::ContextLost { recoverable: true });
        queue.emit(HostEventV1::ContextRestored);

        assert_eq!(queue.drain(), vec![HostEventV1::ContextRestored]);
    }
}
