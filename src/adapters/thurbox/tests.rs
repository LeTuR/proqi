//! Deterministic thurbox discovery, submission, and failure contracts.

use std::{cell::RefCell, collections::VecDeque, ffi::OsString, rc::Rc};

use serde_json::{Value, json};

use crate::ports::{
    agent::{
        AgentAvailability, AgentError, AgentFailureCode, AgentGateway, AgentState,
        SubmissionRouteKind,
    },
    environment::{ProcessError, ProcessOutput, ProcessRequest, ProcessRunner},
};

use super::ThurboxGateway;

#[path = "tests/submission.rs"]
mod submission;

#[derive(Clone, Default)]
struct FakeRunner {
    responses: Rc<RefCell<VecDeque<FakeResponse>>>,
    requests: Rc<RefCell<Vec<ProcessRequest>>>,
}

impl FakeRunner {
    fn with(responses: Vec<FakeResponse>) -> Self {
        Self {
            responses: Rc::new(RefCell::new(responses.into())),
            requests: Rc::default(),
        }
    }
}

impl ProcessRunner for FakeRunner {
    fn run(&mut self, request: ProcessRequest) -> Result<ProcessOutput, ProcessError> {
        self.requests.borrow_mut().push(request);
        match self
            .responses
            .borrow_mut()
            .pop_front()
            .expect("recorded thurbox response")
        {
            FakeResponse::Output(output) => Ok(output),
            FakeResponse::Error(error) => Err(error),
        }
    }
}

enum FakeResponse {
    Output(ProcessOutput),
    Error(ProcessError),
}

fn success(value: &Value) -> FakeResponse {
    FakeResponse::Output(ProcessOutput {
        exit_code: Some(0),
        stdout: serde_json::to_vec(value).expect("fixture JSON"),
        stderr: Vec::new(),
    })
}

/// thurbox prints its failure object on standard output and leaves standard
/// error empty, so the fixture reproduces that exact shape.
fn refusal(message: &str) -> FakeResponse {
    FakeResponse::Output(ProcessOutput {
        exit_code: Some(1),
        stdout: serde_json::to_vec(&json!({"error": message, "suggestion": "check the state"}))
            .expect("fixture JSON"),
        stderr: Vec::new(),
    })
}

fn version() -> Value {
    json!({
        "data_dir": "/home/example/.local/share/thurbox",
        "schema_version": 45,
        "tmux_socket": "thurbox",
        "version": "2.19.0",
    })
}

fn session(id: &str, agent: &str, state: &str) -> Value {
    json!({
        "id": id,
        "name": format!("session {id}"),
        "cwd": "/home/example/code/proqi",
        "agent": agent,
        "agent_session_id": format!("conversation-{id}"),
        "detected_agent": null,
        "reports_as": null,
        "state": state,
        "hook_coverage": "full",
        "stopped": false,
    })
}

fn shell(id: &str, detected_agent: Option<&str>, state: &str) -> Value {
    json!({
        "id": id,
        "name": format!("shell {id}"),
        "cwd": "/home/example/code/proqi",
        "agent": "zsh",
        "agent_session_id": format!("conversation-{id}"),
        "detected_agent": detected_agent,
        "reports_as": null,
        "state": state,
        "hook_coverage": "none",
        "stopped": false,
    })
}

fn gateway(responses: Vec<FakeResponse>) -> (ThurboxGateway<FakeRunner>, FakeRunner) {
    let runner = FakeRunner::with(responses);
    (
        ThurboxGateway::new(OsString::from("thurbox-cli"), runner.clone(), true),
        runner,
    )
}

#[test]
fn capabilities_report_the_installed_release_and_no_pane_adjacency() {
    let (mut gateway, runner) = gateway(vec![success(&version())]);
    let capabilities = gateway.capabilities().expect("capabilities");
    assert_eq!(capabilities.provider, "thurbox");
    assert_eq!(capabilities.version, "2.19.0");
    assert_eq!(capabilities.protocol, 45);
    assert_eq!(capabilities.context, None);
    let requests = runner.requests.borrow();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].args, ["version", "--json"]);
}

#[test]
fn adjacency_is_reported_as_unsupported_rather_than_answered_with_nothing() {
    let (mut gateway, _) = gateway(Vec::new());
    let error = gateway
        .adjacent_targets(&crate::ports::agent::PaneContext {
            workspace_id: "w1".to_owned(),
            tab_id: "w1:t1".to_owned(),
            pane_id: "w1:p1".to_owned(),
            rect: crate::ports::agent::PaneRect {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
        })
        .expect_err("thurbox publishes no adjacency");
    assert_eq!(error.stable_code(), AgentFailureCode::Unsupported);
}

#[test]
fn discovery_verifies_panes_and_addresses_each_session_by_its_complete_identity() {
    let (mut gateway, runner) = gateway(vec![
        success(&version()),
        success(&json!([session("session-one", "claude", "idle")])),
    ]);
    let targets = gateway.global_targets().expect("verified targets");
    let [target] = targets.as_slice() else {
        panic!("expected one verified target");
    };
    assert_eq!(target.provider, "thurbox");
    assert_eq!(target.protocol, 45);
    assert_eq!(target.route.kind(), SubmissionRouteKind::ThurboxSession);
    assert_eq!(target.delivery_id(), "session-one");
    assert!(target.scope().is_empty());
    assert_eq!(target.agent_kind().as_str(), "claude");
    assert_eq!(
        target.agent_session().as_id(),
        Some("conversation-session-one")
    );
    assert_eq!(target.agent_name, "session session-one");
    assert_eq!(
        target.location,
        [
            "/home/example/code/proqi".to_owned(),
            "session-one".to_owned()
        ]
    );
    assert_eq!(target.readiness, AgentState::Idle);
    assert!(target.can_submit());
    let requests = runner.requests.borrow();
    assert_eq!(
        requests[1].args,
        ["session", "list", "--json", "--verify"],
        "discovery must probe each pane so a foreign agent stays visible"
    );
}

#[test]
fn the_honest_state_vocabulary_maps_onto_truthful_readiness_and_eligibility() {
    let rows = json!([
        session("idle", "claude", "idle"),
        session("working", "claude", "working"),
        session("done", "claude", "done"),
        session("blocked", "claude", "blocked"),
        session("unreported", "claude", "unreported"),
        shell("running", Some("claude"), "running"),
    ]);
    let (mut gateway, _) = gateway(vec![success(&version()), success(&rows)]);
    let targets = gateway.global_targets().expect("verified targets");
    let observed = targets
        .iter()
        .map(|target| {
            (
                target.delivery_id().to_owned(),
                target.readiness,
                target.availability,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        observed,
        vec![
            (
                "idle".to_owned(),
                AgentState::Idle,
                AgentAvailability::Available
            ),
            (
                "working".to_owned(),
                AgentState::Working,
                AgentAvailability::Available
            ),
            (
                "done".to_owned(),
                AgentState::Done,
                AgentAvailability::Available
            ),
            (
                "blocked".to_owned(),
                AgentState::Blocked,
                AgentAvailability::Blocked
            ),
            (
                "unreported".to_owned(),
                AgentState::Unknown,
                AgentAvailability::Unknown
            ),
            // A probed foreign agent is confirmed present, so delivery is
            // supported while its own state stays honestly unknown.
            (
                "running".to_owned(),
                AgentState::Unknown,
                AgentAvailability::Available
            ),
        ]
    );
}

#[test]
fn rows_that_agree_through_their_bounded_names_still_say_which_session_they_are() {
    let long = "fm-firstmate-810a5b6d-thurbox-review-";
    let rows = json!([
        {
            "id": "aaaaaaaa-0b00-4000-8000-000000000001",
            "name": format!("{long}one"),
            "cwd": "/home/example/code/proqi",
            "agent": "claude",
            "agent_session_id": "conversation-one",
            "state": "idle",
            "hook_coverage": "full",
            "stopped": false,
        },
        {
            "id": "aaaaaaaa-0c00-4000-8000-000000000002",
            "name": format!("{long}two"),
            "cwd": "/home/example/code/proqi",
            "agent": "claude",
            "agent_session_id": "conversation-two",
            "state": "idle",
            "hook_coverage": "full",
            "stopped": false,
        },
    ]);
    let (mut gateway, _) = gateway(vec![success(&version()), success(&rows)]);
    let targets = gateway.global_targets().expect("verified targets");
    assert_eq!(
        targets[0].agent_name, targets[1].agent_name,
        "the bounded names collide, which is what makes the identity necessary"
    );
    assert_ne!(targets[0].location, targets[1].location);
    assert_eq!(
        targets
            .iter()
            .map(|target| target.location.last().cloned().unwrap_or_default())
            .collect::<Vec<_>>(),
        ["aaaaaaaa-0b".to_owned(), "aaaaaaaa-0c".to_owned()],
        "the prefix widens only as far as separating the listed rows requires"
    );
}

#[test]
fn a_shell_without_an_observed_agent_is_not_offered_as_a_target() {
    let rows = json!([
        shell("bare", None, "uncovered"),
        shell("occupied", Some("codex"), "running"),
    ]);
    let (mut gateway, _) = gateway(vec![success(&version()), success(&rows)]);
    let targets = gateway.global_targets().expect("verified targets");
    let [target] = targets.as_slice() else {
        panic!("only the occupied shell is a coding agent");
    };
    assert_eq!(target.delivery_id(), "occupied");
    assert_eq!(target.agent_kind().as_str(), "codex");
}

#[test]
fn a_parked_session_is_listed_without_becoming_eligible() {
    let mut parked = session("parked", "claude", "stopped");
    parked["stopped"] = json!(true);
    let (mut gateway, _) = gateway(vec![success(&version()), success(&json!([parked]))]);
    let targets = gateway.global_targets().expect("verified targets");
    let [target] = targets.as_slice() else {
        panic!("expected the parked session");
    };
    assert_eq!(target.availability, AgentAvailability::NotInteractive);
    assert!(!target.can_submit());
}

#[test]
fn the_session_hosting_proqi_is_never_offered_as_its_own_target() {
    let runner = FakeRunner::with(vec![
        success(&version()),
        success(&json!([
            session("self", "claude", "idle"),
            session("other", "claude", "idle"),
        ])),
    ]);
    let mut gateway = ThurboxGateway::new(OsString::from("thurbox-cli"), runner, true)
        .with_current_session(Some("self".to_owned()));
    let targets = gateway.global_targets().expect("verified targets");
    let [target] = targets.as_slice() else {
        panic!("expected only the other session");
    };
    assert_eq!(target.delivery_id(), "other");
}

#[test]
fn duplicate_and_incomplete_session_identities_fail_closed() {
    let duplicate = session("same", "claude", "idle");
    let (mut ambiguous, _) = gateway(vec![
        success(&version()),
        success(&json!([duplicate.clone(), duplicate])),
    ]);
    assert_eq!(
        ambiguous
            .global_targets()
            .expect_err("duplicate identity")
            .stable_code(),
        AgentFailureCode::Ambiguous
    );

    let mut blank = session("placeholder", "claude", "idle");
    blank["id"] = json!("   ");
    let (mut malformed, _) = gateway(vec![success(&version()), success(&json!([blank]))]);
    assert_eq!(
        malformed
            .global_targets()
            .expect_err("incomplete identity")
            .stable_code(),
        AgentFailureCode::Malformed
    );
}

#[test]
fn discovery_refuses_a_listing_beyond_the_verified_row_budget() {
    let rows = (0..=super::discovery::MAX_SESSION_ROWS)
        .map(|index| session(&format!("session-{index}"), "claude", "idle"))
        .collect::<Vec<_>>();
    let (mut gateway, _) = gateway(vec![success(&version()), success(&json!(rows))]);
    assert_eq!(
        gateway
            .global_targets()
            .expect_err("row budget")
            .stable_code(),
        AgentFailureCode::Unsupported
    );
}

#[test]
fn a_failure_printed_on_standard_output_is_never_read_as_a_successful_response() {
    let (mut gateway, _) = gateway(vec![refusal("thurbox database is locked")]);
    let error = gateway.global_targets().expect_err("refusal");
    assert_eq!(
        error,
        AgentError::Rejected {
            code: "thurbox_error".to_owned(),
            message: "thurbox database is locked".to_owned(),
        }
    );
}

#[test]
fn an_unreachable_cli_is_reported_as_unavailable_rather_than_a_process_failure() {
    let (mut gateway, _) = gateway(vec![FakeResponse::Error(ProcessError::Io(
        "No such file or directory".to_owned(),
    ))]);
    assert_eq!(
        gateway
            .global_targets()
            .expect_err("missing executable")
            .stable_code(),
        AgentFailureCode::Unavailable
    );
}

#[test]
fn an_unqualified_release_is_refused_before_any_session_is_listed() {
    let mut old = version();
    old["version"] = json!("2.18.0");
    let (mut gateway, runner) = gateway(vec![success(&old)]);
    assert_eq!(
        gateway
            .global_targets()
            .expect_err("unqualified release")
            .stable_code(),
        AgentFailureCode::Unsupported
    );
    assert_eq!(runner.requests.borrow().len(), 1);
}

#[test]
fn a_disabled_integration_reports_unavailable_without_running_anything() {
    let runner = FakeRunner::default();
    let mut gateway = ThurboxGateway::new(OsString::from("thurbox-cli"), runner.clone(), false);
    assert_eq!(
        gateway
            .global_targets()
            .expect_err("disabled integration")
            .stable_code(),
        AgentFailureCode::Unavailable
    );
    assert!(runner.requests.borrow().is_empty());
}
