//! User-configurable terminal appearance and board bindings.

use serde::Deserialize;

/// Optional enhanced keyboard reporting for compatible terminal emulators.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum KeyboardEnhancement {
    /// Enable only the flags compatible with the detected terminal transport.
    #[default]
    Auto,
    /// Use portable Crossterm key events without enhancement negotiation.
    Disabled,
}

/// Complete UI configuration loaded from the platform config directory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiSettings {
    /// Permit automatic stable-release checks on interactive release startup.
    pub check_for_updates: bool,
    /// Show the complete canonical session identifier beside the session name when it fits.
    pub show_session_id: bool,
    /// Continue recognized Markdown list items when Enter inserts a newline.
    pub smart_lists: bool,
    /// Spaces inserted for every list indentation level.
    pub list_indent_width: u8,
    /// Exact text inserted between thoughts by the merge transformation.
    pub merge_separator: String,
    /// Keyboard protocol negotiation.
    pub keyboard_enhancement: KeyboardEnhancement,
    /// Remappable direct board keys.
    pub keybindings: KeyBindings,
    /// Vertical separation between thoughts.
    pub density: BoardDensity,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            check_for_updates: true,
            show_session_id: false,
            smart_lists: true,
            list_indent_width: 2,
            merge_separator: "\n\n".to_owned(),
            keyboard_enhancement: KeyboardEnhancement::default(),
            keybindings: KeyBindings::default(),
            density: BoardDensity::default(),
        }
    }
}

/// Board spacing preference.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BoardDensity {
    /// A restrained separator row between thoughts.
    #[default]
    Comfortable,
    /// Minimize vertical separation in constrained panes.
    Compact,
}

/// Direct character bindings for common board actions.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct KeyBindings {
    /// Create thought.
    pub new: char,
    /// Edit focused thought.
    pub edit: char,
    /// Delete focused thought.
    pub delete: char,
    /// Copy focused thought.
    pub copy: char,
    /// Cut focused thought.
    pub cut: char,
    /// Submit the focused thought, removing it after acceptance.
    #[serde(alias = "send")]
    pub submit_remove: char,
    /// Submit and preserve the focused thought.
    #[serde(alias = "submit")]
    pub submit_keep: char,
    /// Undo board action.
    pub undo: char,
    /// Move focus upward.
    pub focus_up: char,
    /// Move focus downward.
    pub focus_down: char,
    /// Extend a range upward; Primary plus this shifted key reorders upward.
    #[serde(alias = "move_up")]
    pub range_up: char,
    /// Extend a range downward; Primary plus this shifted key reorders downward.
    #[serde(alias = "move_down")]
    pub range_down: char,
    /// Toggle expanded presentation.
    pub collapse: char,
    /// Toggle the focused thought in the multi-selection.
    pub select: char,
    /// Apply the contextual thought transformation.
    pub transform: char,
    /// Select every live thought in board order.
    pub select_all: char,
    /// Latch contiguous range selection.
    pub range_select: char,
    /// Search thought content.
    pub search: char,
    /// Discover commands.
    pub commands: char,
    /// Show help.
    pub help: char,
    /// Exit.
    pub quit: char,
    /// Toggle the macOS screenshot inbox.
    pub screenshot_inbox: char,
    /// Paste exactly in Board mode; its uppercase form pastes with reflow.
    pub paste: char,
    /// Shifted Primary chord suffix for sentence deletion.
    pub delete_sentence: char,
    /// Shifted Primary chord suffix for selection to the visual-row start.
    pub select_visual_row_start: char,
    /// Shifted Primary chord suffix for selection to the visual-row end.
    pub select_visual_row_end: char,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            new: 'n',
            edit: 'e',
            delete: 'd',
            copy: 'y',
            cut: 'x',
            submit_remove: 's',
            submit_keep: 'S',
            undo: 'u',
            focus_up: 'k',
            focus_down: 'j',
            range_up: 'K',
            range_down: 'J',
            collapse: 'c',
            select: ' ',
            transform: 't',
            select_all: 'a',
            range_select: 'v',
            search: '/',
            commands: ':',
            help: '?',
            quit: 'q',
            screenshot_inbox: 'i',
            paste: 'p',
            delete_sentence: 'U',
            select_visual_row_start: 'H',
            select_visual_row_end: 'L',
        }
    }
}

impl KeyBindings {
    pub(crate) fn delete_label(&self) -> String {
        format!("{}/Del", key_label(self.delete))
    }

    /// Reject ambiguous board characters before the terminal starts.
    ///
    /// # Errors
    ///
    /// Returns an error for control characters or duplicate bindings.
    pub fn validate(&self) -> Result<(), &'static str> {
        if is_reserved_recovery_key(self.quit) {
            return Err("the quit binding cannot use the reserved recovery keys r or w");
        }
        if self.transform.is_control() {
            return Err("keybindings must be distinct printable characters");
        }
        if !self.paste.is_ascii_lowercase() {
            return Err("the paste binding must be one lowercase ASCII letter");
        }
        if super::shortcut_registry::presentation::reserved_unshifted_character(self.transform) {
            return Err("the transform binding conflicts with a reserved Primary shortcut");
        }
        if !self.delete_sentence.is_ascii_uppercase() {
            return Err("the sentence deletion binding must be one uppercase ASCII letter");
        }
        if !self.select_visual_row_start.is_ascii_uppercase()
            || !self.select_visual_row_end.is_ascii_uppercase()
        {
            return Err("visual-row selection bindings must be uppercase ASCII letters");
        }
        if super::shortcut_registry::presentation::reserved_shifted_configuration_suffix(
            self.delete_sentence,
        ) {
            return Err("the sentence deletion binding conflicts with a reserved Primary chord");
        }
        if super::shortcut_registry::presentation::reserved_shifted_configuration_suffix(
            self.select_visual_row_start,
        ) || super::shortcut_registry::presentation::reserved_shifted_configuration_suffix(
            self.select_visual_row_end,
        ) {
            return Err("visual-row selection bindings conflict with a reserved Primary chord");
        }
        let shifted_primary = [
            self.delete_sentence,
            self.select_visual_row_start,
            self.select_visual_row_end,
        ];
        for (index, value) in shifted_primary.iter().enumerate() {
            if shifted_primary[index + 1..].contains(value) {
                return Err("shifted Primary bindings must be distinct");
            }
        }
        let values = [
            self.new,
            self.edit,
            self.delete,
            self.copy,
            self.cut,
            self.submit_remove,
            self.submit_keep,
            self.undo,
            self.focus_up,
            self.focus_down,
            self.range_up,
            self.range_down,
            self.collapse,
            self.select,
            self.select_all,
            self.range_select,
            self.search,
            self.commands,
            self.help,
            self.quit,
            self.screenshot_inbox,
            self.delete_sentence,
        ];
        for (index, value) in values.iter().enumerate() {
            if value.is_control() || values[index + 1..].contains(value) {
                return Err("keybindings must be distinct printable characters");
            }
        }
        Ok(())
    }
}

fn is_reserved_recovery_key(key: char) -> bool {
    [
        super::ShortcutActionId::RetryStorage,
        super::ShortcutActionId::ExportRecovery,
    ]
    .into_iter()
    .filter_map(|action| {
        super::shortcut_registry::fixed_character_binding(action, super::ShortcutContext::Recovery)
    })
    .any(|reserved| key == reserved)
}

pub(crate) fn key_label(key: char) -> String {
    match key {
        ' ' => "Space".to_owned(),
        '\t' => "Tab".to_owned(),
        '\n' | '\r' => "Enter".to_owned(),
        _ => key.to_string(),
    }
}

pub(crate) fn primary_key_label(suffix: &str) -> String {
    crate::application::PrimaryKeyPlatform::current().label(suffix)
}

#[cfg(test)]
#[path = "settings/tests.rs"]
mod tests;
