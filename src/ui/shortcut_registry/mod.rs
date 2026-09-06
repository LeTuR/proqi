//! Unified terminal-independent shortcut registry and dispatcher.

mod command_execution;
mod context_policy;
mod diagnostic_projection;
mod dispatch;
mod intentions;
mod inventory;
mod model;
pub(crate) mod presentation;
mod validation;

pub(crate) use command_execution::{
    BoardCommand as PaletteBoardCommand, CommandExecution, EditorCommand as PaletteEditorCommand,
    EntryCommand as PaletteEntryCommand, PasteCommand as PalettePasteCommand,
    RuntimeCommand as PaletteRuntimeCommand, SelectionCommand as PaletteSelectionCommand,
    SubmissionCommand as PaletteSubmissionCommand,
    TransformationCommand as PaletteTransformationCommand,
};
pub(crate) use dispatch::ShortcutRegistry;
#[cfg(test)]
pub(crate) use dispatch::{ResolvedShortcut, ShortcutPlatform};
pub(super) use inventory::fixed_character_binding;
pub(crate) use model::{
    CommandAvailability, CommandLabel, CommandMetadata, HelpAvailability, HelpSurface,
};
pub use model::{
    ShortcutActionId, ShortcutBinding, ShortcutBindingClaim, ShortcutContext, ShortcutContextStack,
    ShortcutDescriptor, ShortcutModifiers, ShortcutSafety,
};

#[cfg(test)]
mod tests;
