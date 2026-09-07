//! Target revalidation and atomic thurbox prompt submission.

use crate::ports::{
    agent::{
        AgentError, AgentGateway as _, AgentTarget, SubmissionReceipt, SubmissionRequest,
        SubmissionRoute,
    },
    environment::ProcessRunner,
};

use super::{SUBMISSION_TIMEOUT, ThurboxGateway, contract::SendReceipt};

pub(super) fn submit<R: ProcessRunner>(
    gateway: &mut ThurboxGateway<R>,
    request: &SubmissionRequest,
) -> Result<SubmissionReceipt, AgentError> {
    if !matches!(request.target.route, SubmissionRoute::ThurboxSession(_)) {
        return Err(AgentError::Unsupported(
            "thurbox cannot deliver to a Herdr pane route".to_owned(),
        ));
    }
    let refreshed = gateway.global_targets()?;
    let verified = refreshed
        .into_iter()
        .filter(|target| {
            target.identity() == request.target.identity()
                && target.protocol == request.target.protocol
        })
        .collect::<Vec<_>>();
    let [target] = verified.as_slice() else {
        return Err(if verified.is_empty() {
            AgentError::Unsupported("target changed before submission".to_owned())
        } else {
            AgentError::Ambiguous("target identity is no longer unique".to_owned())
        });
    };
    if !target.can_submit() {
        return Err(AgentError::Unsupported(format!(
            "target is {} before submission",
            target.availability.as_str()
        )));
    }
    let receipt: SendReceipt = gateway.json(
        &[
            "session",
            "send",
            "--json",
            target.delivery_id(),
            &request.content,
        ],
        SUBMISSION_TIMEOUT,
    )?;
    verify_sent(target, &receipt)?;
    Ok(SubmissionReceipt {
        submission_id: request.submission_id,
        target: target.clone(),
        // thurbox types into the session's terminal and reports no state after
        // delivery, so Proqi records no advisory post-state rather than one it
        // would have to invent.
        post_state: None,
    })
}

fn verify_sent(target: &AgentTarget, receipt: &SendReceipt) -> Result<(), AgentError> {
    if receipt.sent && receipt.submitted && receipt.session_id == target.delivery_id() {
        return Ok(());
    }
    Err(AgentError::Malformed(
        "send receipt does not match the verified target".to_owned(),
    ))
}
