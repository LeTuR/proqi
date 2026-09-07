//! Host rejection before any release artefact is fetched.
//!
//! Upstream builds one Linux target: `x86_64-unknown-linux-gnu`. Every other
//! Linux host must receive a specific explanation instead of a failed download
//! or an executable that cannot start.

use crate::fixture::{Installer, text};

fn rejection(configure: impl FnOnce(&mut Installer)) -> String {
    let mut installer = Installer::new();
    installer.publish("v9.9.9", "9.9.9");
    configure(&mut installer);
    let output = installer.run(&["--version", "v9.9.9"]);
    assert!(
        !output.status.success(),
        "unsupported host was accepted: {}",
        text(&output)
    );
    assert!(
        !installer.installed().exists(),
        "unsupported host was installed on"
    );
    text(&output)
}

#[test]
fn refuses_arm_linux_because_upstream_publishes_no_such_archive() {
    let report = rejection(|installer| {
        installer.set("FIXTURE_UNAME_MACHINE", "aarch64");
    });

    assert!(
        report.contains("aarch64"),
        "no architecture named: {report}"
    );
    assert!(
        report.contains("x86_64"),
        "no supported architecture named: {report}"
    );
    assert!(
        report.contains("cargo install"),
        "no source fallback offered: {report}"
    );
}

#[test]
fn refuses_a_32_bit_linux_host() {
    let report = rejection(|installer| {
        installer.set("FIXTURE_UNAME_MACHINE", "i686");
    });

    assert!(report.contains("i686"), "no architecture named: {report}");
}

#[test]
fn refuses_a_musl_linux_host_because_the_archive_links_against_glibc() {
    let report = rejection(|installer| {
        installer.set("FIXTURE_LDD_VERSION", "musl libc (x86_64)\nVersion 1.2.5");
    });

    assert!(report.contains("musl"), "no libc named: {report}");
    assert!(report.contains("glibc"), "no required libc named: {report}");
    assert!(
        report.contains("cargo install"),
        "no source fallback offered: {report}"
    );
}

#[test]
fn redirects_macos_to_the_supported_homebrew_formula() {
    let report = rejection(|installer| {
        installer.set("FIXTURE_UNAME_SYSTEM", "Darwin");
    });

    assert!(
        report.contains("Darwin"),
        "no operating system named: {report}"
    );
    assert!(
        report.contains("brew install oborchers/tap/proqi"),
        "no Homebrew guidance: {report}"
    );
}

#[test]
fn installs_when_the_libc_cannot_be_identified() {
    let mut installer = Installer::new();
    installer.publish("v9.9.9", "9.9.9");
    installer.set("FIXTURE_LDD_VERSION", "");

    let output = installer.run(&["--version", "v9.9.9"]);

    assert!(
        output.status.success(),
        "an unidentified libc blocked a working host: {}",
        text(&output)
    );
    assert!(installer.installed().is_file());
}
