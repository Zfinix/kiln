//! Terminal restore on panic, early return, or normal exit.

use std::panic;

/// Restores the terminal on drop so no code path (early `?`, panic, normal
/// exit) leaves the shell in raw mode. `restore` undoes whatever the UI set
/// up, e.g. [`crate::terminal::restore_raw`] for an inline viewport.
pub struct TuiGuard {
    restore: fn(),
}

impl TuiGuard {
    /// Chain `restore` in front of the current panic hook. It runs again on drop.
    pub fn install(restore: fn()) -> Self {
        let original = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            restore();
            original(info);
        }));
        Self { restore }
    }
}

impl Drop for TuiGuard {
    fn drop(&mut self) {
        (self.restore)();
        // Drop our hook so a later panic can't emit stray restore sequences.
        // take_hook itself panics on a panicking thread, so skip it while
        // unwinding; the process is on its way out anyway.
        if !std::thread::panicking() {
            let _ = panic::take_hook();
        }
    }
}
