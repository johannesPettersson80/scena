use super::super::Transform;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionAlignment {
    #[default]
    ForwardToForward,
    ForwardToBack,
    NormalToNormal,
    NormalToOpposite,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ConnectionRoll {
    #[default]
    MatchTarget,
    PreserveSource,
    ChooseNearest {
        step_degrees: f32,
    },
    ExplicitDegrees(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionParenting {
    #[default]
    PreserveSourceParent,
    ReparentSourceToTargetParent,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConnectOptions {
    pub(crate) mate_offset: Transform,
    pub(crate) allow_non_uniform_scale: bool,
    alignment: ConnectionAlignment,
    roll: ConnectionRoll,
    parenting: ConnectionParenting,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConnectionRequest {
    source: super::super::ConnectorKey,
    target: super::super::ConnectorKey,
    options: ConnectOptions,
}

impl Default for ConnectOptions {
    fn default() -> Self {
        Self {
            mate_offset: Transform::IDENTITY,
            allow_non_uniform_scale: false,
            alignment: ConnectionAlignment::ForwardToForward,
            roll: ConnectionRoll::MatchTarget,
            parenting: ConnectionParenting::PreserveSourceParent,
        }
    }
}

impl ConnectOptions {
    pub const fn with_mate_offset(mut self, mate_offset: Transform) -> Self {
        self.mate_offset = mate_offset;
        self
    }

    pub const fn allow_non_uniform_scale(mut self, allow: bool) -> Self {
        self.allow_non_uniform_scale = allow;
        self
    }

    pub const fn with_alignment(mut self, alignment: ConnectionAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub const fn alignment(self) -> ConnectionAlignment {
        self.alignment
    }

    pub const fn match_target_roll(mut self) -> Self {
        self.roll = ConnectionRoll::MatchTarget;
        self
    }

    pub const fn preserve_roll(mut self) -> Self {
        self.roll = ConnectionRoll::PreserveSource;
        self
    }

    pub fn choose_nearest_roll_degrees(mut self, step_degrees: f32) -> Self {
        let step_degrees = if step_degrees.is_finite() && step_degrees.abs() > f32::EPSILON {
            step_degrees.abs()
        } else {
            90.0
        };
        self.roll = ConnectionRoll::ChooseNearest { step_degrees };
        self
    }

    pub const fn with_explicit_roll_degrees(mut self, degrees: f32) -> Self {
        self.roll = ConnectionRoll::ExplicitDegrees(degrees);
        self
    }

    pub const fn roll(self) -> ConnectionRoll {
        self.roll
    }

    pub const fn preserve_source_parent(mut self) -> Self {
        self.parenting = ConnectionParenting::PreserveSourceParent;
        self
    }

    pub const fn reparent_source_to_target_parent(mut self) -> Self {
        self.parenting = ConnectionParenting::ReparentSourceToTargetParent;
        self
    }

    pub const fn parenting(self) -> ConnectionParenting {
        self.parenting
    }

    pub(crate) fn alignment_transform(self) -> Transform {
        match self.alignment {
            ConnectionAlignment::ForwardToForward | ConnectionAlignment::NormalToNormal => {
                Transform::IDENTITY
            }
            ConnectionAlignment::ForwardToBack | ConnectionAlignment::NormalToOpposite => {
                Transform::IDENTITY.rotate_y_deg(180.0)
            }
        }
    }
}

impl ConnectionRequest {
    pub const fn new(
        source: super::super::ConnectorKey,
        target: super::super::ConnectorKey,
        options: ConnectOptions,
    ) -> Self {
        Self {
            source,
            target,
            options,
        }
    }

    pub const fn source(self) -> super::super::ConnectorKey {
        self.source
    }

    pub const fn target(self) -> super::super::ConnectorKey {
        self.target
    }

    pub const fn options(self) -> ConnectOptions {
        self.options
    }
}
