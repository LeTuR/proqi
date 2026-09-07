//! Linux one-line installer contract.
//!
//! This suite owns the successful installation and upgrade contract of
//! `scripts/install.sh`. Narrower failure boundaries are registered as
//! satellite modules: archive integrity and released-executable verification
//! in `integrity`, and host rejection in `platform`.

#![cfg(unix)]

#[path = "install_script/fixture.rs"]
mod fixture;

#[path = "install_script/integrity.rs"]
mod integrity;
#[path = "install_script/platform.rs"]
mod platform;

use std::{fs, os::unix::fs::PermissionsExt as _};

use fixture::{Installer, residue, text};

#[test]
fn installs_a_pinned_release_into_a_user_writable_prefix() {
    let installer = Installer::new();
    installer.publish("v9.9.9", "9.9.9");

    let output = installer.run(&["--version", "v9.9.9"]);

    let report = text(&output);
    assert!(output.status.success(), "installer failed: {report}");
    let executable = installer.installed();
    assert!(executable.is_file(), "missing executable: {report}");
    let mode = fs::metadata(&executable)
        .expect("inspect executable")
        .permissions()
        .mode();
    assert_eq!(mode & 0o111, 0o111, "executable is not runnable: {mode:o}");
    assert!(
        report.contains("9.9.9"),
        "no installed version reported: {report}"
    );
    assert!(
        report.contains(&executable.display().to_string()),
        "no installed path reported: {report}"
    );
}

#[test]
fn installs_the_marker_that_identifies_a_standalone_archive() {
    let installer = Installer::new();
    installer.publish("v9.9.9", "9.9.9");

    let output = installer.run(&["--version", "v9.9.9"]);

    assert!(
        output.status.success(),
        "installer failed: {}",
        text(&output)
    );
    let marker = installer.prefix().join("bin/proqi-installation.json");
    let contents = fs::read_to_string(marker).expect("read installation marker");
    assert!(
        contents.contains("\"kind\":\"standalone_archive\""),
        "unexpected marker: {contents}"
    );
}

#[test]
fn accepts_a_release_version_without_its_tag_prefix() {
    let installer = Installer::new();
    installer.publish("v9.9.9", "9.9.9");

    let output = installer.run(&["--version", "9.9.9"]);

    assert!(
        output.status.success(),
        "installer failed: {}",
        text(&output)
    );
    assert!(installer.installed().is_file());
}

#[test]
fn rerunning_replaces_an_older_installation_and_reports_the_change() {
    let installer = Installer::new();
    installer.publish("v9.9.8", "9.9.8");
    installer.publish("v9.9.9", "9.9.9");
    assert!(installer.run(&["--version", "v9.9.8"]).status.success());

    let output = installer.run(&["--version", "v9.9.9"]);

    let report = text(&output);
    assert!(output.status.success(), "upgrade failed: {report}");
    assert!(
        report.contains("9.9.8"),
        "no replaced version reported: {report}"
    );
    assert!(
        report.contains("9.9.9"),
        "no installed version reported: {report}"
    );
}

#[test]
fn honours_an_explicit_prefix_outside_the_home_directory() {
    let installer = Installer::new();
    installer.publish("v9.9.9", "9.9.9");
    let prefix = installer.temporary().join("opt");

    let output = installer.run(&[
        "--version",
        "v9.9.9",
        "--prefix",
        &prefix.display().to_string(),
    ]);

    assert!(
        output.status.success(),
        "installer failed: {}",
        text(&output)
    );
    assert!(prefix.join("bin/proqi").is_file());
    assert!(
        !installer.installed().exists(),
        "default prefix was also used"
    );
}

#[test]
fn explains_how_to_reach_an_installation_outside_path() {
    let installer = Installer::new();
    installer.publish("v9.9.9", "9.9.9");

    let output = installer.run(&["--version", "v9.9.9"]);

    let report = text(&output);
    let bin = installer.prefix().join("bin");
    assert!(
        report.contains("PATH") && report.contains(&bin.display().to_string()),
        "no actionable PATH guidance: {report}"
    );
}

#[test]
fn stays_quiet_about_path_when_the_directory_is_already_reachable() {
    let mut installer = Installer::new();
    installer.publish("v9.9.9", "9.9.9");
    let bin = installer.prefix().join("bin");
    fs::create_dir_all(&bin).expect("create bin directory");
    installer.prepend_path(&bin);

    let output = installer.run(&["--version", "v9.9.9"]);

    let report = text(&output);
    assert!(output.status.success(), "installer failed: {report}");
    assert!(
        !report.contains("PATH"),
        "unnecessary PATH guidance: {report}"
    );
}

#[test]
fn removes_its_temporary_directory_after_a_successful_run() {
    let installer = Installer::new();
    installer.publish("v9.9.9", "9.9.9");

    assert!(installer.run(&["--version", "v9.9.9"]).status.success());

    assert_eq!(
        residue(&installer.temporary()),
        Vec::<std::path::PathBuf>::new()
    );
}

#[test]
fn asks_for_a_prefix_when_no_home_directory_is_known() {
    let mut installer = Installer::new();
    installer.publish("v9.9.9", "9.9.9");
    installer.set("HOME", "");

    let output = installer.run(&["--version", "v9.9.9"]);

    let report = text(&output);
    assert!(
        !output.status.success(),
        "installed without a prefix: {report}"
    );
    assert!(
        report.contains("--prefix"),
        "no actionable guidance: {report}"
    );
}

#[test]
fn describes_its_options_without_installing_anything() {
    let installer = Installer::new();

    let output = installer.run(&["--help"]);

    let report = text(&output);
    assert!(output.status.success(), "help failed: {report}");
    for option in ["--prefix", "--version", "PROQI_PREFIX", "PROQI_VERSION"] {
        assert!(report.contains(option), "help omits {option}: {report}");
    }
    assert!(!installer.installed().exists());
}
