//! Closed verified addresses for semantic agent delivery.

use serde::{Deserialize, Serialize};

use crate::domain::Direction;

use super::{AgentSessionBinding, HarnessKind, PaneContext, PaneRect};

/// Verified identity of one coding agent reachable through an integration.
///
/// The enclosing `scope` and the `delivery_id` are opaque to Proqi and owned by
/// the integration that verified them. Herdr addresses a pane inside a
/// workspace and tab, and thurbox addresses one flat session, so neither
/// topology is part of this contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentAddress {
    /// Ordered opaque identities enclosing the target, outermost first.
    scope: Vec<String>,
    /// Opaque identity of the addressed agent surface itself.
    delivery_id: String,
    /// Recognized coding-agent harness.
    agent_kind: HarnessKind,
    /// Stable harness session, or one explicitly qualified provisional binding.
    agent_session: AgentSessionBinding,
}

impl AgentAddress {
    /// Construct one complete verified delivery address.
    ///
    /// Returns `None` when the delivery identity or any scope segment is blank.
    #[must_use]
    pub fn new(
        scope: Vec<String>,
        delivery_id: String,
        agent_kind: HarnessKind,
        agent_session: AgentSessionBinding,
    ) -> Option<Self> {
        if delivery_id.trim().is_empty() || scope.iter().any(|value| value.trim().is_empty()) {
            return None;
        }
        Some(Self {
            scope,
            delivery_id,
            agent_kind,
            agent_session,
        })
    }

    /// Return the ordered opaque identities enclosing the target.
    #[must_use]
    pub fn scope(&self) -> &[String] {
        &self.scope
    }

    /// Return the opaque identity of the addressed agent surface.
    #[must_use]
    pub fn delivery_id(&self) -> &str {
        &self.delivery_id
    }

    /// Return the recognized harness kind.
    #[must_use]
    pub const fn agent_kind(&self) -> &HarnessKind {
        &self.agent_kind
    }

    /// Return the established or qualified provisional harness session.
    #[must_use]
    pub const fn agent_session(&self) -> &AgentSessionBinding {
        &self.agent_session
    }

    pub(super) fn replace_agent_kind(&mut self, kind: HarnessKind) {
        self.agent_kind = kind;
    }

    pub(super) fn replace_agent_session(&mut self, session: AgentSessionBinding) {
        self.agent_session = session;
    }
}

/// Stable closed route classification shared by persistence and diagnostics.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionRouteKind {
    /// Same-tab directional delivery with verified geometry.
    AdjacentPane,
    /// Current-server global Herdr agent delivery.
    HerdrAgent,
    /// Current-machine global thurbox session delivery.
    ThurboxSession,
}

impl SubmissionRouteKind {
    /// Stable internal storage and diagnostics spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AdjacentPane => "adjacent_pane",
            Self::HerdrAgent => "herdr_agent",
            Self::ThurboxSession => "thurbox_session",
        }
    }
}

/// One closed verified route for semantic prompt delivery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SubmissionRoute {
    /// Existing same-tab directional delivery with independently verified geometry.
    AdjacentPane {
        /// Direction from the Proqi pane.
        direction: Direction,
        /// Current-server identity of the adjacent agent.
        target: AgentAddress,
        /// Source context against which adjacency was verified.
        source: PaneContext,
        /// Verified adjacent target geometry.
        target_rect: PaneRect,
    },
    /// Globally addressed coding agent on the current Herdr server.
    HerdrAgent(AgentAddress),
    /// Globally addressed coding agent in one thurbox session.
    ThurboxSession(AgentAddress),
}

impl SubmissionRoute {
    /// Return the exact verified target address.
    #[must_use]
    pub const fn target(&self) -> &AgentAddress {
        match self {
            Self::AdjacentPane { target, .. }
            | Self::HerdrAgent(target)
            | Self::ThurboxSession(target) => target,
        }
    }

    /// Return the verified adjacent direction, when this is an adjacent route.
    #[must_use]
    pub const fn adjacent_direction(&self) -> Option<Direction> {
        match self {
            Self::AdjacentPane { direction, .. } => Some(*direction),
            Self::HerdrAgent(_) | Self::ThurboxSession(_) => None,
        }
    }

    /// Return the adjacent source context, when geometry is part of this route.
    #[must_use]
    pub const fn adjacent_source(&self) -> Option<&PaneContext> {
        match self {
            Self::AdjacentPane { source, .. } => Some(source),
            Self::HerdrAgent(_) | Self::ThurboxSession(_) => None,
        }
    }

    /// Return the verified adjacent target rectangle, when present.
    #[must_use]
    pub const fn adjacent_target_rect(&self) -> Option<PaneRect> {
        match self {
            Self::AdjacentPane { target_rect, .. } => Some(*target_rect),
            Self::HerdrAgent(_) | Self::ThurboxSession(_) => None,
        }
    }

    /// Stable route-kind spelling used by persistence and diagnostics.
    #[must_use]
    pub const fn kind_str(&self) -> &'static str {
        self.kind().as_str()
    }

    /// Return the closed route classification.
    #[must_use]
    pub const fn kind(&self) -> SubmissionRouteKind {
        match self {
            Self::AdjacentPane { .. } => SubmissionRouteKind::AdjacentPane,
            Self::HerdrAgent(_) => SubmissionRouteKind::HerdrAgent,
            Self::ThurboxSession(_) => SubmissionRouteKind::ThurboxSession,
        }
    }
}
