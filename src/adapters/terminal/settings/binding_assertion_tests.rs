//! Configuration tests assert effective dispatch rather than discarded legacy fields.

use super::super::LoadedSettings;
use crate::ui::{
    KeyStroke, LogicalKey, LogicalModifiers, ShortcutActionId, ShortcutContext,
    ShortcutContextStack,
};

pub(super) fn assert_binding(
    settings: &LoadedSettings,
    action: ShortcutActionId,
    character: char,
    editor: bool,
) {
    let (context, modifiers) = if editor {
        (
            ShortcutContext::Edit,
            if cfg!(target_os = "macos") {
                LogicalModifiers::SUPER
            } else {
                LogicalModifiers::CONTROL
            }
            .union(LogicalModifiers::SHIFT),
        )
    } else {
        (ShortcutContext::Board, LogicalModifiers::NONE)
    };
    assert_eq!(
        settings
            .ui
            .shortcuts
            .dispatch(
                &ShortcutContextStack::new([context]),
                KeyStroke::press(LogicalKey::Character(character)).with_modifiers(modifiers)
            )
            .and_then(|resolved| resolved.action),
        Some(action)
    );
}
