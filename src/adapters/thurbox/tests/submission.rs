//! Revalidation and delivery contracts for one thurbox session.

use serde_json::json;

use crate::{
    adapters::memory::FakeIdGenerator,
    ports::{
        agent::{AgentFailureCode, AgentGateway as _, SubmissionRequest},
        environment::IdGenerator as _,
    },
};

use super::{gateway, session, success, version};

#[test]
fn submission_revalidates_and_passes_exact_text_as_one_distinct_argument() {
    let rows = json!([session("target", "claude", "idle")]);
    let (mut gateway, runner) = gateway(vec![
        success(&version()),
        success(&rows),
        success(&version()),
        success(&rows),
        success(&json!({
            "sent": true,
            "submitted": true,
            "session_id": "target",
            "session_name": "session target",
        })),
    ]);
    let targets = gateway.global_targets().expect("verified targets");
    let target = targets.first().expect("one target").clone();
    let mut ids = FakeIdGenerator::new(1_725_200_000_000);
    let exact = "$(touch never); Grüße\n第二行\u{1b}[31m";
    let receipt = gateway
        .submit(SubmissionRequest {
            submission_id: ids.submission_id(),
            target: target.clone(),
            content: exact.to_owned(),
        })
        .expect("accepted prompt");

    assert_eq!(receipt.target, target);
    assert_eq!(receipt.post_state, None);
    let requests = runner.requests.borrow();
    assert_eq!(
        requests[4].args[0..4],
        ["session", "send", "--json", "target"]
    );
    assert_eq!(requests[4].args[3], "target");
    assert_eq!(requests[4].args[4], exact);
}

#[test]
fn a_receipt_for_another_session_or_an_unsubmitted_paste_is_malformed() {
    let rows = json!([session("target", "claude", "idle")]);
    for receipt in [
        json!({"sent": true, "submitted": true, "session_id": "other"}),
        json!({"sent": true, "submitted": false, "session_id": "target"}),
        json!({"sent": false, "submitted": true, "session_id": "target"}),
    ] {
        let (mut gateway, _) = gateway(vec![
            success(&version()),
            success(&rows),
            success(&version()),
            success(&rows),
            success(&receipt),
        ]);
        let targets = gateway.global_targets().expect("verified targets");
        let target = targets.first().expect("one target").clone();
        let mut ids = FakeIdGenerator::new(1_725_200_000_000);
        let error = gateway
            .submit(SubmissionRequest {
                submission_id: ids.submission_id(),
                target,
                content: "prompt".to_owned(),
            })
            .expect_err("mismatched receipt");
        assert_eq!(error.stable_code(), AgentFailureCode::Malformed);
    }
}

#[test]
fn a_replaced_conversation_stops_the_submission_before_any_text_is_typed() {
    let discovered = json!([session("target", "claude", "idle")]);
    let mut replaced = session("target", "claude", "idle");
    replaced["agent_session_id"] = json!("conversation-replaced");
    let (mut gateway, runner) = gateway(vec![
        success(&version()),
        success(&discovered),
        success(&version()),
        success(&json!([replaced])),
    ]);
    let targets = gateway.global_targets().expect("verified targets");
    let target = targets.first().expect("one target").clone();
    let mut ids = FakeIdGenerator::new(1_725_200_000_000);
    let error = gateway
        .submit(SubmissionRequest {
            submission_id: ids.submission_id(),
            target,
            content: "prompt".to_owned(),
        })
        .expect_err("replaced conversation");
    assert_eq!(error.stable_code(), AgentFailureCode::Unsupported);
    assert_eq!(
        runner.requests.borrow().len(),
        4,
        "no text may be typed after a failed revalidation"
    );
}

#[test]
fn a_thurbox_upgrade_between_discovery_and_delivery_fails_closed() {
    let rows = json!([session("target", "claude", "idle")]);
    let mut upgraded = version();
    upgraded["schema_version"] = json!(46);
    upgraded["version"] = json!("2.20.0");
    let (mut gateway, runner) = gateway(vec![
        success(&version()),
        success(&rows),
        success(&upgraded),
        success(&rows),
    ]);
    let targets = gateway.global_targets().expect("verified targets");
    let target = targets.first().expect("one target").clone();
    let mut ids = FakeIdGenerator::new(1_725_200_000_000);
    let error = gateway
        .submit(SubmissionRequest {
            submission_id: ids.submission_id(),
            target,
            content: "prompt".to_owned(),
        })
        .expect_err("the published contract number changed");
    assert_eq!(error.stable_code(), AgentFailureCode::Unsupported);
    assert_eq!(
        runner.requests.borrow().len(),
        4,
        "no text may be typed after a failed revalidation"
    );
}
