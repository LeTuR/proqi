//! Capture owns raw mode and keyboard reporting until every exit path restores them.
use crate::adapters::terminal::control::{compatible_keyboard_flags, reset_keyboard_reporting};
use crossterm::{
    event::{PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::{self, stdout};

pub(super) trait CaptureTerminal {
    fn raw_on(&mut self) -> io::Result<()>;
    fn push(&mut self) -> io::Result<()>;
    fn pop(&mut self) -> io::Result<()>;
    fn reset(&mut self) -> io::Result<()>;
    fn raw_off(&mut self) -> io::Result<()>;
}

pub(super) struct SystemTerminal;
impl CaptureTerminal for SystemTerminal {
    fn raw_on(&mut self) -> io::Result<()> {
        enable_raw_mode()
    }
    fn push(&mut self) -> io::Result<()> {
        execute!(
            stdout(),
            PushKeyboardEnhancementFlags(compatible_keyboard_flags())
        )
    }
    fn pop(&mut self) -> io::Result<()> {
        execute!(stdout(), PopKeyboardEnhancementFlags)
    }
    fn reset(&mut self) -> io::Result<()> {
        reset_keyboard_reporting()
    }
    fn raw_off(&mut self) -> io::Result<()> {
        disable_raw_mode()
    }
}

pub(super) struct RawInputGuard<T: CaptureTerminal> {
    terminal: T,
    active: bool,
}

impl<T: CaptureTerminal> RawInputGuard<T> {
    pub(super) fn enter(mut terminal: T) -> io::Result<Self> {
        terminal.raw_on()?;
        let mut guard = Self {
            terminal,
            active: true,
        };
        // The guard already owns restoration if reporting setup errors or panics.
        guard.terminal.push()?;
        Ok(guard)
    }

    pub(super) fn finish(mut self) -> io::Result<()> {
        self.restore()
    }

    fn restore(&mut self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }
        self.active = false;
        let keyboard = self.terminal.pop();
        let reset = self.terminal.reset();
        let raw = self.terminal.raw_off();
        keyboard.and(reset).and(raw)
    }
}

impl<T: CaptureTerminal> Drop for RawInputGuard<T> {
    fn drop(&mut self) {
        let _restored = self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    struct Terminal {
        calls: Rc<RefCell<Vec<&'static str>>>,
        failure: Option<&'static str>,
        panic_push: bool,
    }
    impl Terminal {
        fn call(&self, name: &'static str) -> io::Result<()> {
            self.calls.borrow_mut().push(name);
            if self.failure == Some(name) {
                Err(io::Error::other("injected"))
            } else {
                Ok(())
            }
        }
    }
    impl CaptureTerminal for Terminal {
        fn raw_on(&mut self) -> io::Result<()> {
            self.call("raw_on")
        }
        fn push(&mut self) -> io::Result<()> {
            let result = self.call("push");
            assert!(!self.panic_push, "injected push panic");
            result
        }
        fn pop(&mut self) -> io::Result<()> {
            self.call("pop")
        }
        fn reset(&mut self) -> io::Result<()> {
            self.call("reset")
        }
        fn raw_off(&mut self) -> io::Result<()> {
            self.call("raw_off")
        }
    }

    #[test]
    fn every_exit_path_restores_once_even_after_setup_or_output_failure() {
        for failure in [
            None,
            Some("push"),
            Some("pop"),
            Some("reset"),
            Some("raw_off"),
        ] {
            for panic in [false, true] {
                verify_restoration(failure, panic);
            }
        }
    }

    fn verify_restoration(failure: Option<&'static str>, panic: bool) {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let terminal = Terminal {
            calls: Rc::clone(&calls),
            failure,
            panic_push: false,
        };
        let _outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let guard = RawInputGuard::enter(terminal)?;
            assert!(!panic, "injected capture panic");
            guard.finish()
        }));
        assert_eq!(
            *calls.borrow(),
            ["raw_on", "push", "pop", "reset", "raw_off"]
        );
    }

    #[test]
    fn setup_panic_is_owned_before_keyboard_reporting_begins() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let terminal = Terminal {
            calls: Rc::clone(&calls),
            failure: None,
            panic_push: true,
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            RawInputGuard::enter(terminal)
        }));
        assert!(result.is_err());
        assert_eq!(
            *calls.borrow(),
            ["raw_on", "push", "pop", "reset", "raw_off"]
        );
    }
}
