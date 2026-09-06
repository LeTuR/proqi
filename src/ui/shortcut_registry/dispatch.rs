//! Dispatch through the collision-free effective binding graph.

mod board;

use std::collections::BTreeMap;

use crate::ui::input::UiKey;
use crate::ui::{KeyPhase, KeyStroke, LogicalKey, LogicalModifiers, settings::KeyBindings};

use super::{
    context_policy::effective_board_bindings,
    intentions::{action_intention, has_command_modifier, literal, resolved},
    inventory,
    model::{
        CommandMetadata, HelpMetadata, HelpSurface, ShortcutActionId as Action, ShortcutContext,
        ShortcutContextStack, ShortcutDescriptor, ShortcutModifiers,
    },
    validation::{ShortcutRegistryError, validate_registry},
};

/// Platform policy used only to expand the abstract Primary modifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShortcutPlatform {
    MacOs,
    Portable,
}

impl ShortcutPlatform {
    pub(crate) const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Portable
        }
    }

    const fn is_macos(self) -> bool {
        matches!(self, Self::MacOs)
    }
}

/// One action selected by the active context of a registry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedShortcut {
    pub(crate) action: Option<Action>,
    pub(crate) intention: UiKey,
}

type EffectiveKey = (ShortcutContext, LogicalKey, LogicalModifiers);

/// Collision-free effective registry resolved before terminal entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ShortcutRegistry {
    platform: ShortcutPlatform,
    board_bindings: BTreeMap<char, Action>,
    descriptors: Vec<ShortcutDescriptor>,
    effective_bindings: BTreeMap<EffectiveKey, Action>,
}

impl Default for ShortcutRegistry {
    fn default() -> Self {
        Self::from_validated(&KeyBindings::default())
    }
}

impl ShortcutRegistry {
    pub(crate) fn from_validated(keys: &KeyBindings) -> Self {
        Self::build(keys, ShortcutPlatform::current())
    }

    pub(crate) fn resolve(
        keys: &KeyBindings,
        platform: ShortcutPlatform,
    ) -> Result<Self, ShortcutRegistryError> {
        validate_registry(keys, platform)?;
        Ok(Self::build(keys, platform))
    }

    fn build(keys: &KeyBindings, platform: ShortcutPlatform) -> Self {
        let board_bindings = effective_board_bindings(keys);
        let descriptors = inventory::descriptors(keys);
        let effective_bindings = effective_bindings(&descriptors, platform);
        Self {
            platform,
            board_bindings,
            descriptors,
            effective_bindings,
        }
    }

    #[cfg(test)]
    pub(crate) fn descriptors(&self) -> &[ShortcutDescriptor] {
        &self.descriptors
    }

    #[cfg(test)]
    pub(crate) fn descriptor(&self, action: Action) -> Option<&ShortcutDescriptor> {
        self.descriptors
            .iter()
            .find(|descriptor| descriptor.action == action)
    }

    pub(crate) fn binding_label(
        &self,
        context: ShortcutContext,
        actions: &[Action],
    ) -> Option<String> {
        actions
            .iter()
            .map(|action| {
                self.effective_bindings.iter().find_map(
                    |(&(candidate_context, key, modifiers), candidate_action)| {
                        (candidate_context == context
                            && candidate_action == action
                            && modifiers.is_empty())
                        .then_some(logical_key_label(key))
                    },
                )
            })
            .collect::<Option<Vec<_>>>()
            .map(|labels| labels.concat())
    }

    pub(crate) fn binding_label_for_keys(
        &self,
        context: ShortcutContext,
        bindings: &[(Action, LogicalKey)],
    ) -> Option<String> {
        bindings
            .iter()
            .map(|(action, expected_key)| {
                self.effective_bindings
                    .get(&(context, *expected_key, LogicalModifiers::NONE))
                    .filter(|candidate| *candidate == action)
                    .map(|_| logical_key_label(*expected_key))
            })
            .collect::<Option<Vec<_>>>()
            .map(|labels| labels.concat())
    }

    pub(crate) fn commands(&self) -> Vec<(Action, CommandMetadata, super::CommandExecution)> {
        let mut commands = self
            .descriptors
            .iter()
            .filter_map(|descriptor| {
                descriptor
                    .commands
                    .zip(descriptor.command_execution)
                    .map(|(metadata, execution)| (descriptor.action, metadata, execution))
            })
            .collect::<Vec<_>>();
        commands.sort_unstable_by_key(|(_, metadata, _)| metadata.order);
        commands
    }

    pub(crate) fn help(&self, surface: HelpSurface) -> Vec<(Action, HelpMetadata)> {
        let mut help = self
            .descriptors
            .iter()
            .flat_map(|descriptor| {
                descriptor
                    .help
                    .iter()
                    .filter(move |metadata| metadata.surface == surface)
                    .map(|metadata| (descriptor.action, *metadata))
            })
            .collect::<Vec<_>>();
        help.sort_unstable_by_key(|(_, metadata)| metadata.order);
        help
    }

    pub(crate) fn current(keys: &KeyBindings) -> Result<Self, ShortcutRegistryError> {
        Self::resolve(keys, ShortcutPlatform::current())
    }

    pub(crate) fn dispatch(
        &self,
        contexts: &ShortcutContextStack,
        stroke: KeyStroke,
    ) -> Option<ResolvedShortcut> {
        if matches!(stroke.phase, KeyPhase::Release) {
            return None;
        }
        let context = contexts.active()?;
        if let Some(action) = self
            .effective_bindings
            .get(&(context, stroke.key, stroke.modifiers))
            .copied()
        {
            return Some(resolved(action, action_intention(action, context, stroke)));
        }
        self.literal_or_compatible_unbound(stroke)
    }

    pub(crate) fn legacy_keypress_action(&self, stroke: KeyStroke) -> Option<String> {
        super::diagnostic_projection::legacy_keypress_action(stroke, self.platform)
    }

    pub(crate) fn preserves_editor_handoff(
        &self,
        contexts: &ShortcutContextStack,
        stroke: KeyStroke,
    ) -> bool {
        let active_action = self
            .dispatch(contexts, stroke)
            .and_then(|resolved| resolved.action);
        if matches!(
            active_action,
            Some(Action::OpenCommands | Action::ContextualTransform)
        ) {
            return true;
        }
        self.effective_bindings
            .get(&(ShortcutContext::Edit, stroke.key, stroke.modifiers))
            == Some(&Action::ContextualTransform)
    }

    fn literal_or_compatible_unbound(&self, stroke: KeyStroke) -> Option<ResolvedShortcut> {
        let LogicalKey::Character(character) = stroke.key else {
            return None;
        };
        if !has_command_modifier(stroke.modifiers) {
            let intention = if character == ' ' && stroke.modifiers.is_empty() {
                UiKey::UnmodifiedSpace
            } else {
                UiKey::Character(character)
            };
            return Some(literal(intention));
        }
        if inventory::bindings::is_primary(stroke.modifiers, self.platform.is_macos()) {
            let intention =
                if stroke.modifiers.contains(LogicalModifiers::SHIFT) || character.is_uppercase() {
                    UiKey::PrimaryShiftCharacter(character)
                } else {
                    UiKey::PrimaryCharacter(character)
                };
            return Some(literal(intention));
        }
        None
    }
}

fn logical_key_label(key: LogicalKey) -> String {
    match key {
        LogicalKey::Character(character) => character.to_uppercase().collect(),
        LogicalKey::Enter => "Enter".to_owned(),
        LogicalKey::Up => "↑".to_owned(),
        LogicalKey::Down => "↓".to_owned(),
        LogicalKey::Escape => "Esc".to_owned(),
        _ => format!("{key:?}"),
    }
}

fn effective_bindings(
    descriptors: &[ShortcutDescriptor],
    platform: ShortcutPlatform,
) -> BTreeMap<EffectiveKey, Action> {
    let mut effective = BTreeMap::new();
    for descriptor in descriptors {
        let (defaults, aliases) = match platform {
            ShortcutPlatform::MacOs => (&descriptor.macos_defaults, &descriptor.macos_aliases),
            ShortcutPlatform::Portable => {
                (&descriptor.portable_defaults, &descriptor.portable_aliases)
            }
        };
        for claim in defaults.iter().chain(aliases) {
            let ShortcutModifiers::Exact(modifiers) = claim.binding.modifiers else {
                continue;
            };
            for context in claim.contexts.iter().copied() {
                effective.insert((context, claim.binding.key, modifiers), descriptor.action);
            }
        }
    }
    effective
}
