//! Resolved aliases drive real application effects, text ownership and controls.
use proqi::{
    application::Effect,
    ports::editor::CursorMovement,
    ui::{
        HitTarget, KeyStroke, LogicalKey, LogicalModifiers, PointerButton, PointerKind, UiInput,
        UiKey, UiSettings,
    },
};
use ratatui_core::layout::Rect;

use super::{Fixture, draw, durable_thought, key_input, snapshot_support};

fn configured(document: &str) -> Fixture {
    let settings = UiSettings {
        shortcuts: proqi::ui::ShortcutRegistry::from_toml(document).expect("safe keymap"),
        ..UiSettings::default()
    };
    Fixture::with_settings(settings)
}

fn key(key: LogicalKey) -> UiInput {
    UiInput::KeyStroke(KeyStroke::press(key))
}

#[test]
fn multiple_aliases_replace_default_delete_and_preserve_one_step_undo() {
    for alias in [LogicalKey::Function(5), LogicalKey::Function(6)] {
        let mut fixture = configured(
            "schema_version=1\n[bindings.board]\n\"thought.delete\"=[{key='F5'},{key='F6'}]",
        );
        durable_thought(&mut fixture, "Grüße 界");
        for removed in [LogicalKey::Character('d'), LogicalKey::Delete] {
            assert!(fixture.effects(key(removed)).is_empty());
            assert_eq!(fixture.app.state.board.live_thoughts().len(), 1);
        }
        let effects = fixture.effects(key(alias));
        assert!(
            matches!(effects.as_slice(), [Effect::CommitBoardOperation(operation)] if operation.kind == proqi::domain::BoardOperationKind::Delete)
        );
        fixture.input(key(LogicalKey::Escape));
        fixture.input(key_input(UiKey::Undo));
        assert_eq!(
            fixture.app.state.board.live_thoughts()[0].content,
            "Grüße 界"
        );
    }
}

#[test]
fn commands_only_binding_executes_with_the_existing_availability_and_effect_owner() {
    let mut fixture = configured(
        "schema_version=1\n[bindings.board]\n\"agents.refresh\"=[{key='F5'}]\n[bindings.commands]\n\"agents.refresh\"=[{key='F6'}]",
    );
    for content in ["first", "second", "third"] {
        durable_thought(&mut fixture, content);
    }
    fixture.input(key(LogicalKey::Character('k')));
    fixture.input(key(LogicalKey::Character('v')));
    let effects = fixture.effects(key(LogicalKey::Function(5)));
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::DiscoverAgents))
    );
    assert!(fixture.app.palette_view().is_none());
    fixture.input(key(LogicalKey::Character('k')));
    let selected = fixture
        .app
        .state
        .board
        .live_thoughts()
        .iter()
        .filter(|thought| fixture.app.thought_selected(thought.id))
        .map(|thought| thought.content.as_str())
        .collect::<Vec<_>>();
    assert_eq!(selected, ["first", "second"]);
    fixture.input(key(LogicalKey::Character(':')));
    let effects = fixture.effects(key(LogicalKey::Function(6)));
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::DiscoverAgents))
    );
    assert!(fixture.app.palette_view().is_none());
}

#[test]
fn configured_editor_fast_selection_is_semantic_even_without_shift() {
    let mut fixture = configured(
        "schema_version=1\n[bindings.edit]\n\"navigation.fast_extend_next\"=[{key='F5'}]",
    );
    durable_thought(&mut fixture, "one\ntwo\nthree\nfour\nfive\nsix\nseven");
    fixture.input(key(LogicalKey::Enter));
    fixture.input(key_input(UiKey::Move {
        movement: CursorMovement::DocumentStart,
        extend_selection: false,
    }));
    draw(&mut fixture, 80, 24);
    fixture.input(key(LogicalKey::Function(5)));
    let snapshot = fixture.app.editor_snapshot().expect("editor");
    assert_eq!(snapshot.cursor, proqi::domain::TextPosition::new(5, 0));
    assert!(snapshot.selection.is_some());
}

#[test]
fn board_aliases_do_not_steal_shift_option_altgr_or_unicode_text_from_edit() {
    let mut fixture =
        configured("schema_version=1\n[bindings.board]\n\"thought.delete\"=[{key='ä'}]");
    durable_thought(&mut fixture, "placeholder");
    fixture.input(key(LogicalKey::Enter));
    fixture.input(key_input(UiKey::SelectAll));
    for modifiers in [
        LogicalModifiers::NONE,
        LogicalModifiers::SHIFT,
        LogicalModifiers::ALT,
        LogicalModifiers::CONTROL.union(LogicalModifiers::ALT),
    ] {
        fixture.input(UiInput::KeyStroke(
            KeyStroke::press(LogicalKey::Character('ä')).with_modifiers(modifiers),
        ));
    }
    assert_eq!(
        fixture.app.editor_snapshot().expect("editor").content,
        "ääää"
    );
}

#[test]
fn footer_help_and_mouse_share_resolved_alias_geometry_across_widths() {
    let document = "schema_version=1\n[bindings.board]\n\"thought.delete\"=[{key='F5'},{key='F6'}]";
    for width in [28, 60, 100] {
        let mut fixture = configured(document);
        durable_thought(&mut fixture, "visible");
        let terminal = draw(&mut fixture, width, 14);
        let text = snapshot_support::snapshot_buffer(terminal.backend().buffer());
        if width >= 60 {
            assert!(text.contains("F5/F6"), "{text}");
            let layout = fixture.app.prepare_frame(Rect::new(0, 0, width, 14));
            let area = layout
                .controls
                .iter()
                .find_map(|(target, area)| (*target == HitTarget::Delete).then_some(*area))
                .expect("delete control");
            fixture.pointer(area.x, area.y, PointerKind::Down(PointerButton::Left));
            assert!(fixture.app.state.board.live_thoughts().is_empty());
        }
    }
}

#[test]
fn configured_split_keeps_the_exact_editor_handoff_directly_and_after_escape() {
    for leave_editor in [false, true] {
        let mut fixture = configured(
            "schema_version=1\n[bindings.edit]\n\"thought.split\"=[{key='F5'}]\n[bindings.board]\n\"thought.split\"=[{key='F5'}]",
        );
        durable_thought(&mut fixture, "A界B");
        fixture.input(key(LogicalKey::Enter));
        fixture.input(key_input(UiKey::Move {
            movement: CursorMovement::DocumentStart,
            extend_selection: false,
        }));
        fixture.input(key_input(UiKey::Move {
            movement: CursorMovement::GraphemeForward,
            extend_selection: false,
        }));
        fixture.input(key_input(UiKey::Move {
            movement: CursorMovement::GraphemeForward,
            extend_selection: false,
        }));
        if leave_editor {
            fixture.input(key(LogicalKey::Escape));
        }
        let effects = fixture.effects(key(LogicalKey::Function(5)));
        assert!(
            matches!(effects.as_slice(), [Effect::CommitBoardOperation(operation)] if operation.kind == proqi::domain::BoardOperationKind::Split),
            "{effects:?}"
        );
        let contents = fixture
            .app
            .state
            .board
            .live_thoughts()
            .iter()
            .map(|thought| thought.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(contents, ["A界", "B"]);
    }
}

#[test]
fn insertion_row_footer_uses_its_own_independently_configured_map() {
    let mut fixture =
        configured("schema_version=1\n[bindings.insertion_boundary]\n\"thought.new\"=[{key='F5'}]");
    durable_thought(&mut fixture, "thought");
    fixture.input(key(LogicalKey::Down));
    assert!(fixture.app.insertion_focused());
    let terminal = draw(&mut fixture, 100, 14);
    let text = snapshot_support::snapshot_buffer(terminal.backend().buffer());
    assert!(text.contains("F5 New"), "{text}");
    assert!(!text.contains("n New"), "{text}");
    fixture.input(key(LogicalKey::Function(5)));
    assert_eq!(
        fixture.app.interaction_mode(),
        proqi::application::InteractionMode::Edit {
            thought_id: fixture.app.state.board.live_thoughts()[1].id
        }
    );
}

#[test]
fn unbound_modified_printable_and_removed_insertion_alias_do_not_fall_through() {
    let mut fixture = configured(
        "schema_version=1\n[bindings.board]\n\"thought.delete\"=[{key='d'}]\n[bindings.insertion_boundary]\n\"thought.new\"=[]",
    );
    durable_thought(&mut fixture, "keep");
    for modifiers in [
        LogicalModifiers::ALT,
        LogicalModifiers::CONTROL.union(LogicalModifiers::ALT),
    ] {
        assert!(
            fixture
                .effects(UiInput::KeyStroke(
                    KeyStroke::press(LogicalKey::Character('d')).with_modifiers(modifiers)
                ))
                .is_empty()
        );
        assert_eq!(fixture.app.state.board.live_thoughts().len(), 1);
    }
    fixture.input(key(LogicalKey::Down));
    assert!(fixture.effects(key(LogicalKey::Character('n'))).is_empty());
    assert_eq!(fixture.app.state.board.live_thoughts().len(), 1);
}

#[test]
fn directly_bound_editor_delete_flushes_pending_text_before_undo_snapshot() {
    let mut fixture =
        configured("schema_version=1\n[bindings.edit]\n\"thought.delete\"=[{key='F5'}]");
    durable_thought(&mut fixture, "old");
    fixture.input(key(LogicalKey::Enter));
    fixture.input(key(LogicalKey::Character('!')));
    let effects = fixture.effects(key(LogicalKey::Function(5)));
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::CommitBoardOperation(_)))
    );
    fixture.input(key(LogicalKey::Escape));
    fixture.input(key_input(UiKey::Undo));
    assert_eq!(fixture.app.state.board.live_thoughts()[0].content, "old!");
}

#[test]
fn bound_editor_duplicate_new_and_reorder_use_committed_latest_content() {
    for action in ["board.duplicate", "thought.new", "thought.move_up"] {
        let mut fixture = configured(&format!(
            "schema_version=1\n[bindings.edit]\n\"{action}\"=[{{key='F5'}}]"
        ));
        durable_thought(&mut fixture, "first");
        durable_thought(&mut fixture, "second");
        fixture.input(key(LogicalKey::Enter));
        fixture.input(key(LogicalKey::Character('!')));
        fixture.input(key(LogicalKey::Function(5)));
        let thoughts = fixture.app.state.board.live_thoughts();
        assert!(
            thoughts.iter().any(|thought| thought.content == "second!"),
            "{action}"
        );
        match action {
            "board.duplicate" => assert_eq!(
                thoughts
                    .iter()
                    .filter(|thought| thought.content == "second!")
                    .count(),
                2
            ),
            "thought.move_up" => assert_eq!(thoughts[0].content, "second!"),
            _ => assert!(fixture.app.editor_snapshot().is_some()),
        }
    }
}
