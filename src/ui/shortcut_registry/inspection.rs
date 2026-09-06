//! Content-free inspection of the same resolution used by the active UI owner.

use serde_json::{Value, json};

use super::{ShortcutContextStack, ShortcutPlatform, ShortcutRegistry};
use crate::ui::{KeyPhase, KeyStroke, LogicalKey, LogicalKeyState, LogicalModifiers};

impl ShortcutRegistry {
    pub(crate) fn inspect(&self, contexts: &ShortcutContextStack, stroke: KeyStroke) -> Value {
        let resolved = self.dispatch(contexts, stroke);
        let action = resolved.and_then(|value| value.action);
        let classification = if stroke.phase == KeyPhase::Release {
            "release_ignored"
        } else if action.is_some_and(|action| known_no_op(contexts.active(), action)) {
            "no_op"
        } else if action.is_some() {
            "resolved"
        } else if matches!(stroke.key, LogicalKey::Character(character) if !character.is_control())
            && super::validation::reserves_printable(stroke.modifiers)
        {
            if contexts
                .active()
                .is_some_and(super::inventory::bindings::vocabulary::is_text_context)
            {
                "reserved_literal"
            } else {
                "unbound"
            }
        } else {
            "unbound"
        };
        json!({
            "capture_cancelled": stroke.key == LogicalKey::Escape && stroke.phase != KeyPhase::Release,
            "keystroke": stroke_value(stroke),
            "platform": if self.platform() == ShortcutPlatform::MacOs { "macos" } else { "portable" },
            "primary": if self.platform() == ShortcutPlatform::MacOs { vec!["Super", "Meta"] } else { vec!["Control"] },
            "context_stack": contexts.as_slice().iter().map(|context| context.configuration_id()).collect::<Vec<_>>(),
            "active_context": contexts.active().map(super::ShortcutContext::configuration_id),
            "classification": classification,
            "action": action.map(super::ShortcutActionId::diagnostics_id),
            "ui_intention": action.zip(resolved).map(|(_, value)| format!("{:?}", value.intention)),
            "binding_identity": action.and_then(|action| contexts.active().map(|context| format!("{}:{}:{}", context.configuration_id(), action.diagnostics_id(), binding_identity(stroke)))),
        })
    }
}

fn stroke_value(stroke: KeyStroke) -> Value {
    let key = match stroke.key {
        LogicalKey::Character(character) => format!("U+{:04X}", u32::from(character)),
        key => super::contract::key_name(key),
    };
    let modifiers = [
        (LogicalModifiers::CONTROL, "Control"),
        (LogicalModifiers::ALT, "Alt"),
        (LogicalModifiers::SHIFT, "Shift"),
        (LogicalModifiers::SUPER, "Super"),
        (LogicalModifiers::META, "Meta"),
        (LogicalModifiers::HYPER, "Hyper"),
    ]
    .into_iter()
    .filter_map(|(flag, name)| stroke.modifiers.contains(flag).then_some(name))
    .collect::<Vec<_>>();
    let state = [
        (LogicalKeyState::KEYPAD, "Keypad"),
        (LogicalKeyState::CAPS_LOCK, "CapsLock"),
        (LogicalKeyState::NUM_LOCK, "NumLock"),
    ]
    .into_iter()
    .filter_map(|(flag, name)| stroke.state.contains(flag).then_some(name))
    .collect::<Vec<_>>();
    json!({ "key": key, "modifiers": modifiers, "state": state,
        "phase": match stroke.phase { KeyPhase::Press => "press", KeyPhase::Repeat => "repeat", KeyPhase::Release => "release" } })
}

fn binding_identity(stroke: KeyStroke) -> String {
    let value = stroke_value(stroke);
    let key = value["key"].as_str().unwrap_or("unknown");
    let modifiers = value["modifiers"]
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join("+")
        })
        .unwrap_or_default();
    format!("{key}:{modifiers}")
}

fn known_no_op(context: Option<super::ShortcutContext>, action: super::ShortcutActionId) -> bool {
    use super::{ShortcutActionId as Action, ShortcutContext as Context};
    match action {
        Action::Close => context == Some(Context::Recovery),
        Action::FastPrevious
        | Action::FastNext
        | Action::FastExtendPrevious
        | Action::FastExtendNext => matches!(
            context,
            Some(Context::Rename | Context::BrowserRename | Context::Recovery | Context::Direction)
        ),
        _ => false,
    }
}
