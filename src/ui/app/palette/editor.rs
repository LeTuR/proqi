//! Edit-owned command-palette execution.

use crate::{
    application::{Effect, InteractionMode},
    ports::{
        editor::CursorMovement,
        environment::{Clock, IdGenerator},
    },
};

use super::EditorSelectionHandoff;
use crate::ui::app::{BoardApp, UiKey};
use crate::ui::shortcut_registry::PaletteEditorCommand as EditorCommand;

impl BoardApp {
    pub(super) fn execute_editor_command(
        &mut self,
        command: EditorCommand,
        selection_handoff: Option<EditorSelectionHandoff>,
        ids: &mut impl IdGenerator,
        clock: &impl Clock,
    ) -> Vec<Effect> {
        let mut effects = if matches!(self.state.mode, InteractionMode::Edit { .. }) {
            Vec::new()
        } else {
            self.expand_and_enter_edit(ids, clock)
        };
        self.restore_palette_selection_handoff(selection_handoff);
        if command == EditorCommand::PlainNewline {
            effects.extend(self.insert_newline(false, ids, clock));
            return effects;
        }
        if matches!(
            command,
            EditorCommand::DeleteLogicalLine | EditorCommand::DeleteSentence
        ) {
            let key = if command == EditorCommand::DeleteLogicalLine {
                UiKey::DeleteLogicalLine
            } else {
                UiKey::DeleteSentence
            };
            effects.extend(self.handle_edit_key(key, ids, clock));
            return effects;
        }
        if let Some(edge) = visual_row_edge(command) {
            effects.extend(self.handle_edit_key(UiKey::ExtendVisualRow { edge }, ids, clock));
            return effects;
        }
        if let Some(movement) = movement(command) {
            effects.extend(self.handle_edit_key(
                UiKey::Move {
                    movement,
                    extend_selection: false,
                },
                ids,
                clock,
            ));
            return effects;
        }
        effects.extend(self.apply_indentation(command == EditorCommand::Outdent, ids, clock));
        effects
    }
}

const fn visual_row_edge(command: EditorCommand) -> Option<crate::ui::VisualRowEdge> {
    match command {
        EditorCommand::SelectVisualRowStart => Some(crate::ui::VisualRowEdge::Start),
        EditorCommand::SelectVisualRowEnd => Some(crate::ui::VisualRowEdge::End),
        _ => None,
    }
}

const fn movement(command: EditorCommand) -> Option<CursorMovement> {
    match command {
        EditorCommand::JumpUp => Some(CursorMovement::VisualJumpUp),
        EditorCommand::JumpDown => Some(CursorMovement::VisualJumpDown),
        EditorCommand::ThoughtStart => Some(CursorMovement::DocumentStart),
        EditorCommand::ThoughtEnd => Some(CursorMovement::DocumentEnd),
        _ => None,
    }
}
