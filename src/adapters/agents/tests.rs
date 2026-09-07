//! Composition contracts across the installed agent integrations.

use crate::{
    adapters::memory::FakeIdGenerator,
    ports::{
        agent::{
            AgentAddress, AgentAvailability, AgentCapabilities, AgentDeliveryCapabilities,
            AgentError, AgentFailureCode, AgentGateway, AgentSessionBinding, AgentState,
            AgentTarget, HarnessKind, PaneContext, PaneRect, SubmissionReceipt, SubmissionRequest,
        },
        environment::IdGenerator,
    },
};

use super::CompositeAgentGateway;

#[derive(Default)]
struct FakeGateway {
    provider: &'static str,
    global: Option<Result<Vec<AgentTarget>, AgentError>>,
    submitted: Vec<String>,
}

impl FakeGateway {
    fn new(provider: &'static str, global: Result<Vec<AgentTarget>, AgentError>) -> Self {
        Self {
            provider,
            global: Some(global),
            submitted: Vec::new(),
        }
    }
}

impl AgentGateway for FakeGateway {
    fn capabilities(&mut self) -> Result<AgentCapabilities, AgentError> {
        Ok(AgentCapabilities {
            provider: self.provider.to_owned(),
            version: "1.0.0".to_owned(),
            protocol: 1,
            delivery: AgentDeliveryCapabilities::SUBMIT_ONLY,
            context: None,
        })
    }

    fn adjacent_targets(&mut self, _context: &PaneContext) -> Result<Vec<AgentTarget>, AgentError> {
        Ok(Vec::new())
    }

    fn global_targets(&mut self) -> Result<Vec<AgentTarget>, AgentError> {
        self.global.clone().unwrap_or_else(|| Ok(Vec::new()))
    }

    fn submit(&mut self, request: SubmissionRequest) -> Result<SubmissionReceipt, AgentError> {
        self.submitted.push(self.provider.to_owned());
        Ok(SubmissionReceipt {
            submission_id: request.submission_id,
            target: request.target,
            post_state: None,
        })
    }
}

fn address(id: &str) -> AgentAddress {
    AgentAddress::new(
        Vec::new(),
        id.to_owned(),
        HarnessKind::new("claude").expect("fixture harness"),
        AgentSessionBinding::established(format!("conversation-{id}")).expect("fixture session"),
    )
    .expect("fixture address")
}

fn herdr_target(pane: &str) -> AgentTarget {
    AgentTarget::herdr_agent(
        20,
        AgentAddress::new(
            vec!["w1".to_owned(), "w1:t1".to_owned()],
            pane.to_owned(),
            HarnessKind::new("codex").expect("fixture harness"),
            AgentSessionBinding::established("herdr-session").expect("fixture session"),
        )
        .expect("fixture address"),
        "reviewer".to_owned(),
        vec!["Workspace".to_owned(), "Tab".to_owned()],
        AgentState::Idle,
        AgentAvailability::Available,
        AgentDeliveryCapabilities::SUBMIT_ONLY,
    )
}

fn thurbox_target(session: &str) -> AgentTarget {
    AgentTarget::thurbox_session(
        45,
        address(session),
        "crewmate".to_owned(),
        vec!["/home/example/code".to_owned()],
        AgentState::Idle,
        AgentAvailability::Available,
        AgentDeliveryCapabilities::SUBMIT_ONLY,
    )
}

fn adjacent_target() -> AgentTarget {
    let source = PaneContext {
        workspace_id: "w1".to_owned(),
        tab_id: "w1:t1".to_owned(),
        pane_id: "w1:p1".to_owned(),
        rect: PaneRect {
            x: 0,
            y: 0,
            width: 20,
            height: 20,
        },
    };
    AgentTarget::adjacent(
        "herdr".to_owned(),
        20,
        crate::domain::Direction::Right,
        AgentAddress::new(
            vec![source.workspace_id.clone(), source.tab_id.clone()],
            "w1:p2".to_owned(),
            HarnessKind::new("codex").expect("fixture harness"),
            AgentSessionBinding::established("herdr-session").expect("fixture session"),
        )
        .expect("fixture address"),
        "neighbor".to_owned(),
        AgentState::Idle,
        AgentDeliveryCapabilities::SUBMIT_ONLY,
        source.rect,
        source,
    )
}

fn composite(
    herdr: Result<Vec<AgentTarget>, AgentError>,
    thurbox: Result<Vec<AgentTarget>, AgentError>,
) -> CompositeAgentGateway<FakeGateway, FakeGateway> {
    CompositeAgentGateway::new(
        FakeGateway::new("herdr", herdr),
        FakeGateway::new("thurbox", thurbox),
    )
}

#[test]
fn global_discovery_spans_every_installed_integration() {
    let mut gateway = composite(
        Ok(vec![herdr_target("w1:p2")]),
        Ok(vec![thurbox_target("session-one")]),
    );
    let targets = gateway.global_targets().expect("merged targets");
    assert_eq!(
        targets
            .iter()
            .map(|target| target.provider.as_str())
            .collect::<Vec<_>>(),
        ["herdr", "thurbox"]
    );
}

#[test]
fn an_integration_that_is_absent_or_cannot_serve_leaves_the_others_answering() {
    for silent in [
        AgentError::Unavailable("thurbox-cli is not available".to_owned()),
        AgentError::Unsupported("thurbox 2.18.0 is not qualified".to_owned()),
    ] {
        let mut gateway = composite(Ok(vec![herdr_target("w1:p2")]), Err(silent));
        let targets = gateway.global_targets().expect("Herdr answers alone");
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].provider, "herdr");
    }
}

#[test]
fn an_untrustworthy_answer_fails_the_whole_pass_closed() {
    let mut gateway = composite(
        Ok(vec![herdr_target("w1:p2")]),
        Err(AgentError::Ambiguous(
            "two sessions claim one id".to_owned(),
        )),
    );
    assert_eq!(
        gateway
            .global_targets()
            .expect_err("a partial list is never presented as complete")
            .stable_code(),
        AgentFailureCode::Ambiguous
    );
}

#[test]
fn an_integration_answering_with_nothing_still_answers() {
    let mut gateway = composite(
        Ok(Vec::new()),
        Err(AgentError::Unavailable(
            "thurbox-cli is not available".to_owned(),
        )),
    );
    assert_eq!(
        gateway
            .global_targets()
            .expect("an empty verified list is an answer, not a failure"),
        Vec::new()
    );
}

#[test]
fn no_installed_integration_reports_the_first_reason_it_cannot_serve() {
    let mut gateway = composite(
        Err(AgentError::Unavailable("HERDR_ENV is not set".to_owned())),
        Err(AgentError::Unsupported("thurbox is too old".to_owned())),
    );
    assert_eq!(
        gateway
            .global_targets()
            .expect_err("nothing to discover")
            .stable_code(),
        AgentFailureCode::Unavailable
    );
}

#[test]
fn submission_returns_to_the_integration_that_verified_the_route() {
    let mut ids = FakeIdGenerator::new(1_725_200_000_000);
    for (target, expected) in [
        (adjacent_target(), "herdr"),
        (herdr_target("w1:p2"), "herdr"),
        (thurbox_target("session-one"), "thurbox"),
    ] {
        let mut gateway = composite(Ok(Vec::new()), Ok(Vec::new()));
        gateway
            .submit(SubmissionRequest {
                submission_id: ids.submission_id(),
                target,
                content: "prompt".to_owned(),
            })
            .expect("accepted prompt");
        assert_eq!(
            gateway.herdr.submitted.len(),
            usize::from(expected == "herdr")
        );
        assert_eq!(
            gateway.thurbox.submitted.len(),
            usize::from(expected == "thurbox")
        );
    }
}

#[test]
fn pane_adjacency_and_display_metadata_stay_with_the_multiplexer() {
    let mut gateway = composite(Ok(Vec::new()), Ok(Vec::new()));
    assert_eq!(
        gateway.capabilities().expect("capabilities").provider,
        "herdr"
    );
    let source = PaneContext {
        workspace_id: "w1".to_owned(),
        tab_id: "w1:t1".to_owned(),
        pane_id: "w1:p1".to_owned(),
        rect: PaneRect {
            x: 0,
            y: 0,
            width: 20,
            height: 20,
        },
    };
    assert_eq!(
        gateway.adjacent_targets(&source).expect("adjacent targets"),
        Vec::new()
    );
}
