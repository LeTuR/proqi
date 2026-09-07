//! Redacted adjacent-agent target identity projection.

use sha2::{Digest as _, Sha256};

use crate::ports::agent::AgentTarget;

pub(super) fn target_fingerprint(target: &AgentTarget) -> [u8; 32] {
    let mut hasher = Sha256::new();
    let identity = target.identity();
    hasher.update(crate::ports::store::SUBMISSION_ROUTE_VERSION.to_be_bytes());
    let scope = identity.scope.iter().map(String::as_str);
    for field in [identity.provider.as_str(), identity.route_kind.as_str()]
        .into_iter()
        .chain(scope)
        .chain([identity.target_id.as_str(), identity.agent_kind.as_str()])
    {
        hasher.update(field.as_bytes());
        hasher.update([0]);
    }
    if let Some(source_pane_id) = identity.source_pane_id.as_deref() {
        hasher.update(source_pane_id.as_bytes());
    }
    hasher.update([0]);
    if let Some(direction) = identity.direction {
        hasher.update(direction.as_str().as_bytes());
    }
    hasher.update([0]);
    match identity.agent_session.as_id() {
        Some(session_id) => {
            hasher.update([1]);
            hasher.update(session_id.as_bytes());
        }
        None => hasher.update([0]),
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use sha2::{Digest as _, Sha256};

    use super::target_fingerprint;
    use crate::{
        domain::Direction,
        ports::agent::{
            AgentAddress, AgentAvailability, AgentDeliveryCapabilities, AgentSessionBinding,
            AgentState, AgentTarget, CODEX_AGENT_KIND, HarnessKind, PaneContext, PaneRect,
        },
    };

    /// Rebuild the exact byte sequence the direction-only and workspace, tab,
    /// and pane projections hashed before the address became integration
    /// neutral. Any change to the projection breaks this and invalidates every
    /// fingerprint already written to a user's submission journal.
    fn frozen_herdr_digest(
        route_kind: &str,
        workspace_id: &str,
        tab_id: &str,
        pane_id: &str,
        source_pane_id: Option<&str>,
        direction: Option<Direction>,
        session_id: Option<&str>,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(crate::ports::store::SUBMISSION_ROUTE_VERSION.to_be_bytes());
        for field in [
            "herdr",
            route_kind,
            workspace_id,
            tab_id,
            pane_id,
            CODEX_AGENT_KIND,
        ] {
            hasher.update(field.as_bytes());
            hasher.update([0]);
        }
        if let Some(source_pane_id) = source_pane_id {
            hasher.update(source_pane_id.as_bytes());
        }
        hasher.update([0]);
        if let Some(direction) = direction {
            hasher.update(direction.as_str().as_bytes());
        }
        hasher.update([0]);
        match session_id {
            Some(session_id) => {
                hasher.update([1]);
                hasher.update(session_id.as_bytes());
            }
            None => hasher.update([0]),
        }
        hasher.finalize().into()
    }

    fn address(session: AgentSessionBinding) -> AgentAddress {
        AgentAddress::new(
            vec!["w1".to_owned(), "w1:t1".to_owned()],
            "w1:p2".to_owned(),
            HarnessKind::new(CODEX_AGENT_KIND).expect("fixture harness"),
            session,
        )
        .expect("fixture address")
    }

    fn source() -> PaneContext {
        PaneContext {
            workspace_id: "w1".to_owned(),
            tab_id: "w1:t1".to_owned(),
            pane_id: "w1:p1".to_owned(),
            rect: PaneRect {
                x: 0,
                y: 0,
                width: 20,
                height: 20,
            },
        }
    }

    #[test]
    fn a_herdr_target_still_fingerprints_exactly_as_it_did_before_the_neutral_address() {
        let adjacent = AgentTarget::adjacent(
            "herdr".to_owned(),
            20,
            Direction::Right,
            address(AgentSessionBinding::established("agent-session").expect("fixture session")),
            "reviewer".to_owned(),
            AgentState::Idle,
            AgentDeliveryCapabilities::SUBMIT_ONLY,
            source().rect,
            source(),
        );
        assert_eq!(
            target_fingerprint(&adjacent),
            frozen_herdr_digest(
                "adjacent_pane",
                "w1",
                "w1:t1",
                "w1:p2",
                Some("w1:p1"),
                Some(Direction::Right),
                Some("agent-session"),
            )
        );

        let global = AgentTarget::herdr_agent(
            20,
            address(AgentSessionBinding::established("agent-session").expect("fixture session")),
            "reviewer".to_owned(),
            vec!["Workspace".to_owned(), "Tab".to_owned(), "p2".to_owned()],
            AgentState::Idle,
            AgentAvailability::Available,
            AgentDeliveryCapabilities::SUBMIT_ONLY,
        );
        assert_eq!(
            target_fingerprint(&global),
            frozen_herdr_digest(
                "herdr_agent",
                "w1",
                "w1:t1",
                "w1:p2",
                None,
                None,
                Some("agent-session"),
            )
        );

        let provisional = AgentTarget::herdr_agent(
            20,
            address(AgentSessionBinding::provisional()),
            "reviewer".to_owned(),
            Vec::new(),
            AgentState::Idle,
            AgentAvailability::Available,
            AgentDeliveryCapabilities::SUBMIT_ONLY,
        );
        assert_eq!(
            target_fingerprint(&provisional),
            frozen_herdr_digest("herdr_agent", "w1", "w1:t1", "w1:p2", None, None, None)
        );
    }

    #[test]
    fn a_thurbox_target_fingerprints_distinctly_from_a_herdr_target() {
        let thurbox = AgentTarget::thurbox_session(
            45,
            AgentAddress::new(
                Vec::new(),
                "w1:p2".to_owned(),
                HarnessKind::new(CODEX_AGENT_KIND).expect("fixture harness"),
                AgentSessionBinding::established("agent-session").expect("fixture session"),
            )
            .expect("fixture address"),
            "crewmate".to_owned(),
            Vec::new(),
            AgentState::Idle,
            AgentAvailability::Available,
            AgentDeliveryCapabilities::SUBMIT_ONLY,
        );
        let herdr = AgentTarget::herdr_agent(
            20,
            address(AgentSessionBinding::established("agent-session").expect("fixture session")),
            "reviewer".to_owned(),
            Vec::new(),
            AgentState::Idle,
            AgentAvailability::Available,
            AgentDeliveryCapabilities::SUBMIT_ONLY,
        );
        assert_ne!(target_fingerprint(&thurbox), target_fingerprint(&herdr));
    }
}
