//! Typed compatibility policy for the thurbox contracts consumed by Proqi.

use crate::ports::agent::AgentError;

use super::contract::VersionDocument;

/// Oldest thurbox release whose session contract Proqi is qualified against.
///
/// `2.19.0` is the first release that reports `running`, `uncovered` and
/// `unreported` as states distinct from `idle`, and that publishes
/// `detected_agent` for a foreign-launched session. Both facts are required
/// before Proqi can describe a thurbox session truthfully.
const QUALIFIED_FROM: (u32, u32, u32) = (2, 19, 0);

/// One thurbox installation accepted by the complete compatibility policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CompatibleInstallation {
    version: String,
    schema_version: u32,
}

impl CompatibleInstallation {
    /// Return the exact installed version string.
    pub(super) fn version(&self) -> &str {
        &self.version
    }

    /// Return the integer thurbox publishes as its own contract number.
    pub(super) const fn schema_version(&self) -> u32 {
        self.schema_version
    }
}

/// The single compatibility owner for every thurbox consumer.
pub struct ThurboxCompatibilityPolicy;

impl ThurboxCompatibilityPolicy {
    /// Oldest qualified thurbox release, as `major.minor.patch`.
    #[must_use]
    pub const fn qualified_from() -> (u32, u32, u32) {
        QUALIFIED_FROM
    }

    /// Human-readable spelling of the oldest qualified thurbox release.
    #[must_use]
    pub fn qualified_from_display() -> String {
        let (major, minor, patch) = QUALIFIED_FROM;
        format!("{major}.{minor}.{patch}")
    }

    pub(super) fn negotiate(
        document: VersionDocument,
    ) -> Result<CompatibleInstallation, AgentError> {
        let Some(observed) = parse_version(&document.version) else {
            return Err(AgentError::Malformed(format!(
                "thurbox reported an unreadable version {}",
                document.version
            )));
        };
        if observed.0 != QUALIFIED_FROM.0 || observed < QUALIFIED_FROM {
            return Err(AgentError::Unsupported(format!(
                "Proqi supports thurbox {}.x from {}; this installation reports {}",
                QUALIFIED_FROM.0,
                Self::qualified_from_display(),
                document.version
            )));
        }
        Ok(CompatibleInstallation {
            version: document.version,
            schema_version: document.schema_version,
        })
    }
}

fn parse_version(value: &str) -> Option<(u32, u32, u32)> {
    let core = value
        .trim()
        .split(['-', '+'])
        .next()
        .filter(|core| !core.is_empty())?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    parts.next().is_none().then_some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(version: &str) -> VersionDocument {
        serde_json::from_value(serde_json::json!({
            "version": version,
            "schema_version": 45,
        }))
        .expect("version fixture")
    }

    #[test]
    fn the_qualified_release_and_every_later_patch_negotiate() {
        for version in ["2.19.0", "2.19.4", "2.20.0", "2.31.7"] {
            let accepted =
                ThurboxCompatibilityPolicy::negotiate(document(version)).expect("compatible");
            assert_eq!(accepted.version(), version);
            assert_eq!(accepted.schema_version(), 45);
        }
    }

    #[test]
    fn older_releases_and_a_different_major_are_refused() {
        for version in ["2.18.9", "2.0.0", "1.99.0", "3.0.0"] {
            let error =
                ThurboxCompatibilityPolicy::negotiate(document(version)).expect_err("refused");
            assert_eq!(
                error.stable_code(),
                crate::ports::agent::AgentFailureCode::Unsupported,
                "{version}"
            );
        }
    }

    #[test]
    fn an_unreadable_version_is_malformed_rather_than_assumed_compatible() {
        for version in ["", "two.nineteen.zero", "2", "2.19.0.1"] {
            let error =
                ThurboxCompatibilityPolicy::negotiate(document(version)).expect_err("refused");
            assert_eq!(
                error.stable_code(),
                crate::ports::agent::AgentFailureCode::Malformed,
                "{version}"
            );
        }
    }

    #[test]
    fn a_prerelease_suffix_negotiates_on_its_release_core() {
        let accepted =
            ThurboxCompatibilityPolicy::negotiate(document("2.20.0-rc.1")).expect("compatible");
        assert_eq!(accepted.version(), "2.20.0-rc.1");
    }
}
