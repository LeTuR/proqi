//! Process fixture for the structured thurbox version, session, and send CLI.

use std::{ffi::OsString, path::Path};

pub struct ThurboxFixture {
    _temporary: tempfile::TempDir,
    program: std::path::PathBuf,
}

impl ThurboxFixture {
    pub fn new(version: &str) -> Self {
        let temporary = tempfile::tempdir().expect("thurbox fixture directory");
        let program = temporary.path().join("thurbox-cli");
        let send_log = temporary.path().join("send.bin");
        std::fs::write(&program, fixture_script(version, &send_log))
            .expect("thurbox fixture executable");
        make_executable(&program);
        Self {
            _temporary: temporary,
            program,
        }
    }

    pub fn program(&self) -> OsString {
        OsString::from(self.program.as_os_str())
    }

    pub fn sent_bytes(&self) -> Option<Vec<u8>> {
        std::fs::read(self.program.with_file_name("send.bin")).ok()
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt as _;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .expect("thurbox fixture permissions");
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}

fn fixture_script(version: &str, send_log: &Path) -> String {
    // thurbox writes its failure object to standard output with an empty
    // standard error and a non-zero exit status, which the else branch keeps
    // exercising verbatim.
    let template = r#"#!/bin/sh
if [ "$1 $2" = "version --json" ]; then
  printf '%s\n' '{"data_dir":"/tmp/thurbox","schema_version":45,"tmux_socket":"thurbox","version":"__VERSION__","future_version_field":true}'
elif [ "$1 $2 $3 $4" = "session list --json --verify" ]; then
  printf '%s\n' '[{"id":"11111111-1111-4111-8111-111111111111","name":"fixture","cwd":"/tmp/fixture","agent":"claude","reports_as":null,"detected_agent":null,"agent_session_id":"conversation-one","state":"idle","state_source":"hook","hook_coverage":"full","stopped":false,"future_session_field":true},{"id":"22222222-2222-4222-8222-222222222222","name":"crewmate","cwd":"/tmp/crewmate","agent":"zsh","reports_as":null,"detected_agent":"codex","agent_session_id":"conversation-two","state":"running","state_source":"process","hook_coverage":"none","stopped":false},{"id":"33333333-3333-4333-8333-333333333333","name":"plain shell","cwd":"/tmp/shell","agent":"zsh","reports_as":null,"detected_agent":null,"agent_session_id":"conversation-three","state":"uncovered","state_source":null,"hook_coverage":"none","stopped":false}]'
elif [ "$1 $2 $3 $4" = "session send --json 22222222-2222-4222-8222-222222222222" ]; then
  printf '%s' "$5" > "__SEND_LOG__"
  printf '%s\n' '{"sent":true,"submitted":true,"session_id":"22222222-2222-4222-8222-222222222222","session_name":"crewmate"}'
else
  printf '%s\n' '{"error":"unexpected command","suggestion":"the fixture only answers the verified contract"}'
  exit 1
fi
"#;
    template
        .replace("__VERSION__", version)
        .replace("__SEND_LOG__", &send_log.display().to_string())
}
