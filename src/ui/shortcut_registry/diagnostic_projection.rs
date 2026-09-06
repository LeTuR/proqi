//! Compatibility projection for the public `diagnostics keypress` contract.

use crate::ui::{KeyStroke, LogicalKey, LogicalModifiers, VisualRowEdge};

use super::dispatch::ShortcutPlatform;

pub(super) fn legacy_keypress_action(
    stroke: KeyStroke,
    platform: ShortcutPlatform,
) -> Option<String> {
    let primary = platform_primary(stroke.modifiers, platform);
    if primary && let Some(action) = primary_action(stroke) {
        return Some(action);
    }
    if has_command_modifier(stroke.modifiers) && primary_action(stroke).is_some() {
        return None;
    }
    let extend = stroke.modifiers.contains(LogicalModifiers::SHIFT);
    if let Some(horizontal) = horizontal_action(stroke, platform, extend) {
        return Some(horizontal);
    }
    match stroke.key {
        LogicalKey::Character(' ') if stroke.modifiers.is_empty() => {
            Some("UnmodifiedSpace".to_owned())
        }
        LogicalKey::Character(character) => Some(format!("Character({character:?})")),
        LogicalKey::Enter => Some("Enter".to_owned()),
        LogicalKey::BackTab => Some("BackTab".to_owned()),
        LogicalKey::Tab if extend => Some("BackTab".to_owned()),
        LogicalKey::Tab => Some("Tab".to_owned()),
        LogicalKey::Escape => Some("Escape".to_owned()),
        LogicalKey::Backspace => Some("Backspace".to_owned()),
        LogicalKey::Delete if stroke.modifiers.is_empty() => Some("Delete".to_owned()),
        LogicalKey::Delete => Some("ModifiedDelete".to_owned()),
        LogicalKey::Up => Some(vertical_action(true, stroke.modifiers, primary, extend)),
        LogicalKey::Down => Some(vertical_action(false, stroke.modifiers, primary, extend)),
        LogicalKey::PageUp => Some(fast_action("Previous", extend)),
        LogicalKey::PageDown => Some(fast_action("Next", extend)),
        LogicalKey::Home => Some(move_action("LineStart", extend)),
        LogicalKey::End => Some(move_action("LineEnd", extend)),
        _ => None,
    }
}

fn primary_action(stroke: KeyStroke) -> Option<String> {
    let shifted = stroke.modifiers.contains(LogicalModifiers::SHIFT);
    match stroke.key {
        LogicalKey::Enter if shifted => Some("SubmitKeep".to_owned()),
        LogicalKey::Enter => Some("Submit".to_owned()),
        LogicalKey::Character('v' | 'V') if shifted => Some("PasteClipboardReflow".to_owned()),
        LogicalKey::Character(
            character @ ('a' | 'A' | 'c' | 'C' | 'x' | 'X' | 'd' | 'D' | 'q' | 'Q'),
        ) if shifted => Some(format!("PrimaryShiftCharacter({character:?})")),
        LogicalKey::Character('a' | 'A') => Some("SelectAll".to_owned()),
        LogicalKey::Character('c' | 'C') => Some("Copy".to_owned()),
        LogicalKey::Character('x' | 'X') => Some("Cut".to_owned()),
        LogicalKey::Character('v' | 'V') => Some("PasteClipboard".to_owned()),
        LogicalKey::Character('d' | 'D') => Some("Duplicate".to_owned()),
        LogicalKey::Character('q' | 'Q') => Some("Quit".to_owned()),
        LogicalKey::Character('u' | 'U') if !shifted => Some("DeleteLogicalLine".to_owned()),
        LogicalKey::Character('z' | 'Z') if shifted => Some("Redo".to_owned()),
        LogicalKey::Character(character @ ('y' | 'Y')) if shifted => {
            Some(format!("PrimaryShiftCharacter({character:?})"))
        }
        LogicalKey::Character('y' | 'Y') => Some("Redo".to_owned()),
        LogicalKey::Character('z' | 'Z') => Some("Undo".to_owned()),
        LogicalKey::Character('p' | 'P') => Some("PickerPrevious".to_owned()),
        LogicalKey::Character('n' | 'N') => Some("PickerNext".to_owned()),
        LogicalKey::Character(character) if shifted || character.is_uppercase() => {
            Some(format!("PrimaryShiftCharacter({character:?})"))
        }
        LogicalKey::Character(character) => Some(format!("PrimaryCharacter({character:?})")),
        _ => None,
    }
}

fn horizontal_action(
    stroke: KeyStroke,
    platform: ShortcutPlatform,
    extend: bool,
) -> Option<String> {
    let edge = match stroke.key {
        LogicalKey::Left => VisualRowEdge::Start,
        LogicalKey::Right => VisualRowEdge::End,
        _ => return None,
    };
    if platform == ShortcutPlatform::MacOs && platform_primary(stroke.modifiers, platform) {
        return Some(if extend {
            format!("ExtendVisualRow {{ edge: {edge:?} }}")
        } else {
            format!("MoveVisualRow {{ edge: {edge:?} }}")
        });
    }
    let word = match platform {
        ShortcutPlatform::MacOs => stroke.modifiers.contains(LogicalModifiers::ALT),
        ShortcutPlatform::Portable => stroke.modifiers.contains(LogicalModifiers::CONTROL),
    };
    let movement = match (edge, word) {
        (VisualRowEdge::Start, true) => "WordBack",
        (VisualRowEdge::Start, false) => "GraphemeBack",
        (VisualRowEdge::End, true) => "WordForward",
        (VisualRowEdge::End, false) => "GraphemeForward",
    };
    Some(move_action(movement, extend))
}

fn vertical_action(
    previous: bool,
    modifiers: LogicalModifiers,
    primary: bool,
    extend: bool,
) -> String {
    if modifiers.contains(LogicalModifiers::ALT) && !primary {
        return fast_action(if previous { "Previous" } else { "Next" }, extend);
    }
    if primary && extend {
        return format!(
            "PrimaryShiftMove {{ movement: {} }}",
            if previous {
                "DocumentStart"
            } else {
                "DocumentEnd"
            }
        );
    }
    if extend {
        return move_action(if previous { "VisualUp" } else { "VisualDown" }, true);
    }
    format!(
        "EditNavigation {{ editor_movement: {}, board_movement: {} }}",
        if primary {
            if previous {
                "DocumentStart"
            } else {
                "DocumentEnd"
            }
        } else if previous {
            "VisualUp"
        } else {
            "VisualDown"
        },
        if previous { "VisualUp" } else { "VisualDown" }
    )
}

fn move_action(movement: &str, extend: bool) -> String {
    format!("Move {{ movement: {movement}, extend_selection: {extend} }}")
}

fn fast_action(direction: &str, extend: bool) -> String {
    format!("FastNavigation {{ direction: {direction}, extend_selection: {extend} }}")
}

fn platform_primary(modifiers: LogicalModifiers, platform: ShortcutPlatform) -> bool {
    let primary = match platform {
        ShortcutPlatform::MacOs if modifiers.contains(LogicalModifiers::SUPER) => {
            Some(LogicalModifiers::SUPER)
        }
        ShortcutPlatform::MacOs if modifiers.contains(LogicalModifiers::META) => {
            Some(LogicalModifiers::META)
        }
        ShortcutPlatform::Portable if modifiers.contains(LogicalModifiers::CONTROL) => {
            Some(LogicalModifiers::CONTROL)
        }
        ShortcutPlatform::MacOs | ShortcutPlatform::Portable => None,
    };
    primary.is_some_and(|primary| {
        modifiers
            .difference(primary.union(LogicalModifiers::SHIFT))
            .is_empty()
    })
}

fn has_command_modifier(modifiers: LogicalModifiers) -> bool {
    modifiers.intersects(
        LogicalModifiers::CONTROL
            .union(LogicalModifiers::SUPER)
            .union(LogicalModifiers::META)
            .union(LogicalModifiers::HYPER),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_primary_and_raw_control_contract_is_exact() {
        let super_a =
            KeyStroke::press(LogicalKey::Character('a')).with_modifiers(LogicalModifiers::SUPER);
        let meta_a =
            KeyStroke::press(LogicalKey::Character('a')).with_modifiers(LogicalModifiers::META);
        let control_a =
            KeyStroke::press(LogicalKey::Character('a')).with_modifiers(LogicalModifiers::CONTROL);
        assert_eq!(
            legacy_keypress_action(super_a, ShortcutPlatform::MacOs).as_deref(),
            Some("SelectAll")
        );
        assert_eq!(
            legacy_keypress_action(meta_a, ShortcutPlatform::MacOs).as_deref(),
            Some("SelectAll")
        );
        assert_eq!(
            legacy_keypress_action(control_a, ShortcutPlatform::MacOs),
            None
        );
        assert_eq!(
            legacy_keypress_action(control_a, ShortcutPlatform::Portable).as_deref(),
            Some("SelectAll")
        );
    }
}
