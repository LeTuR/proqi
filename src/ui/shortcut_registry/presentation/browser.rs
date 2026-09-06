//! Session Browser footer projected from effective action bindings.

use crate::ui::ShortcutActionId as Action;

use super::super::{ShortcutContext, ShortcutRegistry};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BrowserFooterProjection {
    pub(crate) actions: &'static [Action],
    pub(crate) key: String,
    pub(crate) label: &'static str,
}

const RENAME: &[Action] = &[Action::RenameSession];
const TRASH: &[Action] = &[Action::BrowserTrash];
const SELECT: &[Action] = &[Action::FocusPrevious, Action::FocusNext];
const OPEN: &[Action] = &[Action::Confirm];
const CANCEL: &[Action] = &[Action::Close];

pub(crate) fn browser_footer_projection(
    registry: &ShortcutRegistry,
    width: u16,
    context: ShortcutContext,
) -> Vec<BrowserFooterProjection> {
    let items: &[(&[Action], &str)] = if context == ShortcutContext::BrowserRename {
        &[(OPEN, "Save"), (CANCEL, "Cancel")]
    } else if width >= 60 {
        &[
            (RENAME, "Rename"),
            (TRASH, "Trash"),
            (SELECT, "Select"),
            (OPEN, "Open"),
            (CANCEL, "Cancel"),
        ]
    } else if width >= 36 {
        &[
            (RENAME, "Rename"),
            (TRASH, "Trash"),
            (OPEN, "Open"),
            (CANCEL, "Back"),
        ]
    } else {
        &[(RENAME, "Name"), (TRASH, "Trash"), (CANCEL, "Back")]
    };
    items
        .iter()
        .map(|(actions, label)| {
            let key = actions
                .iter()
                .map(|action| registry.action_label(context, *action, true))
                .filter(|label| !label.is_empty())
                .collect::<Vec<_>>()
                .join("/");
            BrowserFooterProjection {
                actions,
                key,
                label,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::KeyBindings;

    #[test]
    fn established_responsive_browser_footer_is_registry_projected() {
        let registry = ShortcutRegistry::from_validated(&KeyBindings::default());
        let wide = browser_footer_projection(&registry, 80, ShortcutContext::Browser);
        assert_eq!(
            wide.iter()
                .map(|item| item.key.as_str())
                .collect::<Vec<_>>(),
            ["F2", "F8", "↑/↓", "Enter", "Esc"]
        );
        assert_eq!(
            wide.iter().map(|item| item.label).collect::<Vec<_>>(),
            ["Rename", "Trash", "Select", "Open", "Cancel"]
        );
        assert_eq!(
            browser_footer_projection(&registry, 40, ShortcutContext::Browser)[3].label,
            "Back"
        );
        assert_eq!(
            browser_footer_projection(&registry, 30, ShortcutContext::Browser)[0].label,
            "Name"
        );
    }
}
