//! Exact platform defaults that intentionally differ from the shared modifier ladder.

use crate::ui::{LogicalKey, LogicalModifiers};

use super::super::{Action, Context};
use crate::ui::shortcut_registry::model::ShortcutBindingPresentation;

const OPTION_SHIFT: LogicalModifiers = LogicalModifiers::ALT.union(LogicalModifiers::SHIFT);

const MACOS_BOARD_DEFAULTS: &[(LogicalKey, Action, ShortcutBindingPresentation)] = &[
    (
        LogicalKey::Up,
        Action::MoveUp,
        ShortcutBindingPresentation::Explicit,
    ),
    (
        LogicalKey::Character('k'),
        Action::MoveUp,
        ShortcutBindingPresentation::DispatchOnly,
    ),
    (
        LogicalKey::Character('K'),
        Action::MoveUp,
        ShortcutBindingPresentation::DispatchOnly,
    ),
    (
        LogicalKey::Down,
        Action::MoveDown,
        ShortcutBindingPresentation::Explicit,
    ),
    (
        LogicalKey::Character('j'),
        Action::MoveDown,
        ShortcutBindingPresentation::DispatchOnly,
    ),
    (
        LogicalKey::Character('J'),
        Action::MoveDown,
        ShortcutBindingPresentation::DispatchOnly,
    ),
];

pub(super) fn binding(
    context: Context,
    key: LogicalKey,
    modifiers: LogicalModifiers,
    macos: bool,
) -> Option<(Action, ShortcutBindingPresentation)> {
    (matches!(context, Context::Board | Context::InsertionBoundary)
        && macos_reorder_modifiers(macos, modifiers))
    .then(|| {
        MACOS_BOARD_DEFAULTS
            .iter()
            .find_map(|(candidate, action, presentation)| {
                (*candidate == key).then_some((*action, *presentation))
            })
    })
    .flatten()
}

pub(super) fn macos_reorder_modifiers(macos: bool, modifiers: LogicalModifiers) -> bool {
    macos && modifiers == OPTION_SHIFT
}
