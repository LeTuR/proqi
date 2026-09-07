//! Diagnostic resolution retains typed policy until the CLI projection boundary.

use super::super::{
    ShortcutPlatform, ShortcutRegistry,
    inspection::{ShortcutClassification, intention},
};
use super::*;
use crate::ui::{
    KeyBindings, KeyPhase, LogicalKeyState, ShortcutActionId, ShortcutContext,
    ShortcutContextStack, ShortcutModifiers,
};

#[test]
fn diagnostic_matches_actual_contextual_dispatch_and_preserves_all_event_fields() {
    let registry =
        ShortcutRegistry::resolve(&KeyBindings::default(), ShortcutPlatform::MacOs).unwrap();
    let contexts = ShortcutContextStack::new([ShortcutContext::Board, ShortcutContext::Edit]);
    for (modifiers, action) in [
        (LogicalModifiers::SUPER, Some(ShortcutActionId::SelectAll)),
        (LogicalModifiers::META, Some(ShortcutActionId::SelectAll)),
        (LogicalModifiers::CONTROL, None),
    ] {
        let mut stroke = KeyStroke::press(LogicalKey::Character('a')).with_modifiers(modifiers);
        stroke.phase = KeyPhase::Repeat;
        stroke.state = LogicalKeyState::KEYPAD
            .union(LogicalKeyState::CAPS_LOCK)
            .union(LogicalKeyState::NUM_LOCK);
        let event = registry.inspect(&contexts, stroke);
        assert_eq!(event.action, action);
        assert_eq!(event.stroke, stroke);
        assert_eq!(event.active_context, Some(ShortcutContext::Edit));
        assert_eq!(event.context_stack, contexts.as_slice());
        assert_eq!(
            event.classification,
            if action.is_some() {
                ShortcutClassification::Resolved
            } else {
                ShortcutClassification::Unbound
            }
        );
        stroke.phase = KeyPhase::Release;
        let release = registry.inspect(&contexts, stroke);
        assert_eq!(
            release.classification,
            ShortcutClassification::ReleaseIgnored
        );
        assert!(release.action.is_none());
        assert!(release.intention.is_none());
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
        assert_eq!(
            event.classification,
            ShortcutClassification::ReservedLiteral
        );
        assert!(event.action.is_none());
        assert!(event.intention.is_none());
    }
    let event = registry.inspect(
        &ShortcutContextStack::new([ShortcutContext::Help]),
        KeyStroke::press(LogicalKey::Function(35)),
    );
    assert_eq!(event.classification, ShortcutClassification::Unbound);
}

#[test]
fn explicit_intention_spellings_preserve_every_resolved_debug_contract_value() {
    for platform in [ShortcutPlatform::MacOs, ShortcutPlatform::Portable] {
        let registry = ShortcutRegistry::resolve(&KeyBindings::default(), platform).unwrap();
        for descriptor in registry.descriptors() {
            let claims = match platform {
                ShortcutPlatform::MacOs => descriptor
                    .macos_defaults
                    .iter()
                    .chain(&descriptor.macos_aliases),
                ShortcutPlatform::Portable => descriptor
                    .portable_defaults
                    .iter()
                    .chain(&descriptor.portable_aliases),
            };
            claims.for_each(|claim| assert_intention_spelling(&registry, claim));
        }
    }
}

fn assert_intention_spelling(registry: &ShortcutRegistry, claim: &crate::ui::ShortcutBindingClaim) {
    let ShortcutModifiers::Exact(modifiers) = claim.binding.modifiers else {
        panic!("registry must be fully resolved");
    };
    for context in &claim.contexts {
        let stroke = KeyStroke::press(claim.binding.key).with_modifiers(modifiers);
        let resolved = registry
            .dispatch(&ShortcutContextStack::new([*context]), stroke)
            .expect("claimed binding");
        if resolved.action.is_some() {
            assert_eq!(
                intention::name(resolved.intention),
                format!("{:?}", resolved.intention)
            );
        }
    }
}
