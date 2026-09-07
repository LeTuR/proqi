//! Compatibility edges shared across contexts and modifier families.

use crate::ui::{
    KeyBindings, LogicalKey, LogicalModifiers, ShortcutActionId as Action,
    ShortcutContext as Context, ShortcutContextStack,
};

use super::super::{ShortcutPlatform, ShortcutRegistry};
use super::{dispatch::dispatched, stroke};

#[test]
fn modified_escape_preserves_the_established_close_route() {
    let registry = ShortcutRegistry::resolve(&KeyBindings::default(), ShortcutPlatform::MacOs)
        .expect("valid registry");
    for context in super::super::inventory::ESCAPE_CONTEXTS {
        for modifiers in [
            LogicalModifiers::SHIFT,
            LogicalModifiers::ALT,
            LogicalModifiers::CONTROL,
            LogicalModifiers::SUPER.union(LogicalModifiers::META),
            LogicalModifiers::HYPER,
        ] {
            assert_eq!(
                dispatched(&registry, *context, LogicalKey::Escape, modifiers).action,
                Some(Action::Close),
                "context {context:?}, modifiers {modifiers:?}",
            );
        }
    }
}

#[test]
fn page_keys_keep_their_global_fast_navigation_identity() {
    let registry = ShortcutRegistry::resolve(&KeyBindings::default(), ShortcutPlatform::MacOs)
        .expect("valid registry");
    for context in super::super::inventory::bindings::vocabulary::KEYBOARD_CONTEXTS {
        for (key, expected) in [
            (LogicalKey::PageUp, Action::FastPrevious),
            (LogicalKey::PageDown, Action::FastNext),
        ] {
            assert_eq!(
                dispatched(&registry, *context, key, LogicalModifiers::NONE).action,
                Some(expected),
                "context {context:?}, key {key:?}",
            );
        }
        for (key, expected) in [
            (LogicalKey::PageUp, Action::FastExtendPrevious),
            (LogicalKey::PageDown, Action::FastExtendNext),
        ] {
            assert_eq!(
                dispatched(&registry, *context, key, LogicalModifiers::SHIFT).action,
                Some(expected),
                "shifted context {context:?}, key {key:?}",
            );
        }
    }
}

#[test]
fn shifted_lowercase_board_reports_use_the_configured_uppercase_alias() {
    for platform in [ShortcutPlatform::MacOs, ShortcutPlatform::Portable] {
        let registry =
            ShortcutRegistry::resolve(&KeyBindings::default(), platform).expect("valid registry");
        for context in [Context::Board, Context::InsertionBoundary] {
            for (character, expected) in [
                ('d', Action::Duplicate),
                ('p', Action::PasteReflow),
                ('s', Action::SubmitKeep),
            ] {
                assert_eq!(
                    dispatched(
                        &registry,
                        context,
                        LogicalKey::Character(character),
                        LogicalModifiers::SHIFT,
                    )
                    .action,
                    Some(expected),
                    "{platform:?}, {context:?}, {character:?}",
                );
            }
        }
    }
}

#[test]
fn terminal_safe_duplicate_does_not_override_a_legacy_paste_pair() {
    let keys = KeyBindings {
        delete: 'g',
        paste: 'd',
        ..KeyBindings::default()
    };
    let registry =
        ShortcutRegistry::resolve(&keys, ShortcutPlatform::Portable).expect("valid registry");
    for (character, modifiers, expected) in [
        ('d', LogicalModifiers::NONE, Action::PasteExact),
        ('D', LogicalModifiers::NONE, Action::PasteReflow),
        ('d', LogicalModifiers::SHIFT, Action::PasteReflow),
    ] {
        assert_eq!(
            dispatched(
                &registry,
                Context::Board,
                LogicalKey::Character(character),
                modifiers,
            )
            .action,
            Some(expected),
        );
    }
}

#[test]
fn hyper_preserves_named_key_behavior_without_becoming_primary() {
    let registry = ShortcutRegistry::resolve(&KeyBindings::default(), ShortcutPlatform::MacOs)
        .expect("valid registry");
    for (context, key, expected) in [
        (Context::Edit, LogicalKey::Backspace, Action::Backspace),
        (Context::Edit, LogicalKey::Delete, Action::DeleteForward),
        (Context::Board, LogicalKey::Up, Action::FocusPrevious),
        (Context::Board, LogicalKey::PageUp, Action::FastPrevious),
    ] {
        assert_eq!(
            dispatched(&registry, context, key, LogicalModifiers::HYPER).action,
            Some(expected),
            "context {context:?}, key {key:?}",
        );
    }
    assert_eq!(
        registry.dispatch(
            &ShortcutContextStack::new([Context::Edit]),
            stroke(LogicalKey::Character('v'), LogicalModifiers::HYPER),
        ),
        None,
    );
}
