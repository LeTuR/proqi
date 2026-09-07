use super::*;

use proqi::ports::agent::{
    AgentAddress, AgentAvailability, AgentDeliveryCapabilities, AgentSessionBinding, AgentState,
    AgentTarget, HarnessKind, SubmissionRouteKind,
};

use super::global_agent_delivery::{open, prepare, target};

fn thurbox_target(
    session_id: &str,
    cwd: &str,
    name: &str,
    readiness: AgentState,
    availability: AgentAvailability,
) -> AgentTarget {
    AgentTarget::thurbox_session(
        45,
        AgentAddress::new(
            Vec::new(),
            session_id.to_owned(),
            HarnessKind::new("claude").expect("harness"),
            AgentSessionBinding::established(format!("conversation-{session_id}"))
                .expect("session"),
        )
        .expect("address"),
        name.to_owned(),
        vec![cwd.to_owned(), session_id[..8].to_owned()],
        readiness,
        availability,
        AgentDeliveryCapabilities::SUBMIT_ONLY,
    )
}

#[test]
fn one_picker_lists_every_integration_and_delivers_each_by_its_own_route() {
    let mut fixture = Fixture::new();
    prepare(&mut fixture, "source");
    let generation = open(&mut fixture);
    fixture.app.complete_global_agent_discovery(
        generation,
        Ok(vec![
            target(
                "w2",
                "w2:t1",
                "w2:p8",
                "herdr receiver",
                AgentState::Idle,
                AgentAvailability::Available,
            ),
            thurbox_target(
                "44444444-4444-4444-8444-444444444444",
                "/home/example/code/proqi",
                "thurbox crewmate",
                AgentState::Unknown,
                AgentAvailability::Available,
            ),
        ]),
    );

    let rendered = text(draw(&mut fixture, 96, 12).backend().buffer());
    assert!(rendered.contains("herdr receiver"), "{rendered}");
    assert!(
        rendered.contains("Workspace w2 / Tab w2:t1 · p8"),
        "{rendered}"
    );
    assert!(rendered.contains("thurbox crewmate"), "{rendered}");
    assert!(
        rendered.contains("44444444 · unknown"),
        "the session identity and its honest state survive a narrowed row: {rendered}"
    );
    assert!(
        !rendered.contains("proqi · idle"),
        "an unreadable harness state is never presented as idle: {rendered}"
    );

    // The working directory identifies the session in search even when the row
    // has no room to show it beside the identity and state.
    for character in "code/proqi".chars() {
        fixture.input(crate::key_input(UiKey::Character(character)));
    }
    let narrowed = text(draw(&mut fixture, 96, 12).backend().buffer());
    assert!(narrowed.contains("thurbox crewmate"), "{narrowed}");
    assert!(!narrowed.contains("herdr receiver"), "{narrowed}");

    fixture.input(crate::key_input(UiKey::Enter));
    let effects = fixture.effects(crate::key_input(UiKey::Enter));
    let [Effect::PrepareSubmission(attempt)] = effects.as_slice() else {
        panic!("expected durable reservation: {effects:?}");
    };
    assert_eq!(attempt.route.kind(), SubmissionRouteKind::ThurboxSession);
    assert_eq!(attempt.route.adjacent_direction(), None);
    assert_eq!(attempt.provider, "thurbox");
    let request = super::agent::start_submission(&mut fixture, &effects);
    assert_eq!(request.content, "source");
    assert_eq!(
        request.target.delivery_id(),
        "44444444-4444-4444-8444-444444444444"
    );
}
