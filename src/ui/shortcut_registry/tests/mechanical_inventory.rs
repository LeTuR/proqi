//! Mechanical discovery and independently pinned parity of the effective graph.
use super::super::{ShortcutPlatform, ShortcutRegistry, inventory};
use crate::ui::{
    KeyBindings, KeyStroke, ShortcutActionId, ShortcutBinding, ShortcutBindingClaim,
    ShortcutContext, ShortcutContextStack, ShortcutDescriptor, ShortcutModifiers,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
};

type Claims = BTreeSet<(ShortcutContext, ShortcutBinding, ShortcutActionId)>;

fn claims(
    descriptor: &ShortcutDescriptor,
    platform: ShortcutPlatform,
) -> impl Iterator<Item = (&'static str, &ShortcutBindingClaim)> {
    let (defaults, aliases) = match platform {
        ShortcutPlatform::MacOs => (&descriptor.macos_defaults, &descriptor.macos_aliases),
        ShortcutPlatform::Portable => (&descriptor.portable_defaults, &descriptor.portable_aliases),
    };
    [("default", defaults), ("alias", aliases)]
        .into_iter()
        .flat_map(|(origin, claims)| claims.iter().map(move |claim| (origin, claim)))
}

#[test]
fn mechanical_inventory_proves_every_claim_and_metadata_identity() {
    for platform in [ShortcutPlatform::MacOs, ShortcutPlatform::Portable] {
        let registry = ShortcutRegistry::resolve(&KeyBindings::default(), platform).unwrap();
        let mut rows = BTreeSet::new();
        for descriptor in registry.descriptors() {
            println!(
                "DESCRIPTOR\t{platform:?}\t{}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}",
                descriptor.diagnostics,
                descriptor.contexts,
                descriptor.safety,
                descriptor.help,
                descriptor.footer,
                descriptor.commands,
                descriptor.command_execution
            );
            inspect_descriptor(&registry, platform, descriptor, &mut rows);
        }
        let contexts = rows.iter().map(|row| row.0).collect::<BTreeSet<_>>();
        assert_eq!(
            contexts.len(),
            inventory::bindings::vocabulary::KEYBOARD_CONTEXTS.len()
        );
        println!(
            "TOTAL\t{platform:?}\tactions={}\tcontexts={}\tbindings={}",
            registry.descriptors().len(),
            contexts.len(),
            rows.len()
        );
    }
}

fn inspect_descriptor(
    registry: &ShortcutRegistry,
    platform: ShortcutPlatform,
    descriptor: &ShortcutDescriptor,
    rows: &mut Claims,
) {
    for (origin, claim) in claims(descriptor, platform) {
        let ShortcutModifiers::Exact(modifiers) = claim.binding.modifiers else {
            panic!("unresolved modifier");
        };
        for context in &claim.contexts {
            let stroke = KeyStroke::press(claim.binding.key).with_modifiers(modifiers);
            let resolved = registry
                .dispatch(&ShortcutContextStack::new([*context]), stroke)
                .unwrap();
            assert_eq!(resolved.action, Some(descriptor.action));
            rows.insert((*context, claim.binding, descriptor.action));
            println!(
                "BINDING\t{platform:?}\t{context:?}\t{}\t{origin}\t{stroke:?}\t{:?}",
                descriptor.diagnostics, resolved.intention
            );
        }
    }
}

#[test]
fn defaults_preserve_every_foundation_event_and_intention() {
    let mut actual = BTreeMap::new();
    for platform in [ShortcutPlatform::MacOs, ShortcutPlatform::Portable] {
        let registry = ShortcutRegistry::resolve(&KeyBindings::default(), platform).unwrap();
        for descriptor in registry.descriptors() {
            collect_parity(&registry, platform, descriptor, &mut actual);
        }
    }
    let actual = actual
        .into_iter()
        .map(|(identity, rows)| format!("{identity}\t{}\t{}", rows.len(), digest(&rows)))
        .collect::<Vec<_>>()
        .join("\n");
    let expected = include_str!("foundation_parity.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(actual, expected);
}

fn collect_parity(
    registry: &ShortcutRegistry,
    platform: ShortcutPlatform,
    descriptor: &ShortcutDescriptor,
    actual: &mut BTreeMap<String, BTreeSet<String>>,
) {
    for (_, claim) in claims(descriptor, platform) {
        let ShortcutModifiers::Exact(modifiers) = claim.binding.modifiers else {
            panic!("unresolved modifier");
        };
        for context in &claim.contexts {
            if *context == ShortcutContext::Recovery && descriptor.action == ShortcutActionId::Close
            {
                continue;
            }
            let stroke = KeyStroke::press(claim.binding.key).with_modifiers(modifiers);
            let resolved = registry
                .dispatch(&ShortcutContextStack::new([*context]), stroke)
                .unwrap();
            actual
                .entry(format!("{platform:?}\t{context:?}"))
                .or_default()
                .insert(format!(
                    "{}\t{stroke:?}\t{:?}",
                    descriptor.diagnostics, resolved.intention
                ));
        }
    }
}

fn digest(rows: &BTreeSet<String>) -> String {
    let bytes = Sha256::digest(format!(
        "{}\n",
        rows.iter().cloned().collect::<Vec<_>>().join("\n")
    ));
    let mut result = String::new();
    for byte in bytes {
        write!(&mut result, "{byte:02x}").unwrap();
    }
    result
}
