//! Recognition-only integration metadata mapping.

pub(super) fn integration_context(
    target: &crate::ports::agent::AgentTarget,
    verified_at: crate::domain::Timestamp,
) -> Option<crate::domain::IntegrationContext> {
    let direction = target.adjacent_direction()?;
    let mut scope = target.scope().iter();
    Some(crate::domain::IntegrationContext {
        provider: target.provider.clone(),
        direction,
        agent_kind: target.agent_kind().as_str().to_owned(),
        agent_name: target.agent_name.clone(),
        workspace_hint: scope.next().cloned(),
        tab_hint: scope.next().cloned(),
        pane_hint: Some(target.delivery_id().to_owned()),
        verified_at,
    })
}
