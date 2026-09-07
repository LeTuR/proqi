//! Archive integrity and released-executable verification.
//!
//! Every failure here must leave the host exactly as it was: no installed
//! executable, no partially written prefix, and no temporary residue.

use std::path::PathBuf;

use crate::fixture::{Installer, residue, text};

#[test]
fn refuses_an_archive_that_does_not_match_its_published_checksum() {
    let installer = Installer::new();
    installer.publish("v9.9.9", "9.9.9");
    installer.corrupt_checksum("v9.9.9");

    let output = installer.run(&["--version", "v9.9.9"]);

    let report = text(&output);
    assert!(
        !output.status.success(),
        "corrupt archive was accepted: {report}"
    );
    assert!(
        report.contains("checksum"),
        "no checksum diagnosis: {report}"
    );
    assert!(
        !installer.installed().exists(),
        "installed despite a checksum failure"
    );
}

#[test]
fn removes_its_temporary_directory_after_a_checksum_failure() {
    let installer = Installer::new();
    installer.publish("v9.9.9", "9.9.9");
    installer.corrupt_checksum("v9.9.9");

    assert!(!installer.run(&["--version", "v9.9.9"]).status.success());

    assert_eq!(residue(&installer.temporary()), Vec::<PathBuf>::new());
}

#[test]
fn refuses_a_release_that_is_not_published() {
    let installer = Installer::new();

    let output = installer.run(&["--version", "v9.9.9"]);

    let report = text(&output);
    assert!(
        !output.status.success(),
        "missing release was accepted: {report}"
    );
    assert!(
        report.contains("v9.9.9"),
        "no version in diagnosis: {report}"
    );
    assert!(!installer.installed().exists());
}

#[test]
fn refuses_an_executable_that_cannot_run_on_this_host() {
    let installer = Installer::new();
    installer.publish_executable("v9.9.9", "#!/bin/sh\nexit 1\n");

    let output = installer.run(&["--version", "v9.9.9"]);

    let report = text(&output);
    assert!(
        !output.status.success(),
        "unrunnable executable was accepted: {report}"
    );
    assert!(
        report.contains("glibc") || report.contains("GNU libc") || report.contains("2.39"),
        "no libc context in diagnosis: {report}"
    );
    assert!(!installer.installed().exists());
}

#[test]
fn refuses_an_executable_that_reports_another_version() {
    let installer = Installer::new();
    installer.publish("v9.9.9", "1.2.3");

    let output = installer.run(&["--version", "v9.9.9"]);

    let report = text(&output);
    assert!(
        !output.status.success(),
        "mismatched version was accepted: {report}"
    );
    assert!(
        report.contains("1.2.3"),
        "no observed version in diagnosis: {report}"
    );
    assert!(!installer.installed().exists());
}

#[test]
fn keeps_a_working_installation_when_a_later_run_fails() {
    let installer = Installer::new();
    installer.publish("v9.9.8", "9.9.8");
    installer.publish("v9.9.9", "9.9.9");
    assert!(installer.run(&["--version", "v9.9.8"]).status.success());
    installer.corrupt_checksum("v9.9.9");

    assert!(!installer.run(&["--version", "v9.9.9"]).status.success());

    let output = std::process::Command::new(installer.installed())
        .arg("--version")
        .output()
        .expect("run the previously installed executable");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "proqi 9.9.8"
    );
}
