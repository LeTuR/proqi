//! Composite gateway over every installed adjacent-agent integration.

#[cfg(test)]
mod tests;

use std::time::Duration;

use crate::{
    adapters::{herdr::HerdrGateway, process::SystemProcessRunner, thurbox::ThurboxGateway},
    ports::{
        agent::{
            AgentCapabilities, AgentError, AgentFailureCode, AgentGateway, AgentTarget,
            PaneContext, PanePresentation, SubmissionReceipt, SubmissionRequest, SubmissionRoute,
        },
        invocation::{InvocationReferenceCatalog, InvocationReferenceSnapshot},
    },
};

/// Every installed integration behind one provider-independent boundary.
///
/// Herdr owns pane adjacency and Proqi's own display metadata, because those
/// are properties of the multiplexer hosting this pane. Global discovery spans
/// both integrations, and submission returns to the integration that verified
/// the route.
pub struct CompositeAgentGateway<H, T> {
    herdr: H,
    thurbox: T,
}

impl<H, T> CompositeAgentGateway<H, T> {
    /// Construct an injectable composite for composition or contract tests.
    pub const fn new(herdr: H, thurbox: T) -> Self {
        Self { herdr, thurbox }
    }
}

impl CompositeAgentGateway<HerdrGateway<SystemProcessRunner>, ThurboxGateway<SystemProcessRunner>> {
    /// Compose the installed integrations from the inherited environment.
    pub(crate) fn from_environment_with_runner(
        presentation_source: String,
        runner: SystemProcessRunner,
    ) -> Self {
        Self::new(
            HerdrGateway::from_environment_with_runner(presentation_source, runner.clone()),
            ThurboxGateway::from_environment_with_runner(runner),
        )
    }
}

impl<H: AgentGateway, T: AgentGateway> AgentGateway for CompositeAgentGateway<H, T> {
    fn capabilities(&mut self) -> Result<AgentCapabilities, AgentError> {
        self.herdr.capabilities()
    }

    fn adjacent_targets(&mut self, context: &PaneContext) -> Result<Vec<AgentTarget>, AgentError> {
        self.herdr.adjacent_targets(context)
    }

    fn global_targets(&mut self) -> Result<Vec<AgentTarget>, AgentError> {
        merge_global(self.herdr.global_targets(), self.thurbox.global_targets())
    }

    fn submit(&mut self, request: SubmissionRequest) -> Result<SubmissionReceipt, AgentError> {
        match request.target.route {
            SubmissionRoute::AdjacentPane { .. } | SubmissionRoute::HerdrAgent(_) => {
                self.herdr.submit(request)
            }
            SubmissionRoute::ThurboxSession(_) => self.thurbox.submit(request),
        }
    }
}

/// Combine one discovery pass per integration into one truthful list.
///
/// An integration that reports `Unavailable` or `Unsupported` has said it has
/// nothing to offer here, so the remaining integrations answer alone. Every
/// other failure means an answer that could not be trusted, and Proqi refuses
/// to present a list it cannot describe as complete.
fn merge_global(
    herdr: Result<Vec<AgentTarget>, AgentError>,
    thurbox: Result<Vec<AgentTarget>, AgentError>,
) -> Result<Vec<AgentTarget>, AgentError> {
    let mut merged = Vec::new();
    let mut answered = false;
    let mut silent: Option<AgentError> = None;
    for outcome in [herdr, thurbox] {
        match outcome {
            Ok(targets) => {
                answered = true;
                merged.extend(targets);
            }
            Err(error) if contributes_nothing(&error) => silent = silent.or(Some(error)),
            Err(error) => return Err(error),
        }
    }
    match silent {
        Some(error) if !answered => Err(error),
        _ => Ok(merged),
    }
}

const fn contributes_nothing(error: &AgentError) -> bool {
    matches!(
        error.stable_code(),
        AgentFailureCode::Unavailable | AgentFailureCode::Unsupported
    )
}

impl<H: InvocationReferenceCatalog, T: Send> InvocationReferenceCatalog
    for CompositeAgentGateway<H, T>
{
    fn discover_live_references(
        &mut self,
    ) -> Result<InvocationReferenceSnapshot, AgentFailureCode> {
        self.herdr.discover_live_references()
    }
}

impl<H: PanePresentation, T> PanePresentation for CompositeAgentGateway<H, T> {
    fn publish(&mut self, pane_id: &str, sequence: u64, ttl: Duration) -> Result<(), AgentError> {
        self.herdr.publish(pane_id, sequence, ttl)
    }

    fn clear(&mut self, pane_id: &str, sequence: u64) -> Result<(), AgentError> {
        self.herdr.clear(pane_id, sequence)
    }
}
