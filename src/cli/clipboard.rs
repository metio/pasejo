// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

#[cfg(target_os = "macos")]
use arboard::SetExtApple;
#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "android", target_os = "emscripten")),
))]
use arboard::SetExtLinux;
#[cfg(windows)]
use arboard::SetExtWindows;
use notify_rust::{Notification, Timeout};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

const POLL_TICK: Duration = Duration::from_millis(100);

/// Upper bound on how long we'll hold a secret in the clipboard. Absurd
/// `clipboard_timeout` values get clamped to this so the wait loop is
/// guaranteed to terminate (and `Instant + Duration` can't overflow).
const MAX_DEADLINE: Duration = Duration::from_hours(24 * 365);

/// Outcome of attempting to clear the clipboard at the end of a copy.
enum ClearOutcome {
    /// Secret was still in the clipboard and we removed it.
    Cleared,
    /// User copied something else; we left their clipboard untouched.
    Unchanged,
    /// Couldn't read the clipboard to compare, so we cleared defensively.
    /// May have wiped the user's new clipboard contents.
    ForciblyCleared,
}

/// RAII guard: clears the clipboard on drop, but only if it still contains
/// the secret we wrote. If the user has copied something else in the
/// meantime, we leave their new clipboard contents alone. The expected
/// value is held in `Zeroizing` so it's wiped from memory on drop.
struct ClipboardGuard {
    clipboard: arboard::Clipboard,
    expected: Zeroizing<String>,
    /// Set once `clear_if_unchanged` has been called. When true, `Drop`
    /// skips its fallback so we don't repeat the (potentially blocking)
    /// clipboard read on platforms like X11.
    cleared: bool,
}

impl ClipboardGuard {
    /// Performs the clear-if-unchanged logic and returns the outcome so the
    /// caller can react (e.g. tailor a notification, log a failure).
    fn clear_if_unchanged(&mut self) -> anyhow::Result<ClearOutcome> {
        // Mark first so Drop becomes a no-op even if an early `?` propagates.
        self.cleared = true;
        match self.clipboard.get().text() {
            Ok(current) if current.as_str() == self.expected.as_str() => {
                self.clipboard.clear()?;
                Ok(ClearOutcome::Cleared)
            }
            Ok(_) => Ok(ClearOutcome::Unchanged),
            Err(error) => {
                // Couldn't read the clipboard to compare — clear defensively
                // rather than risk leaving the secret behind.
                log::debug!("Failed to read clipboard for compare: {error}");
                self.clipboard.clear()?;
                Ok(ClearOutcome::ForciblyCleared)
            }
        }
    }
}

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        // Best-effort fallback for panic / early-return paths. The explicit
        // call site sets `cleared = true` so this becomes a no-op there.
        if self.cleared {
            return;
        }
        let _ = self.clear_if_unchanged();
    }
}

static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static HANDLER_INSTALLED: OnceLock<()> = OnceLock::new();

/// Installs a process-wide Ctrl-C handler on first call. `ctrlc::set_handler`
/// only allows one handler per process, so we install it lazily once and
/// reuse the same `INTERRUPTED` flag for every call. If installation fails,
/// the wait loop will only exit when the configured timeout elapses.
fn install_interrupt_handler() {
    HANDLER_INSTALLED.get_or_init(|| {
        if let Err(error) = ctrlc::set_handler(|| INTERRUPTED.store(true, Ordering::Relaxed)) {
            log::warn!(
                "Failed to install Ctrl-C handler: {error}. \
                 Clipboard will only clear after the configured timeout."
            );
        }
    });
}

pub fn copy_text_to_clipboard(text: &str, duration: Duration) -> anyhow::Result<()> {
    // Install the Ctrl-C handler before any secret enters the clipboard, so
    // SIGINT triggers our handler (which lets the loop exit and Drop run
    // clear()) instead of the default action that _exit()s without unwinding.
    install_interrupt_handler();
    // Reset the flag so a Ctrl-C from a prior invocation in the same process
    // doesn't make us skip the wait. The handler itself is `'static` and
    // remains installed across calls.
    INTERRUPTED.store(false, Ordering::Relaxed);

    let mut guard = ClipboardGuard {
        clipboard: arboard::Clipboard::new()?,
        expected: Zeroizing::new(text.to_owned()),
        cleared: false,
    };
    guard.clipboard.set().exclude_from_history().text(text)?;

    // Poll so we respond to Ctrl-C promptly, clamping each tick so we never
    // oversleep the requested duration. Clamping `duration` to `MAX_DEADLINE`
    // both guarantees loop termination and keeps `Instant + Duration` from
    // overflowing on absurd configured timeouts.
    let deadline = Instant::now() + duration.min(MAX_DEADLINE);
    loop {
        if INTERRUPTED.load(Ordering::Relaxed) {
            break;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        thread::sleep(remaining.min(POLL_TICK));
    }

    let cancelled = INTERRUPTED.load(Ordering::Relaxed);
    let outcome = guard.clear_if_unchanged();
    drop(guard); // ensures the secret is wiped from memory even on early-return paths

    let suffix = if cancelled { " (cancelled)" } else { "" };
    let (body, timeout) = match outcome {
        Ok(ClearOutcome::Cleared) => (format!("Clipboard cleared{suffix}"), Timeout::Default),
        Ok(ClearOutcome::Unchanged) => (
            format!("Clipboard left untouched (you copied something else){suffix}"),
            Timeout::Default,
        ),
        Ok(ClearOutcome::ForciblyCleared) => (
            format!("Clipboard cleared (couldn't verify contents){suffix}"),
            Timeout::Default,
        ),
        Err(error) => {
            // Critical path: the user's secret may still be in the clipboard.
            // Emit to stderr in addition to the notification, since a missing
            // notification daemon would otherwise leave the user uninformed.
            log::warn!("Failed to clear clipboard: {error}");
            log::error!(
                "Clipboard could not be cleared automatically — please clear it manually now."
            );
            (
                format!("Failed to clear clipboard! Please clear it manually.{suffix}"),
                Timeout::Never,
            )
        }
    };
    if let Err(error) = Notification::new()
        .summary("pasejo")
        .body(&body)
        .timeout(timeout)
        .show()
    {
        log::debug!("Failed to show clipboard-cleared notification: {error}");
    }
    Ok(())
}
