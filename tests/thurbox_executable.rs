#![cfg(unix)]
//! End-to-end thurbox discovery and submission contracts through a fake executable.

use proqi::{
    adapters::{memory::FakeIdGenerator, process::SystemProcessRunner, thurbox::ThurboxGateway},
    ports::{
        agent::{
            AgentAvailability, AgentFailureCode, AgentGateway, AgentState, SubmissionRequest,
            SubmissionRouteKind,
        },
        environment::IdGenerator,
    },
};

#[path = "support/thurbox.rs"]
mod thurbox_fixture;

#[test]
fn a_fake_executable_proves_the_qualified_session_and_send_cli_contract() {
    let fixture = thurbox_fixture::ThurboxFixture::new("2.19.0");
    let mut gateway = ThurboxGateway::new(fixture.program(), SystemProcessRunner::default(), true);

    let capabilities = gateway.capabilities().expect("capabilities");
    assert_eq!(capabilities.provider, "thurbox");
    assert_eq!(capabilities.version, "2.19.0");
    assert_eq!(capabilities.protocol, 45);
    assert_eq!(capabilities.context, None);

    let targets = gateway.global_targets().expect("verified targets");
    assert_eq!(
        gateway.global_targets().expect("repeated verified targets"),
        targets
    );
    assert_eq!(
        targets
            .iter()
            .map(|target| (target.delivery_id(), target.agent_kind().as_str()))
            .collect::<Vec<_>>(),
        [
            ("11111111-1111-4111-8111-111111111111", "claude"),
            ("22222222-2222-4222-8222-222222222222", "codex"),
        ],
        "a plain shell with no observed agent is not a delivery target"
    );

    let foreign = targets
        .iter()
        .find(|target| target.delivery_id() == "22222222-2222-4222-8222-222222222222")
        .expect("foreign-launched session");
    assert_eq!(foreign.route.kind(), SubmissionRouteKind::ThurboxSession);
    assert_eq!(foreign.readiness, AgentState::Unknown);
    assert_eq!(foreign.availability, AgentAvailability::Available);
    assert_eq!(
        foreign.location,
        ["/tmp/crewmate".to_owned(), "22222222".to_owned()],
        "a long session identity is abbreviated for the row, never for delivery"
    );

    let mut ids = FakeIdGenerator::new(1_725_200_000_000);
    let exact = "$(touch never); Grüße\n第二行\u{1b}[31m";
    let receipt = gateway
        .submit(SubmissionRequest {
            submission_id: ids.submission_id(),
            target: foreign.clone(),
            content: exact.to_owned(),
        })
        .expect("accepted prompt");
    assert_eq!(receipt.target, *foreign);
    assert_eq!(fixture.sent_bytes().as_deref(), Some(exact.as_bytes()));
}

#[test]
fn an_unqualified_release_is_refused_without_listing_a_session() {
    let fixture = thurbox_fixture::ThurboxFixture::new("2.18.9");
    let mut gateway = ThurboxGateway::new(fixture.program(), SystemProcessRunner::default(), true);
    assert_eq!(
        gateway
            .global_targets()
            .expect_err("unqualified release")
            .stable_code(),
        AgentFailureCode::Unsupported
    );
}
