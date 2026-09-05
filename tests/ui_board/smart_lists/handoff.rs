//! Commands selection-handoff lifecycle for smart-list actions.

use super::{Fixture, UiKey, revision};

#[test]
fn cancelling_the_palette_discards_the_selection_handoff() {
    let mut fixture = Fixture::new();
    fixture.paste("- parent\n- child\n- untouched");
    fixture.input(crate::key_input(UiKey::SelectAll));
    fixture.input(crate::key_input(UiKey::Escape));
    fixture.input(crate::key_input(UiKey::Character(':')));
    fixture.input(crate::key_input(UiKey::Escape));
    fixture.input(crate::key_input(UiKey::Character(':')));
    for character in "indent line".chars() {
        fixture.input(crate::key_input(UiKey::Character(character)));
    }

    let effects = fixture.effects(crate::key_input(UiKey::Enter));
    assert_eq!(
        revision(&effects).after_content,
        "- parent\n- child\n  - untouched"
    );
}

#[test]
fn board_navigation_discards_the_selection_handoff() {
    let mut fixture = Fixture::new();
    fixture.paste("- first");
    fixture.input(crate::key_input(UiKey::Escape));
    fixture.paste("- second");
    fixture.input(crate::key_input(UiKey::SelectAll));
    fixture.input(crate::key_input(UiKey::Escape));
    fixture.input(crate::key_input(UiKey::Character('k')));
    fixture.input(crate::key_input(UiKey::Character(':')));
    for character in "indent line".chars() {
        fixture.input(crate::key_input(UiKey::Character(character)));
    }

    let effects = fixture.effects(crate::key_input(UiKey::Enter));
    assert_eq!(revision(&effects).after_content, "  - first");
}
