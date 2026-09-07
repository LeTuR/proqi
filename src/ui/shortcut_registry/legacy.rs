//! Legacy TOML translation boundary, never retained by live UI state.

use serde::Deserialize;

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
        if super::legacy_validation::reserved_unshifted_character(self.transform) {
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
        if super::legacy_validation::reserved_shifted_configuration_suffix(self.delete_sentence) {
            return Err("the sentence deletion binding conflicts with a reserved Primary chord");
        }
        if super::legacy_validation::reserved_shifted_configuration_suffix(
            self.select_visual_row_start,
        ) || super::legacy_validation::reserved_shifted_configuration_suffix(
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
    .filter_map(|action| super::fixed_character_binding(action, super::ShortcutContext::Recovery))
    .any(|reserved| key == reserved)
}
