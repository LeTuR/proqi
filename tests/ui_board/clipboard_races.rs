//! Delayed clipboard completions that must stay bound to their source editor.

use super::Fixture;
use proqi::{
    application::Effect,
    domain::{ContentAnnotation, ContentAnnotationKind},
    ports::editor::CursorMovement,
    ui::{PastePayload, UiInput, UiKey},
};

#[test]
fn delayed_editor_cut_cannot_delete_an_identical_annotated_neighbor() {
    let content = "/tmp/repeated.png";
    let annotation = ContentAnnotation {
        start: 0,
        end: content.len(),
        kind: ContentAnnotationKind::Attachment {
            image: true,
            display_name: "repeated.png".to_owned(),
        },
    };
    let mut fixture = Fixture::with_annotated_thought(content, vec![annotation.clone()]);
    fixture.input(UiInput::PasteAnnotated(
        PastePayload::annotated(content.to_owned(), vec![annotation.clone()])
            .expect("second annotated thought"),
    ));
    fixture.input(crate::key_input(UiKey::Escape));
    fixture.input(crate::key_input(UiKey::Move {
        movement: CursorMovement::VisualUp,
        extend_selection: false,
    }));
    fixture.input(crate::key_input(UiKey::Enter));
    fixture.input(crate::key_input(UiKey::SelectAll));
    let cut = fixture.effects(crate::key_input(UiKey::Cut));
    let [Effect::WriteClipboard { request_id, .. }] = cut.as_slice() else {
        panic!("expected selection write");
    };

    fixture.input(crate::key_input(UiKey::Escape));
    fixture.input(crate::key_input(UiKey::Move {
        movement: CursorMovement::VisualDown,
        extend_selection: false,
    }));
    fixture.input(crate::key_input(UiKey::Enter));
    fixture.input(crate::key_input(UiKey::SelectAll));
    let completion =
        fixture
            .app
            .complete_clipboard_write(*request_id, Ok(()), &mut fixture.ids, &fixture.clock);

    assert!(completion.is_empty());
    assert_eq!(fixture.app.state.board.live_thoughts().len(), 2);
    for thought in fixture.app.state.board.live_thoughts() {
        assert_eq!(thought.content, content);
        assert_eq!(thought.annotations, vec![annotation.clone()]);
    }
    assert_eq!(
        fixture.app.status_text(),
        Some("selection changed before clipboard confirmation")
    );
}
