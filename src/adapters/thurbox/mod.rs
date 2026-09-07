//! Optional, fail-closed thurbox session prompt adapter.

mod compatibility;
mod contract;
mod discovery;
mod submission;
#[cfg(test)]
mod tests;

use std::{ffi::OsString, time::Duration};

use serde::de::DeserializeOwned;

use crate::ports::{
    agent::{
        AgentCapabilities, AgentError, AgentGateway, AgentTarget, PaneContext, SubmissionReceipt,
        SubmissionRequest,
    },
    environment::{ProcessError, ProcessRequest, ProcessRunner},
};

use compatibility::CompatibleInstallation;
use contract::{ErrorDocument, VersionDocument};

pub use compatibility::ThurboxCompatibilityPolicy;

/// Stable provider name recorded on every thurbox target and receipt.
pub(crate) const PROVIDER: &str = "thurbox";

/// Session listing probes each pane, so discovery is allowed the same bound as
/// submission rather than the shorter read-only budget Herdr uses.
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(5);
const SUBMISSION_TIMEOUT: Duration = Duration::from_secs(5);

/// thurbox gateway using direct, bounded child-process calls.
pub struct ThurboxGateway<R> {
    runner: R,
    program: OsString,
    enabled: bool,
    current_session: Option<String>,
}

impl<R> ThurboxGateway<R> {
    /// Construct an injectable gateway for composition or contract tests.
    #[must_use]
    pub fn new(program: OsString, runner: R, enabled: bool) -> Self {
        Self {
            runner,
            program,
            enabled,
            current_session: None,
        }
    }

    /// Exclude the thurbox session hosting this Proqi instance from discovery.
    #[must_use]
    pub fn with_current_session(mut self, session: Option<String>) -> Self {
        self.current_session = session.filter(|value| !value.trim().is_empty());
        self
    }

    fn current_session(&self) -> Option<&str> {
        self.current_session.as_deref()
    }
}

impl ThurboxGateway<crate::adapters::process::SystemProcessRunner> {
    /// Compose the installed thurbox CLI and the inherited session identity.
    #[must_use]
    pub fn from_environment() -> Self {
        Self::from_environment_with_runner(crate::adapters::process::SystemProcessRunner::default())
    }

    pub(crate) fn from_environment_with_runner(
        runner: crate::adapters::process::SystemProcessRunner,
    ) -> Self {
        let environment = ThurboxEnvironment::detect();
        Self::new(
            OsString::from("thurbox-cli"),
            runner,
            environment.integration_enabled(),
        )
        .with_current_session(environment.current_session)
    }
}

/// Whether the thurbox integration is enabled, and which session hosts Proqi.
struct ThurboxEnvironment {
    disabled: bool,
    current_session: Option<String>,
}

impl ThurboxEnvironment {
    fn detect() -> Self {
        Self {
            disabled: std::env::var_os("PROQI_DISABLE_THURBOX").is_some(),
            current_session: std::env::var("THURBOX_SESSION").ok(),
        }
    }

    const fn integration_enabled(&self) -> bool {
        !self.disabled
    }
}

impl<R: ProcessRunner> ThurboxGateway<R> {
    /// Negotiate the installed thurbox release for one operation.
    ///
    /// Every operation renegotiates, so an upgrade between discovery and
    /// delivery changes the published contract number and the submission
    /// revalidation fails closed instead of trusting a cached answer.
    fn installation(&mut self) -> Result<CompatibleInstallation, AgentError> {
        if !self.enabled {
            return Err(AgentError::Unavailable(
                "the thurbox integration is disabled for this process".to_owned(),
            ));
        }
        let document: VersionDocument = self
            .json(&["version", "--json"], DISCOVERY_TIMEOUT)
            .map_err(unreachable_cli)?;
        ThurboxCompatibilityPolicy::negotiate(document)
    }

    /// Run one bounded thurbox command and decode its JSON body.
    ///
    /// thurbox writes its failure object to standard output with an empty
    /// standard error, so the exit status is captured before anything is
    /// parsed. Parsing first would read a failure as a successful response.
    fn json<T: DeserializeOwned>(
        &mut self,
        args: &[&str],
        timeout: Duration,
    ) -> Result<T, AgentError> {
        let output = self
            .runner
            .run(ProcessRequest {
                program: self.program.clone(),
                args: args.iter().map(OsString::from).collect(),
                stdin: None,
                timeout,
            })
            .map_err(process_error)?;
        if output.exit_code != Some(0) {
            return Err(command_error(&output.stdout, &output.stderr));
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|error| AgentError::Malformed(error.to_string()))
    }
}

impl<R: ProcessRunner> AgentGateway for ThurboxGateway<R> {
    fn capabilities(&mut self) -> Result<AgentCapabilities, AgentError> {
        discovery::capabilities(self)
    }

    fn adjacent_targets(&mut self, _context: &PaneContext) -> Result<Vec<AgentTarget>, AgentError> {
        Err(AgentError::Unsupported(
            "thurbox does not publish pane adjacency".to_owned(),
        ))
    }

    fn global_targets(&mut self) -> Result<Vec<AgentTarget>, AgentError> {
        discovery::targets(self)
    }

    fn submit(&mut self, request: SubmissionRequest) -> Result<SubmissionReceipt, AgentError> {
        submission::submit(self, &request)
    }
}

/// A version probe that cannot run at all means thurbox is not installed here.
fn unreachable_cli(error: AgentError) -> AgentError {
    match error {
        AgentError::Process(detail) => {
            AgentError::Unavailable(format!("thurbox-cli is not available: {detail}"))
        }
        other => other,
    }
}

fn process_error(error: ProcessError) -> AgentError {
    match error {
        ProcessError::TimedOut => AgentError::TimedOut,
        ProcessError::Cancelled => AgentError::Process("process cancelled".to_owned()),
        ProcessError::Io(message) => AgentError::Process(message),
        ProcessError::OutputLimit => AgentError::Malformed("provider output exceeded limit".into()),
    }
}

fn command_error(stdout: &[u8], stderr: &[u8]) -> AgentError {
    serde_json::from_slice::<ErrorDocument>(stdout).map_or_else(
        |_| {
            let detail = if stderr.is_empty() { stdout } else { stderr };
            AgentError::Process(String::from_utf8_lossy(detail).trim().to_owned())
        },
        |document| AgentError::Rejected {
            code: "thurbox_error".to_owned(),
            message: document.error,
        },
    )
}
