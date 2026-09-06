//! Shared board-fixture construction helpers.

use super::{Fixture, UiKey, key_input};

pub(super) fn durable_thought(fixture: &mut Fixture, content: &str) {
    fixture.paste(content);
    fixture.input(key_input(UiKey::Escape));
}
