//! Diagnostic resolution never duplicates keyboard policy or includes typed text.
use super::super::{ShortcutPlatform, ShortcutRegistry};
use super::*;
use crate::ui::KeyBindings;
use crate::ui::{KeyPhase, LogicalKeyState, ShortcutContext, ShortcutContextStack};

#[test]
fn diagnostic_matches_actual_contextual_dispatch_and_preserves_all_event_fields() {
    let registry =
        ShortcutRegistry::resolve(&KeyBindings::default(), ShortcutPlatform::MacOs).unwrap();
    let contexts = ShortcutContextStack::new([ShortcutContext::Board, ShortcutContext::Edit]);
    for (modifiers, action) in [
        (LogicalModifiers::SUPER, Some("selection.select_all")),
        (LogicalModifiers::META, Some("selection.select_all")),
        (LogicalModifiers::CONTROL, None),
    ] {
        let mut stroke = KeyStroke::press(LogicalKey::Character('a')).with_modifiers(modifiers);
        stroke.phase = KeyPhase::Repeat;
        stroke.state = LogicalKeyState::KEYPAD
            .union(LogicalKeyState::CAPS_LOCK)
            .union(LogicalKeyState::NUM_LOCK);
        let event = registry.inspect(&contexts, stroke);
        assert_eq!(event["action"].as_str(), action);
        assert_eq!(event["keystroke"]["key"], "U+0061");
        assert_eq!(event["keystroke"]["phase"], "repeat");
        assert_eq!(
            event["keystroke"]["state"],
            serde_json::json!(["Keypad", "CapsLock", "NumLock"])
        );
        assert_eq!(event["active_context"], "edit");
        stroke.phase = KeyPhase::Release;
        let release = registry.inspect(&contexts, stroke);
        assert_eq!(release["classification"], "release_ignored");
        assert!(release["action"].is_null());
    }
}

#[test]
fn diagnostic_distinguishes_reserved_text_and_unbound_chords_in_the_top_owner() {
    let registry = ShortcutRegistry::default();
    for modifiers in [
        LogicalModifiers::NONE,
        LogicalModifiers::SHIFT,
        LogicalModifiers::ALT,
        LogicalModifiers::CONTROL.union(LogicalModifiers::ALT),
    ] {
        let event = registry.inspect(
            &ShortcutContextStack::new([ShortcutContext::Board, ShortcutContext::Search]),
            KeyStroke::press(LogicalKey::Character('ä')).with_modifiers(modifiers),
        );
        assert_eq!(event["classification"], "reserved_literal");
        assert!(!event.to_string().contains('ä'));
        assert!(event["ui_intention"].is_null());
    }
    let event = registry.inspect(
        &ShortcutContextStack::new([ShortcutContext::Help]),
        KeyStroke::press(LogicalKey::Function(35)),
    );
    assert_eq!(event["classification"], "unbound");
}
