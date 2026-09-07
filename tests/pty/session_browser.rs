//! Session-browser search, selection, and resume through a real PTY.

use super::support::{expect_command, json_command};

#[test]
fn session_browser_searches_and_resumes_in_a_real_pty() {
    let state = tempfile::tempdir().expect("temporary state");
    let binary = env!("CARGO_BIN_EXE_proqi");
    let first = json_command(binary, state.path(), &[]);
    let first_id = first["data"]["session_id"].as_str().expect("first ID");
    let _renamed = json_command(
        binary,
        state.path(),
        &["sessions", "rename", first_id, "Other session"],
    );
    let target = json_command(binary, state.path(), &[]);
    let target_id = target["data"]["session_id"].as_str().expect("target ID");
    let _renamed = json_command(
        binary,
        state.path(),
        &["sessions", "rename", target_id, "Needle target"],
    );
    let before = json_command(binary, state.path(), &["sessions", "list"]);
    let opened_before = before["data"]["sessions"]
        .as_array()
        .expect("sessions")
        .iter()
        .find(|session| session["id"] == target_id)
        .and_then(|session| session["last_opened_at"].as_i64())
        .expect("opening timestamp");

    let browse = r#"
        log_user 0
        set timeout 10
        set binary $env(PROQI_TEST_BINARY)
        set state $env(PROQI_TEST_STATE)
        spawn $binary --state-dir $state -r
        stty rows 24 columns 100
        expect -exact "\x1b\[?1049h"
        send -- "Needle"
        expect -re "Needle"
        send "\r"
        expect -exact "\x1b\[?1049l"; expect -exact "\x1b\[?1049h"
        send -- $env(PROQI_TEST_PRIMARY_Q)
        expect {
            eof {}
            timeout { exit 93 }
        }
        catch wait result
        exit [lindex $result 3]
    "#;
    let status = expect_command()
        .args(["-c", browse])
        .env("PROQI_TEST_BINARY", binary)
        .env("PROQI_TEST_STATE", state.path())
        .status()
        .expect("run PTY browser workflow");
    assert!(status.success(), "browser PTY exited with {status}");

    let sessions = json_command(binary, state.path(), &["sessions", "list"]);
    let selected = sessions["data"]["sessions"]
        .as_array()
        .expect("sessions")
        .iter()
        .find(|session| session["id"] == target_id)
        .expect("resumed target");
    assert!(
        selected["last_opened_at"]
            .as_i64()
            .is_some_and(|opened_after| opened_after > opened_before)
    );
}

#[test]
fn session_browser_mouse_save_persists_unicode_name_in_a_real_pty() {
    let state = tempfile::tempdir().expect("temporary state");
    let binary = env!("CARGO_BIN_EXE_proqi");
    let created = json_command(binary, state.path(), &[]);
    let id = created["data"]["session_id"].as_str().expect("session ID");
    let browse = r#"
        log_user 0
        set timeout 10
        set binary $env(PROQI_TEST_BINARY)
        set state $env(PROQI_TEST_STATE)
        spawn -noecho sh -c {stty rows 24 columns 80; exec "$PROQI_TEST_BINARY" --state-dir "$PROQI_TEST_STATE" -r}
        expect {
            -exact "Rename" {}
            timeout { exit 91 }
        }
        send -- "\x1bOQ"
        expect {
            -exact "Save" {}
            timeout { exit 92 }
        }
        send -- "\x1b\[200~  Grüße 界 e\u0301  \x1b\[201~"
        expect {
            -exact "Grüße" {}
            timeout { exit 93 }
        }
        send -- "\x1b\[<0;9;24M\x1b\[<0;9;24m"
        expect {
            -exact "\x1b\[?1049l" {}
            timeout { exit 94 }
        }
        expect {
            -exact "\x1b\[?1049h" {}
            timeout { exit 95 }
        }
        send -- "\x1b"
        expect {
            eof {}
            timeout { exit 96 }
        }
        catch wait result
        exit [lindex $result 3]
    "#;
    let status = expect_command()
        .args(["-c", browse])
        .env("PROQI_TEST_BINARY", binary)
        .env("PROQI_TEST_STATE", state.path())
        .status()
        .expect("run PTY browser mouse save");
    assert!(
        status.success(),
        "browser mouse save PTY exited with {status}"
    );
    let sessions = json_command(binary, state.path(), &["sessions", "list"]);
    let renamed = sessions["data"]["sessions"]
        .as_array()
        .expect("sessions")
        .iter()
        .find(|session| session["id"] == id)
        .expect("renamed session");
    assert_eq!(renamed["name"], "Grüße 界 e\u{301}");
}
