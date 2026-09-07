//! Content-redacted typed keymap configuration failures.

use super::model::{ShortcutActionId as Action, ShortcutContext as Context};
use std::fmt;

/// Deterministic configuration failure reported before terminal setup.
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(
    missing_docs,
    reason = "variants identify closed configuration failure classes"
)]
pub enum ShortcutRegistryError {
    MalformedDocument,
    UnsupportedVersion(u16),
    UnknownContext(usize),
    UnknownAction {
        context: Context,
        index: usize,
    },
    UnsupportedContext {
        context: Context,
        action: Action,
    },
    TooManyAliases {
        context: Context,
        action: Action,
    },
    MalformedAlias {
        context: Context,
        action: Action,
        index: usize,
    },
    DuplicateAlias {
        context: Context,
        action: Action,
        index: usize,
    },
    InvalidOverride(&'static str),
    DuplicateBinding {
        context: Context,
        first: Action,
        second: Action,
    },
    DuplicateContext {
        action: Action,
        context: Context,
    },
    InvalidModifier {
        action: Action,
    },
    TextInputTheft {
        action: Action,
        context: Context,
    },
    InvariantEscapeLoss {
        context: Context,
    },
    UnreachableRecovery(Action),
    UnreachableAction {
        context: Context,
        action: Action,
    },
    MissingDescriptor(Action),
    DuplicateDescriptor(Action),
    MissingDiagnostics(Action),
    DuplicateDiagnostics(&'static str),
    StaleCommandsReference(Action),
    MissingCommandExecution(Action),
    StaleCommandExecution(Action),
    UnexpectedCommandExecution(Action),
    StaleHelpReference(Action),
    StaleFooterReference(Action),
}

impl fmt::Display for ShortcutRegistryError {
    #[expect(
        clippy::too_many_lines,
        reason = "the closed typed validation error contract keeps every actionable message exhaustive"
    )]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedDocument => formatter.write_str(
                "invalid keymap document; use schema_version = 1 and context/action alias lists",
            ),
            Self::UnsupportedVersion(version) => write!(
                formatter,
                "unsupported keymap schema_version {version}; expected 1"
            ),
            Self::UnknownContext(index) => write!(
                formatter,
                "unknown keymap context at position {index}; use a documented context identity"
            ),
            Self::UnknownAction { context, index } => write!(
                formatter,
                "unknown action at position {index} in {}; use a documented action identity",
                context.configuration_id()
            ),
            Self::UnsupportedContext { context, action } => write!(
                formatter,
                "action {} is not supported in {}",
                action.diagnostics_id(),
                context.configuration_id()
            ),
            Self::TooManyAliases { context, action } => write!(
                formatter,
                "action {} in {} supports at most 128 aliases",
                action.diagnostics_id(),
                context.configuration_id()
            ),
            Self::MalformedAlias {
                context,
                action,
                index,
            } => write!(
                formatter,
                "invalid alias {index} for {} in {}; use a logical key and distinct supported modifiers",
                action.diagnostics_id(),
                context.configuration_id()
            ),
            Self::DuplicateAlias {
                context,
                action,
                index,
            } => write!(
                formatter,
                "duplicate alias {index} for {} in {}; remove overlapping aliases",
                action.diagnostics_id(),
                context.configuration_id()
            ),
            Self::InvalidOverride(message) => formatter.write_str(message),
            Self::DuplicateBinding {
                context,
                first,
                second,
            } => write!(
                formatter,
                "shortcut actions {} and {} claim the same binding in {context:?}",
                first.diagnostics_id(),
                second.diagnostics_id()
            ),
            Self::DuplicateContext { action, context } => write!(
                formatter,
                "shortcut action {} repeats context {context:?}",
                action.diagnostics_id()
            ),
            Self::InvalidModifier { action } => write!(
                formatter,
                "shortcut action {} has an ineligible modifier combination",
                action.diagnostics_id()
            ),
            Self::TextInputTheft { action, context } => write!(
                formatter,
                "shortcut action {} would steal printable input in {context:?}",
                action.diagnostics_id()
            ),
            Self::InvariantEscapeLoss { context } => {
                write!(
                    formatter,
                    "Escape must remain the close or cancel route in {context:?}"
                )
            }
            Self::UnreachableAction { context, action } => write!(
                formatter,
                "required action {} in {} needs at least one binding",
                action.diagnostics_id(),
                context.configuration_id()
            ),
            Self::UnreachableRecovery(action) => write_action_error(
                formatter,
                *action,
                "required recovery action",
                "is unreachable",
            ),
            Self::MissingDescriptor(action) => write_action_error(
                formatter,
                *action,
                "shortcut action",
                "has no registry descriptor",
            ),
            Self::DuplicateDescriptor(action) => write_action_error(
                formatter,
                *action,
                "shortcut action",
                "has duplicate registry descriptors",
            ),
            Self::MissingDiagnostics(action) => write!(
                formatter,
                "shortcut action {action:?} has no diagnostics identity"
            ),
            Self::DuplicateDiagnostics(identity) => {
                write!(
                    formatter,
                    "shortcut diagnostics identity {identity} is duplicated"
                )
            }
            Self::StaleCommandsReference(action) => write_action_error(
                formatter,
                *action,
                "Commands references missing shortcut action",
                "",
            ),
            Self::MissingCommandExecution(action) => write_action_error(
                formatter,
                *action,
                "Commands action",
                "has no execution owner",
            ),
            Self::StaleCommandExecution(action) => write_action_error(
                formatter,
                *action,
                "Commands action",
                "has the wrong execution owner",
            ),
            Self::UnexpectedCommandExecution(action) => write_action_error(
                formatter,
                *action,
                "non-Commands action",
                "has a Commands execution owner",
            ),
            Self::StaleHelpReference(action) => write_action_error(
                formatter,
                *action,
                "Help references missing shortcut action",
                "",
            ),
            Self::StaleFooterReference(action) => write_action_error(
                formatter,
                *action,
                "footer references missing shortcut action",
                "",
            ),
        }
    }
}

fn write_action_error(
    formatter: &mut fmt::Formatter<'_>,
    action: Action,
    subject: &str,
    problem: &str,
) -> fmt::Result {
    if problem.is_empty() {
        write!(formatter, "{subject} {}", action.diagnostics_id())
    } else {
        write!(formatter, "{subject} {} {problem}", action.diagnostics_id())
    }
}

impl std::error::Error for ShortcutRegistryError {}
