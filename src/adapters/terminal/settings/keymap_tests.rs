//! Legacy translation and versioned keymap loading contracts.
use super::*;

#[test]
fn legacy_reorder_binding_names_migrate_to_shifted_range_keys() {
    let directory = tempfile::tempdir().expect("config directory");
    fs::write(
        directory.path().join("config.toml"),
        "[keybindings]\nmove_up = 'W'\nmove_down = 'G'\n",
    )
    .expect("write config");
    let settings = load_settings(directory.path()).expect("settings");
    assert_binding(
        &settings,
        crate::ui::ShortcutActionId::ExtendPrevious,
        'W',
        false,
    );
    assert_binding(
        &settings,
        crate::ui::ShortcutActionId::ExtendNext,
        'G',
        false,
    );
}

#[test]
fn range_selection_latch_binding_is_remappable() {
    let directory = tempfile::tempdir().expect("config directory");
    fs::write(
        directory.path().join("config.toml"),
        "[keybindings]\nrange_select = 'b'\n",
    )
    .expect("write config");
    let settings = load_settings(directory.path()).expect("settings");
    assert_binding(
        &settings,
        crate::ui::ShortcutActionId::RangeSelect,
        'b',
        false,
    );
}

#[test]
fn contextual_transform_binding_is_remappable() {
    let directory = tempfile::tempdir().expect("config directory");
    fs::write(
        directory.path().join("config.toml"),
        "[keybindings]\ntransform = 'g'\n",
    )
    .expect("write config");
    let settings = load_settings(directory.path()).expect("settings");
    assert_binding(
        &settings,
        crate::ui::ShortcutActionId::ContextualTransform,
        'g',
        false,
    );
}

#[test]
fn contextual_transform_rejects_reserved_primary_bindings() {
    let directory = tempfile::tempdir().expect("config directory");
    fs::write(
        directory.path().join("config.toml"),
        "[keybindings]\ntransform = 'x'\n",
    )
    .expect("write config");
    let error = load_settings(directory.path()).expect_err("reserved transform");
    assert!(error.to_string().contains("reserved Primary shortcut"));
}

#[test]
fn whole_board_selection_binding_is_remappable() {
    let directory = tempfile::tempdir().expect("config directory");
    fs::write(
        directory.path().join("config.toml"),
        "[keybindings]\nselect_all = 'z'\n",
    )
    .expect("write config");
    let settings = load_settings(directory.path()).expect("settings");
    assert_binding(
        &settings,
        crate::ui::ShortcutActionId::SelectAll,
        'z',
        false,
    );
}

#[test]
fn sentence_deletion_chord_is_remappable() {
    let directory = tempfile::tempdir().expect("config directory");
    fs::write(
        directory.path().join("config.toml"),
        "[keybindings]\ndelete_sentence = 'G'\n",
    )
    .expect("write config");
    let settings = load_settings(directory.path()).expect("settings");
    assert_binding(
        &settings,
        crate::ui::ShortcutActionId::DeleteSentence,
        'G',
        true,
    );
}

#[test]
fn sentence_deletion_rejects_unshifted_or_reserved_primary_suffixes() {
    for suffix in ['g', '1', 'A', 'Z', 'Ü'] {
        let directory = tempfile::tempdir().expect("config directory");
        fs::write(
            directory.path().join("config.toml"),
            format!("[keybindings]\ndelete_sentence = '{suffix}'\n"),
        )
        .expect("write invalid sentence binding");
        assert!(load_settings(directory.path()).is_err(), "suffix {suffix}");
    }
}

#[test]
fn visual_row_selection_fallbacks_are_remappable_and_validated() {
    let directory = tempfile::tempdir().expect("config directory");
    fs::write(
        directory.path().join("config.toml"),
        "[keybindings]\nselect_visual_row_start = 'G'\nselect_visual_row_end = 'R'\n",
    )
    .expect("write config");
    let settings = load_settings(directory.path()).expect("settings");
    assert_binding(
        &settings,
        crate::ui::ShortcutActionId::ExtendVisualRowStart,
        'G',
        true,
    );
    assert_binding(
        &settings,
        crate::ui::ShortcutActionId::ExtendVisualRowEnd,
        'R',
        true,
    );

    for suffix in ['g', '1', 'A', 'Z', 'Ü'] {
        let directory = tempfile::tempdir().expect("config directory");
        fs::write(
            directory.path().join("config.toml"),
            format!("[keybindings]\nselect_visual_row_end = '{suffix}'\n"),
        )
        .expect("write invalid visual-row binding");
        assert!(load_settings(directory.path()).is_err(), "suffix {suffix}");
    }
}

#[test]
fn quit_cannot_shadow_recovery_controls() {
    for key in ['r', 'w'] {
        let directory = tempfile::tempdir().expect("config directory");
        fs::write(
            directory.path().join("config.toml"),
            format!("[keybindings]\nquit = '{key}'\n"),
        )
        .expect("config");
        assert!(load_settings(directory.path()).is_err());
    }
}

#[test]
fn versioned_keymap_loads_once_and_legacy_mixing_is_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let config = "[keymap]\nschema_version=1\n[keymap.bindings.board]\n\"thought.new\"=[{key='z'}]";
    fs::write(directory.path().join("config.toml"), config).unwrap();
    let settings = load_settings(directory.path()).expect("versioned settings");
    assert_binding(&settings, crate::ui::ShortcutActionId::New, 'z', false);
    fs::write(
        directory.path().join("config.toml"),
        format!("[keybindings]\nnew='z'\n{config}"),
    )
    .unwrap();
    let error = load_settings(directory.path()).unwrap_err().to_string();
    assert!(error.contains("cannot be combined"), "{error}");
}

#[test]
fn malformed_keymaps_never_echo_configuration_content() {
    let directory = tempfile::tempdir().unwrap();
    for config in [
        "[keymap]\nschema_version=1\nprivate_secret='secret'",
        "[keymap]\nschema_version=1\n[keymap.bindings.private_secret]\n",
        "[keymap]\nschema_version=1\n[keymap.bindings.board]\nprivate_secret=[]",
        "[keymap]\nschema_version=1\n[keymap.bindings.board]\n\"thought.new\"=[{key='private_secret'}]",
        "private_secret='unterminated",
    ] {
        fs::write(directory.path().join("config.toml"), config).unwrap();
        let error = load_settings(directory.path()).unwrap_err().to_string();
        assert!(!error.contains("private_secret"), "{error}");
        assert!(
            !error.contains(directory.path().to_str().unwrap()),
            "{error}"
        );
    }
}
