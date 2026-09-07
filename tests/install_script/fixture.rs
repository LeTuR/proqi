//! Harness primitives for the Linux installer contract.
//!
//! Provides a local release mirror served over `file://`, a controllable
//! platform probe, and a bounded runner for `scripts/install.sh`. It owns no
//! installer policy and makes no correctness assertions.

use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt as _,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use sha2::{Digest as _, Sha256};
use tempfile::TempDir;

/// Archive stem published for the only supported Linux release target.
pub(crate) const PACKAGE: &str = "proqi-x86_64-unknown-linux-gnu";

const MARKER: &str = r#"{"schema_version":1,"product":"proqi","kind":"standalone_archive"}"#;

/// A sandboxed installer invocation with its own release mirror and probes.
pub(crate) struct Installer {
    sandbox: TempDir,
    environment: BTreeMap<String, String>,
}

impl Installer {
    /// Build a sandbox that reports a supported glibc x86-64 Linux host.
    pub(crate) fn new() -> Self {
        let sandbox = tempfile::Builder::new()
            .prefix("proqi-install-")
            .tempdir()
            .expect("create installer sandbox");
        for relative in ["releases", "stubs", "home", "temporary"] {
            fs::create_dir_all(sandbox.path().join(relative)).expect("create sandbox directory");
        }
        let mut installer = Self {
            sandbox,
            environment: BTreeMap::new(),
        };
        installer.write_stubs();
        let path = installer.path("stubs");
        installer.set("PATH", &format!("{}:{}", path.display(), inherited_path()));
        installer.set("HOME", &installer.path("home").display().to_string());
        installer.set("TMPDIR", &installer.path("temporary").display().to_string());
        installer.set(
            "PROQI_RELEASES_URL",
            &format!("file://{}", installer.path("releases").display()),
        );
        installer.set("FIXTURE_UNAME_SYSTEM", "Linux");
        installer.set("FIXTURE_UNAME_MACHINE", "x86_64");
        installer.set("FIXTURE_LDD_VERSION", "ldd (GNU libc) 2.39");
        installer
    }

    /// Override one environment variable seen by the script and its probes.
    pub(crate) fn set(&mut self, key: &str, value: &str) -> &mut Self {
        self.environment.insert(key.to_owned(), value.to_owned());
        self
    }

    /// Put `directory` first on the `PATH` the installer observes.
    pub(crate) fn prepend_path(&mut self, directory: &Path) -> &mut Self {
        let current = self.environment.get("PATH").cloned().unwrap_or_default();
        let value = format!("{}:{current}", directory.display());
        self.set("PATH", &value)
    }

    /// Publish a release whose executable reports `version` and exits zero.
    pub(crate) fn publish(&self, tag: &str, version: &str) -> &Self {
        self.publish_executable(
            tag,
            &format!("#!/bin/sh\n[ \"$1\" = --version ] && echo 'proqi {version}'\nexit 0\n"),
        )
    }

    /// Publish a release whose executable body is supplied verbatim.
    pub(crate) fn publish_executable(&self, tag: &str, body: &str) -> &Self {
        let stage = self.path("stage").join(tag).join(PACKAGE);
        fs::create_dir_all(&stage).expect("create release stage");
        write_program(&stage.join("proqi"), body);
        fs::write(stage.join("proqi-installation.json"), MARKER).expect("write install marker");
        fs::write(stage.join("LICENSE"), "MIT\n").expect("write license");
        let directory = self.path("releases").join("download").join(tag);
        fs::create_dir_all(&directory).expect("create release directory");
        let archive = directory.join(format!("{PACKAGE}.tar.gz"));
        let status = Command::new("tar")
            .args(["-czf".as_ref(), archive.as_os_str()])
            .arg("-C")
            .arg(self.path("stage").join(tag))
            .arg(PACKAGE)
            .status()
            .expect("run tar");
        assert!(status.success(), "tar failed for {tag}");
        self.write_checksum(tag, &fs::read(&archive).expect("read archive"))
    }

    /// Replace the published checksum with one that no archive can satisfy.
    pub(crate) fn corrupt_checksum(&self, tag: &str) -> &Self {
        self.write_checksum(tag, b"different bytes")
    }

    fn write_checksum(&self, tag: &str, bytes: &[u8]) -> &Self {
        let digest = Sha256::digest(bytes)
            .iter()
            .fold(String::new(), |mut text, byte| {
                use std::fmt::Write as _;
                let _ = write!(text, "{byte:02x}");
                text
            });
        let path = self
            .path("releases")
            .join("download")
            .join(tag)
            .join(format!("{PACKAGE}.tar.gz.sha256"));
        fs::write(path, format!("{digest}  {PACKAGE}.tar.gz\n")).expect("write checksum");
        self
    }

    /// Run the installer with `arguments` and capture its complete output.
    pub(crate) fn run(&self, arguments: &[&str]) -> Output {
        let mut command = Command::new("sh");
        command
            .arg("scripts/install.sh")
            .args(arguments)
            .env_clear()
            .envs(&self.environment)
            .current_dir(repository_root());
        command.output().expect("run installer")
    }

    /// Default installation prefix inside the sandbox home.
    pub(crate) fn prefix(&self) -> PathBuf {
        self.path("home").join(".local")
    }

    /// Path the installer is expected to own on a successful run.
    pub(crate) fn installed(&self) -> PathBuf {
        self.prefix().join("bin/proqi")
    }

    /// Directory the installer must leave empty on every exit path.
    pub(crate) fn temporary(&self) -> PathBuf {
        self.path("temporary")
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.sandbox.path().join(relative)
    }

    fn write_stubs(&self) {
        let stubs = self.path("stubs");
        write_program(
            &stubs.join("uname"),
            "#!/bin/sh\ncase \"$1\" in\n-m) echo \"$FIXTURE_UNAME_MACHINE\" ;;\n*) echo \"$FIXTURE_UNAME_SYSTEM\" ;;\nesac\n",
        );
        write_program(
            &stubs.join("ldd"),
            "#!/bin/sh\n[ -n \"${FIXTURE_LDD_VERSION:-}\" ] || exit 127\necho \"$FIXTURE_LDD_VERSION\"\n",
        );
    }
}

fn write_program(path: &Path, body: &str) {
    fs::write(path, body).expect("write program");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("mark program executable");
}

fn inherited_path() -> String {
    std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".to_owned())
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Combined standard output and standard error of a finished run.
pub(crate) fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Entries left behind in the installer's temporary directory.
pub(crate) fn residue(directory: &Path) -> Vec<PathBuf> {
    fs::read_dir(directory)
        .expect("read temporary directory")
        .map(|entry| entry.expect("read temporary entry").path())
        .collect()
}
