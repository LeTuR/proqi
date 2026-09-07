//! Board compatibility intentions and modifier ladder.

use crate::ui::input::UiKey;

use super::ShortcutRegistry;
use crate::ui::shortcut_registry::{
    context_policy::board_navigation_action, model::ShortcutActionId as Action,
};

impl ShortcutRegistry {
    fn board_character_action(&self, character: char) -> Option<Action> {
        self.effective_bindings
            .get(&(
                super::ShortcutContext::Board,
                crate::ui::LogicalKey::Character(character),
                crate::ui::LogicalModifiers::NONE,
            ))
            .copied()
    }

    pub(crate) fn board_action_for_intention(&self, key: UiKey) -> Option<Action> {
        match key {
            UiKey::Shortcut(action) => Some(action),
            UiKey::Delete => Some(Action::Delete),
            UiKey::Submit => Some(Action::SubmitRemove),
            UiKey::SubmitKeep => Some(Action::SubmitKeep),
            UiKey::UnmodifiedSpace => self.board_character_action(' '),
            UiKey::Character(character) => self.board_character_action(character),
            UiKey::Move {
                movement,
                extend_selection,
            } => board_navigation_action(movement, extend_selection, false),
            UiKey::PrimaryShiftMove { movement } => board_navigation_action(movement, false, true),
            UiKey::Enter => Some(Action::Edit),
            UiKey::SelectAll => Some(Action::SelectAll),
            UiKey::Undo => Some(Action::Undo),
            UiKey::Redo => Some(Action::Redo),
            UiKey::Copy => Some(Action::Copy),
            UiKey::Cut => Some(Action::Cut),
            UiKey::PasteClipboard => Some(Action::PasteExact),
            UiKey::PasteClipboardReflow => Some(Action::PasteReflow),
            UiKey::Duplicate => Some(Action::Duplicate),
            UiKey::Quit => Some(Action::Quit),
            _ => None,
        }
    }
}
