use super::{
    ConnectOptions, ConnectionError, ConnectionLineOverlay, ConnectionPreview, ConnectorFrame,
};
use crate::scene::{Scene, Transform};

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionMagnetPreview {
    preview: ConnectionPreview,
    distance: f32,
    tolerance: f32,
    visual_cue: ConnectionMagnetVisualCue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionMagnetVisualCue {
    SnapReady,
    OutOfRange,
}

impl ConnectionMagnetPreview {
    pub const fn preview(&self) -> &ConnectionPreview {
        &self.preview
    }

    pub const fn distance(&self) -> f32 {
        self.distance
    }

    pub const fn tolerance(&self) -> f32 {
        self.tolerance
    }

    pub const fn is_snap_ready(&self) -> bool {
        matches!(self.visual_cue, ConnectionMagnetVisualCue::SnapReady)
    }

    pub const fn visual_cue(&self) -> ConnectionMagnetVisualCue {
        self.visual_cue
    }

    pub const fn ghost_transform(&self) -> Transform {
        self.preview.resolved_transform()
    }

    pub const fn connection_line(&self) -> ConnectionLineOverlay {
        self.preview.connection_line()
    }
}

impl ConnectionMagnetVisualCue {
    pub const fn css_class(self) -> &'static str {
        match self {
            Self::SnapReady => "scena-magnet-ready",
            Self::OutOfRange => "scena-magnet-out-of-range",
        }
    }

    pub const fn accent_rgba(self) -> [f32; 4] {
        match self {
            Self::SnapReady => [0.1, 0.8, 0.35, 1.0],
            Self::OutOfRange => [1.0, 0.72, 0.2, 1.0],
        }
    }
}

impl Scene {
    pub fn preview_connector_magnet(
        &self,
        source: ConnectorFrame,
        target: ConnectorFrame,
        options: ConnectOptions,
    ) -> Result<ConnectionMagnetPreview, ConnectionError> {
        let preview = self.preview_connection(source, target, options)?;
        let connection_line = preview.connection_line();
        let distance = connection_line.start().distance(connection_line.end());
        let tolerance = connector_magnet_tolerance(preview.source(), preview.target());
        let visual_cue = if tolerance > 0.0 && distance <= tolerance {
            ConnectionMagnetVisualCue::SnapReady
        } else {
            ConnectionMagnetVisualCue::OutOfRange
        };
        Ok(ConnectionMagnetPreview {
            preview,
            distance,
            tolerance,
            visual_cue,
        })
    }
}

fn connector_magnet_tolerance(source: &ConnectorFrame, target: &ConnectorFrame) -> f32 {
    source
        .snap_tolerance()
        .into_iter()
        .chain(target.snap_tolerance())
        .fold(0.0, f32::max)
}
