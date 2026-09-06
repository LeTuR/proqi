//! Four-way non-text chooser parity across irrelevant modifiers.

use super::*;
use proqi::domain::Direction;

#[test]
fn direction_chooser_accepts_modified_arrows_and_vim_spellings_equally() {
    let cases = [
        (crate::key_input(UiKey::Character('H')), Direction::Left),
        (
            crate::key_input(UiKey::PrimaryCharacter('h')),
            Direction::Left,
        ),
        (
            crate::key_input(UiKey::Move {
                movement: CursorMovement::WordBack,
                extend_selection: true,
            }),
            Direction::Left,
        ),
        (crate::key_input(UiKey::Character('J')), Direction::Down),
        (
            crate::key_input(UiKey::PrimaryCharacter('j')),
            Direction::Down,
        ),
        (
            crate::key_input(UiKey::PrimaryShiftMove {
                movement: CursorMovement::DocumentEnd,
            }),
            Direction::Down,
        ),
        (crate::key_input(UiKey::Character('K')), Direction::Up),
        (
            crate::key_input(UiKey::PrimaryCharacter('k')),
            Direction::Up,
        ),
        (
            UiInput::KeyStroke(
                KeyStroke::press(LogicalKey::Up).with_modifiers(LogicalModifiers::ALT),
            ),
            Direction::Up,
        ),
        (crate::key_input(UiKey::Character('L')), Direction::Right),
        (
            crate::key_input(UiKey::PrimaryCharacter('l')),
            Direction::Right,
        ),
        (
            crate::key_input(UiKey::Move {
                movement: CursorMovement::WordForward,
                extend_selection: true,
            }),
            Direction::Right,
        ),
    ];

    for (key, expected) in cases {
        let mut fixture = Fixture::new();
        super::agent::prepare_thought(&mut fixture);
        fixture.app.complete_agent_discovery(Ok(vec![
            super::agent::target(Direction::Left, "w1:p2"),
            super::agent::target(Direction::Down, "w1:p3"),
            super::agent::target(Direction::Up, "w1:p4"),
            super::agent::target(Direction::Right, "w1:p5"),
        ]));
        fixture.input(crate::key_input(UiKey::Character('S')));
        let effects = fixture.effects(key.clone());
        let request = super::agent::start_submission(&mut fixture, &effects);
        assert_eq!(
            request.target.adjacent_direction(),
            Some(expected),
            "key: {key:?}"
        );
    }
}
