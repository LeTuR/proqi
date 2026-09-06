//! Typed load-time validation of the effective shortcut graph.

use std::collections::{BTreeMap, BTreeSet};

use crate::ui::{LogicalKey, LogicalModifiers, settings::KeyBindings};

use super::{
    dispatch::ShortcutPlatform,
    inventory,
    model::{
        ShortcutActionId as Action, ShortcutBinding, ShortcutContext as Context,
        ShortcutDescriptor, ShortcutModifiers, ShortcutSafety,
    },
};

pub(crate) use super::errors::ShortcutRegistryError;

pub(super) fn validate_registry(
    keys: &KeyBindings,
    platform: ShortcutPlatform,
) -> Result<(), ShortcutRegistryError> {
    keys.validate()
        .map_err(ShortcutRegistryError::InvalidOverride)?;
    let descriptors = inventory::descriptors(keys);
    validate_descriptors(&descriptors, platform)
}

pub(super) fn validate_descriptors(
    descriptors: &[ShortcutDescriptor],
    platform: ShortcutPlatform,
) -> Result<(), ShortcutRegistryError> {
    validate_descriptor_identities(descriptors)?;
    validate_contexts(descriptors)?;
    validate_binding_claims(descriptors, platform)?;
    validate_escape(descriptors, platform)?;
    validate_recovery(descriptors, platform)?;
    validate_escape_routes(descriptors, platform)?;
    validate_presentation_references(descriptors)?;
    validate_commands(descriptors)
}

fn validate_descriptor_identities(
    descriptors: &[ShortcutDescriptor],
) -> Result<(), ShortcutRegistryError> {
    let mut actions = BTreeSet::new();
    let mut diagnostics = BTreeSet::new();
    for descriptor in descriptors {
        if !actions.insert(descriptor.action) {
            return Err(ShortcutRegistryError::DuplicateDescriptor(
                descriptor.action,
            ));
        }
        if descriptor.diagnostics.is_empty() {
            return Err(ShortcutRegistryError::MissingDiagnostics(descriptor.action));
        }
        if !diagnostics.insert(descriptor.diagnostics) {
            return Err(ShortcutRegistryError::DuplicateDiagnostics(
                descriptor.diagnostics,
            ));
        }
    }
    for action in inventory::DIRECT_ACTIONS {
        if !actions.contains(action) {
            return Err(ShortcutRegistryError::MissingDescriptor(*action));
        }
    }
    Ok(())
}

fn validate_contexts(descriptors: &[ShortcutDescriptor]) -> Result<(), ShortcutRegistryError> {
    for descriptor in descriptors {
        let mut contexts = BTreeSet::new();
        for context in &descriptor.contexts {
            if !contexts.insert(*context) {
                return Err(ShortcutRegistryError::DuplicateContext {
                    action: descriptor.action,
                    context: *context,
                });
            }
        }
    }
    Ok(())
}

fn validate_binding_claims(
    descriptors: &[ShortcutDescriptor],
    platform: ShortcutPlatform,
) -> Result<(), ShortcutRegistryError> {
    let mut claims = BTreeMap::new();
    for descriptor in descriptors {
        let (defaults, aliases) = match platform {
            ShortcutPlatform::MacOs => (&descriptor.macos_defaults, &descriptor.macos_aliases),
            ShortcutPlatform::Portable => {
                (&descriptor.portable_defaults, &descriptor.portable_aliases)
            }
        };
        for binding_claim in defaults.iter().chain(aliases) {
            validate_modifier(descriptor.action, &binding_claim.binding)?;
            for context in &binding_claim.contexts {
                validate_text_safety(descriptor.action, *context, &binding_claim.binding)?;
                claim(
                    &mut claims,
                    *context,
                    binding_claim.binding,
                    descriptor.action,
                )?;
            }
        }
    }
    Ok(())
}

fn claim(
    claims: &mut BTreeMap<(Context, ShortcutBinding), Action>,
    context: Context,
    binding: ShortcutBinding,
    action: Action,
) -> Result<(), ShortcutRegistryError> {
    if let Some(first) = claims.insert((context, binding), action)
        && first != action
    {
        return Err(ShortcutRegistryError::DuplicateBinding {
            context,
            first,
            second: action,
        });
    }
    Ok(())
}

fn validate_modifier(
    action: Action,
    binding: &ShortcutBinding,
) -> Result<(), ShortcutRegistryError> {
    match binding.modifiers {
        ShortcutModifiers::Exact(_) => Ok(()),
        ShortcutModifiers::Primary
        | ShortcutModifiers::PrimaryShift
        | ShortcutModifiers::Contextual => Err(ShortcutRegistryError::InvalidModifier { action }),
    }
}

fn validate_text_safety(
    action: Action,
    context: Context,
    binding: &ShortcutBinding,
) -> Result<(), ShortcutRegistryError> {
    let printable =
        matches!(binding.key, LogicalKey::Character(character) if !character.is_control());
    let reserved_text = matches!(
        binding.modifiers,
        ShortcutModifiers::Exact(modifiers)
            if reserves_printable(modifiers)
    );
    if inventory::bindings::vocabulary::is_text_context(context) && printable && reserved_text {
        return Err(ShortcutRegistryError::TextInputTheft { action, context });
    }
    Ok(())
}

pub(super) fn reserves_printable(modifiers: LogicalModifiers) -> bool {
    modifiers
        .difference(LogicalModifiers::SHIFT.union(LogicalModifiers::ALT))
        .is_empty()
        || modifiers.contains(LogicalModifiers::CONTROL.union(LogicalModifiers::ALT))
            && modifiers
                .difference(
                    LogicalModifiers::CONTROL
                        .union(LogicalModifiers::ALT)
                        .union(LogicalModifiers::SHIFT),
                )
                .is_empty()
}

fn validate_escape(
    descriptors: &[ShortcutDescriptor],
    platform: ShortcutPlatform,
) -> Result<(), ShortcutRegistryError> {
    let Some(close) = descriptors
        .iter()
        .find(|descriptor| descriptor.safety == ShortcutSafety::InvariantClose)
    else {
        return Err(ShortcutRegistryError::MissingDescriptor(Action::Close));
    };
    let bindings = match platform {
        ShortcutPlatform::MacOs => &close.macos_defaults,
        ShortcutPlatform::Portable => &close.portable_defaults,
    };
    for context in inventory::ESCAPE_CONTEXTS.iter().copied() {
        let owns_escape = close.contexts.contains(&context)
            && bindings.iter().any(|claim| {
                claim.contexts.contains(&context)
                    && claim.binding.key == LogicalKey::Escape
                    && claim.binding.modifiers == ShortcutModifiers::Exact(LogicalModifiers::NONE)
            });
        if !owns_escape {
            return Err(ShortcutRegistryError::InvariantEscapeLoss { context });
        }
    }
    Ok(())
}

fn validate_recovery(
    descriptors: &[ShortcutDescriptor],
    platform: ShortcutPlatform,
) -> Result<(), ShortcutRegistryError> {
    for descriptor in descriptors
        .iter()
        .filter(|descriptor| descriptor.safety == ShortcutSafety::RecoveryCritical)
    {
        let action = descriptor.action;
        let reachable = {
            let (defaults, aliases) = match platform {
                ShortcutPlatform::MacOs => (&descriptor.macos_defaults, &descriptor.macos_aliases),
                ShortcutPlatform::Portable => {
                    (&descriptor.portable_defaults, &descriptor.portable_aliases)
                }
            };
            descriptor.contexts.contains(&Context::Recovery)
                && defaults
                    .iter()
                    .chain(aliases)
                    .any(|claim| claim.contexts.contains(&Context::Recovery))
        };
        if !reachable {
            return Err(ShortcutRegistryError::UnreachableRecovery(action));
        }
    }
    Ok(())
}

fn validate_escape_routes(
    descriptors: &[ShortcutDescriptor],
    platform: ShortcutPlatform,
) -> Result<(), ShortcutRegistryError> {
    for (context, action) in [
        (Context::Board, Action::OpenCommands),
        (Context::Board, Action::Quit),
        (Context::InsertionBoundary, Action::OpenCommands),
        (Context::InsertionBoundary, Action::Quit),
    ] {
        let reachable = descriptors
            .iter()
            .find(|descriptor| descriptor.action == action)
            .is_some_and(|descriptor| {
                let (defaults, aliases) = match platform {
                    ShortcutPlatform::MacOs => {
                        (&descriptor.macos_defaults, &descriptor.macos_aliases)
                    }
                    ShortcutPlatform::Portable => {
                        (&descriptor.portable_defaults, &descriptor.portable_aliases)
                    }
                };
                defaults
                    .iter()
                    .chain(aliases)
                    .any(|claim| claim.contexts.contains(&context))
            });
        if !reachable {
            return Err(ShortcutRegistryError::UnreachableAction { context, action });
        }
    }
    Ok(())
}

fn validate_commands(descriptors: &[ShortcutDescriptor]) -> Result<(), ShortcutRegistryError> {
    let mut orders = BTreeSet::new();
    for (order, (action, label)) in Action::COMMANDS.into_iter().enumerate() {
        let expected = inventory::metadata::command_metadata(action, order, label);
        let Some(descriptor) = descriptors
            .iter()
            .find(|descriptor| descriptor.action == action)
        else {
            return Err(ShortcutRegistryError::StaleCommandsReference(action));
        };
        if descriptor.commands != Some(expected) || !orders.insert(expected.order) {
            return Err(ShortcutRegistryError::StaleCommandsReference(action));
        }
        let Some(execution) = descriptor.command_execution else {
            return Err(ShortcutRegistryError::MissingCommandExecution(action));
        };
        if Some(execution) != super::command_execution::execution_for(action) {
            return Err(ShortcutRegistryError::StaleCommandExecution(action));
        }
    }
    for descriptor in descriptors {
        if descriptor.commands.is_none() && descriptor.command_execution.is_some() {
            return Err(ShortcutRegistryError::UnexpectedCommandExecution(
                descriptor.action,
            ));
        }
    }
    Ok(())
}

fn validate_presentation_references(
    descriptors: &[ShortcutDescriptor],
) -> Result<(), ShortcutRegistryError> {
    for descriptor in descriptors {
        if descriptor.help != inventory::metadata::help_metadata(descriptor.action) {
            return Err(ShortcutRegistryError::StaleHelpReference(descriptor.action));
        }
        if descriptor.footer != inventory::metadata::footer_metadata(descriptor.action) {
            return Err(ShortcutRegistryError::StaleFooterReference(
                descriptor.action,
            ));
        }
    }
    Ok(())
}
