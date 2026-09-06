//! Bounded key capture using the application decoder and panic-safe terminal guard.
use super::{CrosstermEventSource, EventSource, translation::decode_key};
use crate::{
    adapters::terminal::{TerminalError, control::TerminationGuard},
    ui::{KeyStroke, ShortcutContextStack, ShortcutRegistry},
};
use crossterm::event::Event;
mod guard;
use std::{
    io,
    time::{Duration, Instant},
};

pub(crate) struct KeyInspection {
    pub(crate) event: Option<serde_json::Value>,
    pub(crate) cancelled: bool,
}

#[derive(Debug, Eq, PartialEq)]
enum CaptureOutcome {
    Event(KeyStroke),
    TimedOut,
    Cancelled,
}

pub(crate) fn inspect_keypress(
    registry: &ShortcutRegistry,
    contexts: &ShortcutContextStack,
    timeout: Duration,
) -> Result<KeyInspection, TerminalError> {
    let termination = TerminationGuard::register()?;
    let guard = guard::RawInputGuard::enter(guard::SystemTerminal)?;
    eprintln!(
        "Press one key to inspect (Escape cancels). Capture ends after {} ms.",
        timeout.as_millis()
    );
    let start = Instant::now();
    let outcome = capture(
        &mut CrosstermEventSource,
        timeout,
        || start.elapsed(),
        || termination.requested(),
    )?;
    guard.finish()?;
    Ok(KeyInspection {
        event: match outcome {
            CaptureOutcome::Event(stroke) => Some(registry.inspect(contexts, stroke)),
            _ => None,
        },
        cancelled: outcome == CaptureOutcome::Cancelled,
    })
}

fn capture(
    source: &mut impl EventSource,
    timeout: Duration,
    mut elapsed: impl FnMut() -> Duration,
    mut cancelled: impl FnMut() -> bool,
) -> io::Result<CaptureOutcome> {
    loop {
        if cancelled() {
            return Ok(CaptureOutcome::Cancelled);
        }
        let remaining = timeout.saturating_sub(elapsed());
        if remaining.is_zero() {
            return Ok(CaptureOutcome::TimedOut);
        }
        if source.poll(remaining.min(Duration::from_millis(40)))?
            && let Event::Key(key) = source.read()?
        {
            return Ok(CaptureOutcome::Event(decode_key(key)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    struct Source {
        event: Option<Event>,
        fail: bool,
    }
    impl EventSource for Source {
        fn poll(&mut self, _: Duration) -> io::Result<bool> {
            if self.fail {
                return Err(io::Error::other("injected"));
            }
            Ok(self.event.is_some())
        }
        fn read(&mut self) -> io::Result<Event> {
            self.event.take().ok_or_else(|| io::Error::other("empty"))
        }
    }

    #[test]
    fn capture_is_bounded_without_an_event_or_after_non_key_input() {
        for event in [
            None,
            Some(Event::Paste("private text".into())),
            Some(Event::Resize(80, 24)),
        ] {
            let mut source = Source { event, fail: false };
            let mut elapsed = Duration::ZERO;
            let result = capture(
                &mut source,
                Duration::from_millis(100),
                || {
                    elapsed += Duration::from_millis(20);
                    elapsed
                },
                || false,
            )
            .unwrap();
            assert_eq!(result, CaptureOutcome::TimedOut);
        }
    }

    #[test]
    fn capture_preserves_event_and_propagates_failure() {
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let mut source = Source {
            event: Some(Event::Key(key)),
            fail: false,
        };
        assert_eq!(
            capture(
                &mut source,
                Duration::from_secs(1),
                || Duration::ZERO,
                || false
            )
            .unwrap(),
            CaptureOutcome::Event(decode_key(key))
        );
        source.fail = true;
        assert!(
            capture(
                &mut source,
                Duration::from_secs(1),
                || Duration::ZERO,
                || false
            )
            .is_err()
        );
    }
}
