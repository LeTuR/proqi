//! Modal-aware normalization of editor and board navigation intentions.

use crate::application::InteractionMode;

use super::super::{BoardApp, UiInput, UiKey};

impl BoardApp {
    pub(in crate::ui::app) fn resolve_edit_navigation(
        &self,
        input: UiInput,
        owner: super::super::input_dispatch::ActiveInputOwner,
    ) -> UiInput {
        if let UiInput::Key(UiKey::FastNavigation {
            direction,
            extend_selection,
        }) = input
        {
            if owner.owns_modal_surface() {
                return input;
            }
            let movement = if matches!(
                self.interaction_mode(),
                InteractionMode::Edit { .. } | InteractionMode::Compose
            ) {
                direction.editor_movement()
            } else {
                direction.board_movement()
            };
            return UiInput::Key(UiKey::Move {
                movement,
                extend_selection,
            });
        }
        input
    }
}
