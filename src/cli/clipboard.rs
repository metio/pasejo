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

const POLL_TICK: Duration = Duration::from_millis(100);

/// RAII guard: clears the clipboard on drop. Covers normal return, `?`
/// propagation, panic unwind, and the explicit `drop(guard)` we use to
/// clear before showing the notification.
struct ClipboardGuard {
    clipboard: arboard::Clipboard,
}

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        // Best-effort: nothing useful to do if this fails on shutdown.
        let _ = self.clipboard.clear();
    }
}

static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static HANDLER_INSTALLED: OnceLock<()> = OnceLock::new();

/// Installs a process-wide Ctrl-C handler on first call. `ctrlc::set_handler`
/// only allows one handler per process, so we install it lazily once and
/// reuse the same `INTERRUPTED` flag for every call.
fn install_interrupt_handler() {
    HANDLER_INSTALLED.get_or_init(|| {
        let _ = ctrlc::set_handler(|| {
            INTERRUPTED.store(true, Ordering::Relaxed);
        });
    });
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

    drop(guard); // clear clipboard before notifying the user it's cleared

    if let Err(error) = Notification::new()
        .summary("pasejo")
        .body("Clipboard cleared")
        .timeout(Timeout::Default)
        .show()
    {
        log::debug!("Failed to show clipboard-cleared notification: {error}");
    }
    Ok(())
}
