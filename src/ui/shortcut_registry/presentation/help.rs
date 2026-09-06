//! Contextual Help projected from ordered descriptor metadata and resolved aliases.

use crate::ui::shortcut_registry::{HelpAvailability, HelpSurface};
use crate::ui::{BoardApp, ShortcutActionId as Action, ShortcutContext as Context};

pub(crate) type HelpItem = (String, &'static str);

pub(crate) fn help_items(app: &BoardApp) -> Vec<HelpItem> {
    let context = app.footer_shortcut_context();
    let surface = match context {
        Context::Recovery => HelpSurface::Recovery,
        Context::Compose | Context::Edit => HelpSurface::Editor,
        _ => HelpSurface::Board,
    };
    let registry = app.shortcut_registry();
    registry
        .help(surface)
        .into_iter()
        .filter(|(_, metadata)| {
            metadata.availability != HelpAvailability::Submission || app.supports_submission()
        })
        .filter_map(|(action, metadata)| {
            let label = registry.help_label(context, &related_actions(action));
            (!label.is_empty()).then_some((label, metadata.label))
        })
        .collect()
}

fn related_actions(action: Action) -> Vec<Action> {
    match action {
        Action::FocusNext => vec![Action::FocusNext, Action::FocusPrevious],
        Action::ExtendNext => vec![Action::ExtendNext, Action::ExtendPrevious],
        Action::MoveDown => vec![Action::MoveDown, Action::MoveUp],
        Action::FastNext => vec![Action::FastPrevious, Action::FastNext],
        Action::MoveDocumentStart => vec![Action::MoveDocumentStart, Action::MoveDocumentEnd],
        Action::ExtendVisualRowStart => {
            vec![Action::ExtendVisualRowStart, Action::ExtendVisualRowEnd]
        }
        Action::MoveVisualDown => vec![Action::MoveVisualUp, Action::MoveVisualDown],
        _ => vec![action],
    }
}
