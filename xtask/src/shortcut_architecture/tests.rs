//! Accepted and rejected fixtures for the shortcut architecture detector.

use super::*;

#[test]
fn canonical_owners_and_explicit_test_fixtures_are_accepted() {
    assert!(
        check_source(
            Path::new("src/ui/shortcut_registry/inventory.rs"),
            "use crate::ui::LogicalKey; const KEY: LogicalKey = LogicalKey::Enter;",
        )
        .is_empty()
    );
    assert!(
        check_source(
            Path::new("src/adapters/terminal/input/translation.rs"),
            "use crossterm::event::KeyCode; fn decode(code: KeyCode) {}",
        )
        .is_empty()
    );
    assert!(
        check_source(
            Path::new("src/ui/tests/shortcut_fixture.rs"),
            "use crate::ui::{LogicalKey, ShortcutBinding};",
        )
        .is_empty()
    );
}

#[test]
fn raw_terminal_and_logical_key_routes_outside_their_owners_are_rejected() {
    let terminal = check_source(
        Path::new("src/ui/app.rs"),
        "use crossterm::event::{KeyCode, KeyModifiers};",
    );
    assert!(terminal[0].contains("terminal translation boundary"));

    let logical = check_source(
        Path::new("src/ui/app/help.rs"),
        "use crate::ui::LogicalKey; fn route(key: LogicalKey) {}",
    );
    assert!(logical[0].contains("registry dispatcher"));
}

#[test]
fn parallel_bindings_metadata_and_commands_inventories_are_rejected() {
    let binding = check_source(
        Path::new("src/ui/settings.rs"),
        "use crate::ui::ShortcutBinding; fn bind(value: ShortcutBinding) {}",
    );
    assert!(binding[0].contains("outside the shortcut registry"));

    let metadata = check_source(Path::new("src/ui/shortcut_metadata.rs"), "fn label() {}");
    assert!(metadata[0].contains("parallel shortcut metadata"));

    let commands = check_source(
        Path::new("src/ui/app/palette/command.rs"),
        "struct Command; impl Command { const COMMANDS: [Self; 0] = []; }",
    );
    assert!(commands[0].contains("Commands inventory"));
}

#[test]
fn semantic_character_routes_and_configured_dispatch_outside_registry_are_rejected() {
    let literal = check_source(
        Path::new("src/ui/app/help.rs"),
        "fn route(key: UiKey) { match key { UiKey::Character('j') => {}, _ => {} } }",
    );
    assert!(literal[0].contains("semantic character binding"));

    let nested = check_source(
        Path::new("src/ui/browser/management.rs"),
        "fn route(key: UiKey) { match key { UiKey::Character(character) => match character { 'R' => rename(), _ => {} }, _ => {} } }",
    );
    assert!(nested[0].contains("semantic character binding"));

    let configured = check_source(
        Path::new("src/ui/app/help.rs"),
        "fn route(app: &App, value: char) -> bool { value == app.settings.keybindings.help }",
    );
    assert!(configured[0].contains("configured keybinding access"));

    let parallel_help = check_source(
        Path::new("src/ui/shortcuts.rs"),
        "fn label(app: &App) -> char { app.settings.keybindings.help }",
    );
    assert!(parallel_help[0].contains("configured keybinding access"));
}

#[test]
fn only_test_exclusive_configuration_is_exempt() {
    let test_only = check_source(
        Path::new("src/ui/app/help.rs"),
        "#[cfg(test)] fn route(key: UiKey) { if let UiKey::Character('j') = key {} }",
    );
    assert!(test_only.is_empty());

    for source in [
        "#[cfg(not(test))] fn route(key: UiKey) { if let UiKey::Character('j') = key {} }",
        "#[cfg(all(unix, not(test)))] fn route(key: UiKey) { if let UiKey::Character('j') = key {} }",
        "#[cfg(any(test, unix))] fn route(key: UiKey) { if let UiKey::Character('j') = key {} }",
    ] {
        let findings = check_source(Path::new("src/ui/app/help.rs"), source);
        assert!(findings[0].contains("semantic character binding"));
    }
}

#[test]
fn comparisons_if_let_and_matches_macros_are_rejected() {
    for source in [
        "fn route(key: UiKey) { if let UiKey::Character(character) = key { if character == 'j' { move_down(); } } }",
        "fn route(key: UiKey) { match key { UiKey::Character(character) if matches!(character, 'j' | 'k') => move_item(), _ => {} } }",
        "fn route(key: UiKey) { if matches!(key, UiKey::Character('j')) { move_item(); } }",
    ] {
        let findings = check_source(Path::new("src/ui/app/help.rs"), source);
        assert!(findings[0].contains("semantic character binding"));
    }
}

#[test]
fn crossterm_key_events_and_parallel_footer_tables_are_rejected() {
    let terminal = check_source(
        Path::new("src/ui/app.rs"),
        "use crossterm::event::{KeyEvent, KeyEventState}; fn route(event: KeyEvent) {}",
    );
    assert!(terminal[0].contains("terminal translation boundary"));

    let enhancement_control = check_source(
        Path::new("src/adapters/terminal/control.rs"),
        "use crossterm::event::{KeyboardEnhancementFlags, PushKeyboardEnhancementFlags};",
    );
    assert!(enhancement_control.is_empty());

    let footer = check_source(
        Path::new("src/ui/browser.rs"),
        "fn controls() { let _ = [(BrowserHit::Rename, \"R\", \"Rename\")]; }",
    );
    assert!(footer[0].contains("shortcut presentation"));
}

#[test]
fn literal_text_insertion_and_typed_action_consumers_are_accepted() {
    let source = "fn route(key: UiKey) { match key { UiKey::Character(character) => insert(character), UiKey::Shortcut(action) => execute(action), _ => {} } }";
    assert!(check_source(Path::new("src/ui/app/query.rs"), source).is_empty());
    assert!(
        check_source(
            Path::new("src/ui/app/query.rs"),
            "fn copy() -> [&'static str; 2] { [\"Rename\", \"Press R to rename\"] }",
        )
        .is_empty()
    );
}
