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

/// RAII guard: clears the clipboard on drop, but only if it still contains
/// the secret we wrote. If the user has copied something else in the
/// meantime, we leave their new clipboard contents alone. The expected
/// value is held in `Zeroizing` so it's wiped from memory on drop.
struct ClipboardGuard {
    clipboard: arboard::Clipboard,
    expected: Zeroizing<String>,
}

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        match self.clipboard.get().text() {
            Ok(current) if current.as_str() == self.expected.as_str() => {
                let _ = self.clipboard.clear();
            }
            Ok(_) => {
                // User copied something else; leave their clipboard alone.
            }
            Err(error) => {
                // Couldn't read the clipboard to compare — clear defensively
                // rather than risk leaving the secret behind.
                log::debug!("Failed to read clipboard for compare: {error}");
                let _ = self.clipboard.clear();
            }
        }
    }
}

static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static HANDLER_OK: OnceLock<bool> = OnceLock::new();

/// Installs a process-wide Ctrl-C handler on first call. `ctrlc::set_handler`
/// only allows one handler per process, so we install it lazily once and
/// reuse the same `INTERRUPTED` flag for every call. Returns `true` when
/// the handler is in place; `false` means Ctrl-C will not break the wait
/// loop, and the caller has already been warned.
fn install_interrupt_handler() -> bool {
    *HANDLER_OK.get_or_init(|| {
        match ctrlc::set_handler(|| INTERRUPTED.store(true, Ordering::Relaxed)) {
            Ok(()) => true,
            Err(error) => {
                log::warn!(
                    "Failed to install Ctrl-C handler: {error}. \
                     Clipboard will only clear after the configured timeout."
                );
                false
            }
        }
    })
}

pub fn copy_text_to_clipboard(text: &str, duration: Duration) -> anyhow::Result<()> {
    // Install the Ctrl-C handler before any secret enters the clipboard, so
    // SIGINT triggers our handler (which lets the loop exit and Drop run
    // clear()) instead of the default action that _exit()s without unwinding.
    // INTERRUPTED starts `false` from the static initializer; we don't reset
    // it here because pasejo only calls this function once per process.
    install_interrupt_handler();

    let mut guard = ClipboardGuard {
        clipboard: arboard::Clipboard::new()?,
        expected: Zeroizing::new(text.to_owned()),
    };
    guard.clipboard.set().exclude_from_history().text(text)?;

    // Poll so we respond to Ctrl-C promptly, clamping each tick so we never
    // oversleep the requested duration.
    let deadline = Instant::now() + duration;
    while !INTERRUPTED.load(Ordering::Relaxed) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        thread::sleep(remaining.min(POLL_TICK));
    }

    let cancelled = INTERRUPTED.load(Ordering::Relaxed);
    drop(guard); // clear clipboard before notifying the user it's cleared

    let body = if cancelled {
        "Clipboard cleared (cancelled)"
    } else {
        "Clipboard cleared"
    };
    if let Err(error) = Notification::new()
        .summary("pasejo")
        .body(body)
        .timeout(Timeout::Default)
        .show()
    {
        log::debug!("Failed to show clipboard-cleared notification: {error}");
    }
    Ok(())
}
