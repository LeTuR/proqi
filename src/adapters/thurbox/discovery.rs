//! Compatibility negotiation and verified thurbox session discovery.

use std::collections::BTreeSet;

use crate::ports::{
    agent::{
        AgentAddress, AgentAvailability, AgentCapabilities, AgentDeliveryCapabilities, AgentError,
        AgentSessionBinding, AgentState, AgentTarget, HarnessKind, MAX_AGENT_NAME_CHARS,
        MAX_LOCATION_LABEL_CHARS, bounded_integration_label,
    },
    environment::ProcessRunner,
};

use super::{DISCOVERY_TIMEOUT, ThurboxGateway, contract::SessionRow};

/// Verified row budget for one discovery pass, matching the Herdr adapter.
pub(super) const MAX_SESSION_ROWS: usize = 128;

/// Shortest session-identity prefix Proqi will ever display.
const MIN_DISPLAYED_ID_CHARS: usize = 8;

/// A session identity no longer than this is shown whole, never abbreviated.
const WHOLE_ID_CHARS: usize = 16;

pub(super) fn capabilities<R: ProcessRunner>(
    gateway: &mut ThurboxGateway<R>,
) -> Result<AgentCapabilities, AgentError> {
    let installation = gateway.installation()?;
    Ok(AgentCapabilities {
        provider: super::PROVIDER.to_owned(),
        version: installation.version().to_owned(),
        protocol: installation.schema_version(),
        delivery: AgentDeliveryCapabilities::SUBMIT_ONLY,
        // thurbox addresses flat sessions and publishes no pane containing
        // Proqi, so it verifies no directional adjacency.
        context: None,
    })
}

pub(super) fn targets<R: ProcessRunner>(
    gateway: &mut ThurboxGateway<R>,
) -> Result<Vec<AgentTarget>, AgentError> {
    let protocol = gateway.installation()?.schema_version();
    // `--verify` is what makes `running` and `detected_agent` answerable, so a
    // foreign-launched agent is discoverable instead of silently absent.
    let rows: Vec<SessionRow> = gateway.json(
        &["session", "list", "--json", "--verify"],
        DISCOVERY_TIMEOUT,
    )?;
    if rows.len() > MAX_SESSION_ROWS {
        return Err(AgentError::Unsupported(
            "thurbox session discovery exceeds the verified row budget".to_owned(),
        ));
    }
    unique_targets(rows, protocol, gateway.current_session())
}

fn unique_targets(
    rows: Vec<SessionRow>,
    protocol: u32,
    current_session: Option<&str>,
) -> Result<Vec<AgentTarget>, AgentError> {
    let mut session_ids = BTreeSet::new();
    let mut listed = Vec::new();
    for row in rows {
        let id = row.id.trim();
        if id.is_empty() {
            return Err(AgentError::Malformed(
                "thurbox session has an incomplete delivery identity".to_owned(),
            ));
        }
        if !session_ids.insert(id.to_owned()) {
            return Err(AgentError::Ambiguous(
                "multiple thurbox sessions claim one delivery identity".to_owned(),
            ));
        }
        if current_session != Some(id) {
            listed.push(row);
        }
    }
    let displayed_id_chars = displayed_id_chars(&listed);
    let mut targets = Vec::new();
    for row in &listed {
        if let Some(target) = target(row, protocol, displayed_id_chars)? {
            targets.push(target);
        }
    }
    Ok(targets)
}

/// Return the shortest session-identity prefix that separates every listed row.
///
/// A driver names sessions from one long generated pattern, so two rows can
/// share a working directory and agree through the bounded part of their names.
/// The row still has to say which session it is, and thurbox itself addresses a
/// session by unique identity prefix. An identity short enough to read is shown
/// whole, and delivery always uses the complete identity from the address.
fn displayed_id_chars(rows: &[SessionRow]) -> usize {
    let ids = rows.iter().map(|row| row.id.trim()).collect::<Vec<_>>();
    let longest = ids.iter().map(|id| id.chars().count()).max().unwrap_or(0);
    if longest <= WHOLE_ID_CHARS {
        return longest;
    }
    (MIN_DISPLAYED_ID_CHARS..=longest)
        .find(|length| {
            ids.iter()
                .map(|id| id.chars().take(*length).collect::<String>())
                .collect::<BTreeSet<_>>()
                .len()
                == ids.len()
        })
        .unwrap_or(longest)
}

fn target(
    row: &SessionRow,
    protocol: u32,
    displayed_id_chars: usize,
) -> Result<Option<AgentTarget>, AgentError> {
    let Some(kind) = harness(row) else {
        return Ok(None);
    };
    let address = AgentAddress::new(
        Vec::new(),
        row.id.trim().to_owned(),
        kind.clone(),
        session_binding(row),
    )
    .ok_or_else(|| AgentError::Malformed("invalid thurbox session address".to_owned()))?;
    let (readiness, availability) = live_state(row);
    let agent_name = row
        .name
        .as_deref()
        .and_then(|name| bounded_integration_label(name, MAX_AGENT_NAME_CHARS))
        .unwrap_or_else(|| format!("{kind} session"));
    // The last segment identifies the session itself, which is what a narrow
    // row keeps. Delivery always uses the complete identity in the address.
    let mut location = row
        .cwd
        .as_deref()
        .and_then(|cwd| bounded_integration_label(cwd, MAX_LOCATION_LABEL_CHARS))
        .map(|cwd| vec![cwd])
        .unwrap_or_default();
    location.push(
        address
            .delivery_id()
            .chars()
            .take(displayed_id_chars)
            .collect(),
    );
    Ok(Some(AgentTarget::thurbox_session(
        protocol,
        address,
        agent_name,
        location,
        readiness,
        availability,
        AgentDeliveryCapabilities::SUBMIT_ONLY,
    )))
}

/// Resolve which registered agent holds the session.
///
/// thurbox names three: `agent` is what the row was created as, `reports_as`
/// what a driver declared, and `detected_agent` what the pane probe observed
/// running. The observation wins, then the declaration, then the row. A
/// session whose agent can report nothing and whose pane holds no observed
/// agent is a plain shell and is not a delivery target.
fn harness(row: &SessionRow) -> Option<HarnessKind> {
    if let Some(detected) = nonblank(row.detected_agent.as_deref()) {
        return HarnessKind::new(detected);
    }
    if !reports_state(row) {
        return None;
    }
    let declared =
        nonblank(row.reports_as.as_deref()).or_else(|| nonblank(row.agent.as_deref()))?;
    HarnessKind::new(declared)
}

fn reports_state(row: &SessionRow) -> bool {
    nonblank(row.hook_coverage.as_deref()).is_some_and(|coverage| coverage != "none")
}

/// thurbox records the conversation its agent was launched with, and republishes
/// it on every listing, so a replaced conversation is caught when Proqi
/// revalidates the target immediately before submission.
fn session_binding(row: &SessionRow) -> AgentSessionBinding {
    nonblank(row.agent_session_id.as_deref())
        .and_then(AgentSessionBinding::established)
        .unwrap_or_else(AgentSessionBinding::provisional)
}

/// Project thurbox's honest state vocabulary onto Proqi's readiness contract.
///
/// `running` is a positive observation: the probe found a registered agent
/// holding the pane and thurbox cannot read that agent's own state. Delivery is
/// supported, and the readiness stays `unknown` rather than claiming a report
/// that does not exist. `unreported` and `uncovered` confirm nothing, so they
/// stay ineligible.
fn live_state(row: &SessionRow) -> (AgentState, AgentAvailability) {
    if row.stopped == Some(true) {
        return (AgentState::Unknown, AgentAvailability::NotInteractive);
    }
    match nonblank(row.state.as_deref()) {
        Some("idle") => (AgentState::Idle, AgentAvailability::Available),
        Some("working") => (AgentState::Working, AgentAvailability::Available),
        Some("done") => (AgentState::Done, AgentAvailability::Available),
        Some("running") => (AgentState::Unknown, AgentAvailability::Available),
        Some("blocked") => (AgentState::Blocked, AgentAvailability::Blocked),
        Some("stopped") => (AgentState::Unknown, AgentAvailability::NotInteractive),
        _ => (AgentState::Unknown, AgentAvailability::Unknown),
    }
}

fn nonblank(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
