//! Explicit stable formatting for UI intentions exposed by capture schema 1.

use crate::{
    ports::editor::CursorMovement,
    ui::{FastNavigation, UiKey, VisualRowEdge},
};

pub(in crate::ui::shortcut_registry) fn name(intention: UiKey) -> String {
    match intention {
        UiKey::Shortcut(action) => format!("Shortcut({})", action.diagnostic_variant()),
        UiKey::Character(character) => format!("Character(U+{:04X})", u32::from(character)),
        UiKey::PrimaryCharacter(character) => {
            format!("PrimaryCharacter(U+{:04X})", u32::from(character))
        }
        UiKey::PrimaryShiftCharacter(character) => {
            format!("PrimaryShiftCharacter(U+{:04X})", u32::from(character))
        }
        UiKey::FastNavigation {
            direction,
            extend_selection,
        } => format!(
            "FastNavigation {{ direction: {}, extend_selection: {extend_selection} }}",
            fast_name(direction)
        ),
        UiKey::Move {
            movement,
            extend_selection,
        } => format!(
            "Move {{ movement: {}, extend_selection: {extend_selection} }}",
            movement_name(movement)
        ),
        UiKey::ExtendVisualRow { edge } => {
            format!("ExtendVisualRow {{ edge: {} }}", edge_name(edge))
        }
        UiKey::MoveVisualRow { edge } => {
            format!("MoveVisualRow {{ edge: {} }}", edge_name(edge))
        }
        UiKey::PrimaryShiftMove { movement } => format!(
            "PrimaryShiftMove {{ movement: {} }}",
            movement_name(movement)
        ),
        other => unit_name(other).to_owned(),
    }
}

const fn unit_name(intention: UiKey) -> &'static str {
    match intention {
        UiKey::Quit => "Quit",
        UiKey::UnmodifiedSpace => "UnmodifiedSpace",
        UiKey::Enter => "Enter",
        UiKey::Submit => "Submit",
        UiKey::SubmitKeep => "SubmitKeep",
        UiKey::Tab => "Tab",
        UiKey::BackTab => "BackTab",
        UiKey::PickerPrevious => "PickerPrevious",
        UiKey::PickerNext => "PickerNext",
        UiKey::Escape => "Escape",
        UiKey::Backspace => "Backspace",
        UiKey::Delete => "Delete",
        UiKey::ModifiedDelete => "ModifiedDelete",
        UiKey::SelectAll => "SelectAll",
        UiKey::DeleteLogicalLine => "DeleteLogicalLine",
        UiKey::DeleteSentence => "DeleteSentence",
        UiKey::Undo => "Undo",
        UiKey::Redo => "Redo",
        UiKey::Copy => "Copy",
        UiKey::Cut => "Cut",
        UiKey::PasteClipboard => "PasteClipboard",
        UiKey::PasteClipboardReflow => "PasteClipboardReflow",
        UiKey::Duplicate => "Duplicate",
        UiKey::Shortcut(_)
        | UiKey::Character(_)
        | UiKey::PrimaryCharacter(_)
        | UiKey::PrimaryShiftCharacter(_)
        | UiKey::FastNavigation { .. }
        | UiKey::Move { .. }
        | UiKey::ExtendVisualRow { .. }
        | UiKey::MoveVisualRow { .. }
        | UiKey::PrimaryShiftMove { .. } => "Unknown",
    }
}

const fn movement_name(movement: CursorMovement) -> &'static str {
    match movement {
        CursorMovement::GraphemeBack => "GraphemeBack",
        CursorMovement::GraphemeForward => "GraphemeForward",
        CursorMovement::WordBack => "WordBack",
        CursorMovement::WordForward => "WordForward",
        CursorMovement::VisualUp => "VisualUp",
        CursorMovement::VisualDown => "VisualDown",
        CursorMovement::VisualJumpUp => "VisualJumpUp",
        CursorMovement::VisualJumpDown => "VisualJumpDown",
        CursorMovement::LineStart => "LineStart",
        CursorMovement::LineEnd => "LineEnd",
        CursorMovement::DocumentStart => "DocumentStart",
        CursorMovement::DocumentEnd => "DocumentEnd",
    }
}

const fn fast_name(direction: FastNavigation) -> &'static str {
    match direction {
        FastNavigation::Previous => "Previous",
        FastNavigation::Next => "Next",
    }
}

const fn edge_name(edge: VisualRowEdge) -> &'static str {
    match edge {
        VisualRowEdge::Start => "Start",
        VisualRowEdge::End => "End",
    }
}
