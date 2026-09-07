//! Bounded, content-redacted topology labels for live invocation references.

use std::collections::BTreeMap;

use crate::ports::agent::{
    AgentError, MAX_AGENT_NAME_CHARS, MAX_LOCATION_LABEL_CHARS, bounded_integration_label,
};

use super::super::contract::{TabInfo, WorkspaceInfo};

pub(super) fn workspace_labels(
    values: Vec<WorkspaceInfo>,
    maximum: usize,
) -> Result<BTreeMap<String, Option<String>>, AgentError> {
    let mut labels = BTreeMap::new();
    for value in values.into_iter().take(maximum) {
        if labels
            .insert(
                value.workspace_id,
                bounded_topology_label(value.label.as_deref()),
            )
            .is_some()
        {
            return Err(AgentError::Ambiguous(
                "duplicate workspace identity in Herdr snapshot".to_owned(),
            ));
        }
    }
    Ok(labels)
}

pub(super) fn tab_labels(
    values: Vec<TabInfo>,
    maximum: usize,
) -> Result<BTreeMap<String, (String, Option<String>)>, AgentError> {
    let mut labels = BTreeMap::new();
    for value in values.into_iter().take(maximum) {
        if labels
            .insert(
                value.tab_id,
                (
                    value.workspace_id,
                    bounded_topology_label(value.label.as_deref()),
                ),
            )
            .is_some()
        {
            return Err(AgentError::Ambiguous(
                "duplicate tab identity in Herdr snapshot".to_owned(),
            ));
        }
    }
    Ok(labels)
}

pub(super) fn correlated_workspace_label(
    workspaces: &BTreeMap<String, Option<String>>,
    workspace_id: &str,
    truncated: bool,
) -> Result<Option<String>, AgentError> {
    if workspaces.is_empty() {
        return Ok(None);
    }
    match workspaces.get(workspace_id).cloned() {
        Some(label) => Ok(label),
        None if truncated => Ok(None),
        None => Err(AgentError::Malformed(
            "recognized agent has no matching workspace identity".to_owned(),
        )),
    }
}

pub(super) fn correlated_tab_label(
    tabs: &BTreeMap<String, (String, Option<String>)>,
    workspace_id: &str,
    tab_id: &str,
    truncated: bool,
) -> Result<Option<String>, AgentError> {
    if tabs.is_empty() {
        return Ok(None);
    }
    let Some((tab_workspace, label)) = tabs.get(tab_id) else {
        return if truncated {
            Ok(None)
        } else {
            Err(AgentError::Malformed(
                "recognized agent has no matching tab identity".to_owned(),
            ))
        };
    };
    if tab_workspace != workspace_id {
        return Err(AgentError::Malformed(
            "recognized agent and tab belong to different workspaces".to_owned(),
        ));
    }
    Ok(label.clone())
}

/// Return the display location of one recognized agent, outermost first.
///
/// The last segment identifies the pane itself, which is what a narrow row
/// keeps. Herdr spells a child identity as `<parent>:<child>`, so the shared
/// prefix is redundant beside the workspace it is already shown under.
pub(super) fn display_location(
    workspace_id: &str,
    workspace_label: Option<String>,
    tab_id: &str,
    tab_label: Option<String>,
    pane_id: &str,
) -> Vec<String> {
    vec![
        workspace_label.unwrap_or_else(|| workspace_id.to_owned()),
        tab_label.unwrap_or_else(|| tab_id.to_owned()),
        compact_child(workspace_id, pane_id).to_owned(),
    ]
}

fn compact_child<'a>(workspace_id: &str, identity: &'a str) -> &'a str {
    identity
        .strip_prefix(workspace_id)
        .and_then(|suffix| suffix.strip_prefix(':'))
        .unwrap_or(identity)
}

pub(super) fn sanitize_agent_name(value: &str) -> Option<String> {
    bounded_integration_label(value, MAX_AGENT_NAME_CHARS)
}

fn bounded_topology_label(value: Option<&str>) -> Option<String> {
    bounded_integration_label(value?, MAX_LOCATION_LABEL_CHARS)
}
