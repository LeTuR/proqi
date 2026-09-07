//! Minimal thurbox CLI response shapes consumed by Proqi.

use serde::Deserialize;

/// Failure body thurbox prints on standard output with a non-zero exit status.
#[derive(Deserialize)]
pub(super) struct ErrorDocument {
    pub(super) error: String,
}

/// Published installation identity from `thurbox-cli version --json`.
#[derive(Deserialize)]
pub(super) struct VersionDocument {
    pub(super) version: String,
    pub(super) schema_version: u32,
}

/// One row of `thurbox-cli session list --json --verify`.
#[derive(Clone, Deserialize)]
pub(super) struct SessionRow {
    pub(super) id: String,
    #[serde(default)]
    pub(super) name: Option<String>,
    #[serde(default)]
    pub(super) cwd: Option<String>,
    /// Registered agent the session row was created as.
    #[serde(default)]
    pub(super) agent: Option<String>,
    /// Registered agent a driver declared for this session.
    #[serde(default)]
    pub(super) reports_as: Option<String>,
    /// Registered agent the pane probe observed holding the session.
    #[serde(default)]
    pub(super) detected_agent: Option<String>,
    /// Conversation identity thurbox recorded for the session's agent.
    #[serde(default)]
    pub(super) agent_session_id: Option<String>,
    /// Honest one-word state: a hook report, `running`, `unreported`,
    /// `uncovered`, or `stopped`.
    #[serde(default)]
    pub(super) state: Option<String>,
    /// Whether the session's registered agent can report states at all.
    #[serde(default)]
    pub(super) hook_coverage: Option<String>,
    #[serde(default)]
    pub(super) stopped: Option<bool>,
}

/// Receipt body from `thurbox-cli session send --json`.
#[derive(Deserialize)]
pub(super) struct SendReceipt {
    pub(super) sent: bool,
    pub(super) submitted: bool,
    pub(super) session_id: String,
}
