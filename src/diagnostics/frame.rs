#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum DevicePollStatus {
    /// The backend safely retires logical resources without claiming GPU completion.
    Automatic,
    /// The renderer/backend has no explicit GPU completion path.
    #[default]
    Unsupported,
    /// A real asynchronous queue-completion signal has been submitted but not observed yet.
    Submitted,
    /// The backend confirmed completion and the reported destructions may be retired.
    Confirmed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DevicePoll {
    pub pending_destructions_before: u64,
    pub pending_destructions_after: u64,
    pub destroyed_resources: u64,
    pub status: DevicePollStatus,
    /// Compatibility projection: true only when `status == Confirmed`.
    pub gpu_polled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderOutcome {
    pub width: u32,
    pub height: u32,
    pub draw_calls: u64,
    pub primitives: u64,
    pub skipped: bool,
}
