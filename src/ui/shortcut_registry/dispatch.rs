//! Dispatch through the collision-free effective binding graph.

mod board;

use std::collections::BTreeMap;

use crate::ui::input::UiKey;
use crate::ui::{KeyPhase, KeyStroke, LogicalKey, LogicalModifiers, settings::KeyBindings};

use super::{
    intentions::{action_intention, literal, resolved},
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
pub struct ShortcutRegistry {
    platform: ShortcutPlatform,
    pub(super) descriptors: Vec<ShortcutDescriptor>,
    pub(super) projected_bindings: BTreeMap<(ShortcutContext, Action), Vec<super::ShortcutBinding>>,
    effective_bindings: BTreeMap<EffectiveKey, Action>,
}

impl Default for ShortcutRegistry {
    fn default() -> Self {
        Self::from_validated(&KeyBindings::default())
    }
}

impl ShortcutRegistry {
    /// Resolve a standalone versioned keymap TOML document for this platform.
    ///
    /// # Errors
    /// Rejects malformed documents, unknown identities, collisions, or unsafe bindings.
    pub fn from_toml(content: &str) -> Result<Self, ShortcutRegistryError> {
        let document: super::config::KeymapDocument =
            toml::from_str(content).map_err(|_| ShortcutRegistryError::MalformedDocument)?;
        document.resolve(ShortcutPlatform::current())
    }

    /// Translate a legacy character map once into validated contextual bindings.
    ///
    /// # Errors
    /// Rejects invalid legacy settings and unsafe or ambiguous effective bindings.
    pub fn from_legacy(keys: &KeyBindings) -> Result<Self, ShortcutRegistryError> {
        Self::current(keys)
    }
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
        let descriptors = inventory::descriptors(keys);
        Self::from_descriptors(descriptors, platform)
    }

    pub(super) fn from_descriptors(
        descriptors: Vec<ShortcutDescriptor>,
        platform: ShortcutPlatform,
    ) -> Self {
        let effective_bindings = effective_bindings(&descriptors, platform);
        let mut registry = Self {
            platform,
            descriptors,
            effective_bindings,
            projected_bindings: BTreeMap::new(),
        };
        registry.projected_bindings = registry.project_bindings();
        registry
    }

    #[cfg(test)]
    pub(crate) fn descriptors(&self) -> &[ShortcutDescriptor] {
        &self.descriptors
    }

    pub(crate) fn descriptor(&self, action: Action) -> Option<&ShortcutDescriptor> {
        self.descriptors
            .iter()
            .find(|descriptor| descriptor.action == action)
    }

    pub(super) const fn platform(&self) -> ShortcutPlatform {
        self.platform
    }

    pub(super) fn action_claims(
        &self,
        context: ShortcutContext,
        action: Action,
    ) -> Vec<&super::ShortcutBindingClaim> {
        let Some(descriptor) = self.descriptor(action) else {
            return Vec::new();
        };
        let (defaults, aliases) = match self.platform {
            ShortcutPlatform::MacOs => (&descriptor.macos_defaults, &descriptor.macos_aliases),
            ShortcutPlatform::Portable => {
                (&descriptor.portable_defaults, &descriptor.portable_aliases)
            }
        };
        defaults
            .iter()
            .chain(aliases)
            .filter(|claim| claim.contexts.contains(&context))
            .collect()
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
        super::inventory::bindings::vocabulary::is_text_context(context)
            .then(|| Self::literal_unbound(stroke))
            .flatten()
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
            Some(
                Action::OpenCommands
                    | Action::ContextualTransform
                    | Action::SplitThought
                    | Action::ExtractSelection
                    | Action::MergeThoughts
            )
        ) {
            return true;
        }
        self.effective_bindings
            .get(&(ShortcutContext::Edit, stroke.key, stroke.modifiers))
            == Some(&Action::ContextualTransform)
    }

    fn literal_unbound(stroke: KeyStroke) -> Option<ResolvedShortcut> {
        let LogicalKey::Character(character) = stroke.key else {
            return None;
        };
        if super::validation::reserves_printable(stroke.modifiers) {
            let intention = if character == ' ' && stroke.modifiers.is_empty() {
                UiKey::UnmodifiedSpace
            } else {
                UiKey::Character(character)
            };
            return Some(literal(intention));
        }
        None
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
