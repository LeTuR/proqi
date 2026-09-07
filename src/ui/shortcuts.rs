//! Responsive measurements for registry-projected contextual Help.

use super::{BoardApp, shortcut_registry::presentation};

pub(crate) type Shortcut = presentation::HelpItem;

pub(crate) fn items(app: &BoardApp) -> Vec<Shortcut> {
    presentation::help_items(app)
}

pub(crate) fn grid_metrics(items: &[Shortcut], width: u16) -> (usize, usize) {
    let label_width = widest_label(items);
    let half_width = width / 2;
    let half_key_width = selected_key_width(items, half_width, label_width);
    let half_widest = widest_with_key_width(items, half_key_width);
    if usize::from(half_width) >= half_widest {
        return (2, half_key_width);
    }
    (1, selected_key_width(items, width, label_width))
}

fn selected_key_width(items: &[Shortcut], width: u16, label_width: usize) -> usize {
    items
        .iter()
        .map(|item| {
            crate::ports::text_layout::terminal_cell_width(key(
                item,
                usize::from(width),
                label_width,
            ))
        })
        .max()
        .unwrap_or(1)
}

fn widest_with_key_width(items: &[Shortcut], key_width: usize) -> usize {
    items
        .iter()
        .map(|item| key_width + 1 + crate::ports::text_layout::terminal_cell_width(item.label))
        .max()
        .unwrap_or(1)
}

fn widest_label(items: &[Shortcut]) -> usize {
    items
        .iter()
        .map(|item| crate::ports::text_layout::terminal_cell_width(item.label))
        .max()
        .unwrap_or(1)
}

pub(crate) fn key(item: &Shortcut, width: usize, label_width: usize) -> &str {
    let available = width.saturating_sub(label_width.saturating_add(1));
    if crate::ports::text_layout::terminal_cell_width(&item.full_key) > available {
        &item.compact_key
    } else {
        &item.full_key
    }
}

pub(crate) fn row_count(app: &BoardApp, width: u16) -> usize {
    let items = items(app);
    let (columns, _) = grid_metrics(&items, width);
    items.len().div_ceil(columns)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(full_key: &str, compact_key: &str, label: &'static str) -> Shortcut {
        Shortcut {
            full_key: full_key.to_owned(),
            compact_key: compact_key.to_owned(),
            label,
        }
    }

    #[test]
    fn narrow_help_compacts_only_the_alias_group_that_would_overflow() {
        let ordinary = item("Cmd+Shift+Enter/Shift+S", "Shift+S", "Submit & keep");
        let reorder = item(
            "Cmd+Shift+↑/↓/Option+Shift+↑/↓",
            "Option+Shift+↑/↓",
            "Reorder",
        );
        let items = [ordinary.clone(), reorder.clone()];
        let label_width = widest_label(&items);

        assert_eq!(key(&ordinary, 42, label_width), ordinary.full_key);
        assert_eq!(key(&reorder, 42, label_width), reorder.compact_key);
    }

    #[test]
    fn wide_help_retains_every_presented_alias() {
        let reorder = item(
            "Cmd+Shift+↑/↓/Option+Shift+↑/↓",
            "Option+Shift+↑/↓",
            "Reorder",
        );
        assert_eq!(key(&reorder, 80, 7), reorder.full_key);
    }
}
