//! Shortcut configuration must fail before Proqi takes terminal ownership.

use std::fs;

use super::support::expect_command;

#[test]
fn invalid_shortcut_registry_is_rejected_before_alternate_screen_entry() {
    for (source, expected) in [
        (
            "[keybindings]\nnew = 'e'\n",
            "keybindings must be distinct printable characters",
        ),
        ("[keymap]\nschema_version=99\n", "version"),
        (
            "[keymap]\nschema_version=1\n[keymap.bindings.edit]\n\"clipboard.copy\"=[{key='ä'}]\n",
            "steal printable",
        ),
        (
            "[keymap]\nschema_version=1\n[keymap.bindings.board]\n\"context.close\"=[]\n",
            "Escape",
        ),
        ("[keybindings]\n[keymap]\nschema_version=1\n", "keybindings"),
    ] {
        assert_rejected(source, expected);
    }
}

fn assert_rejected(source: &str, expected: &str) {
    let state = tempfile::tempdir().expect("temporary state");
    let config = state.path().join("config");
    fs::create_dir(&config).expect("config directory");
    fs::write(config.join("config.toml"), source).expect("invalid shortcut config");
    let transcript = state.path().join("startup.transcript");
    let script = r"
        log_user 0
        set timeout 10
        spawn $env(PROQI_TEST_BINARY) --state-dir $env(PROQI_TEST_STATE)
        expect eof
        set transcript [open $env(PROQI_TEST_TRANSCRIPT) w]
        puts -nonewline $transcript $expect_out(buffer)
        close $transcript
        catch wait result
        exit [lindex $result 3]
    ";
    let status = expect_command()
        .args(["-c", script])
        .env("PROQI_TEST_BINARY", env!("CARGO_BIN_EXE_proqi"))
        .env("PROQI_TEST_STATE", state.path())
        .env("PROQI_TEST_TRANSCRIPT", &transcript)
        .status()
        .expect("run invalid-config startup in PTY");
    assert!(
        !status.success(),
        "invalid configuration unexpectedly launched"
    );

    let output = fs::read(&transcript).expect("startup transcript");
    assert!(
        !output
            .windows(b"\x1b[?1049h".len())
            .any(|bytes| bytes == b"\x1b[?1049h"),
        "terminal alternate screen was entered before configuration rejection"
    );
    let output = String::from_utf8_lossy(&output);
    assert!(output.contains(expected), "{output}");
}
