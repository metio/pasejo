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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

/// RAII guard: clears the clipboard when dropped, whether that's because
/// `copy_text_to_clipboard` returned normally, an error propagated, or the
/// thread unwound from a panic. The Ctrl-C handler sets the shared flag,
/// which breaks the wait loop and causes the guard to drop early.
struct ClipboardGuard {
    clipboard: arboard::Clipboard,
    cleared: bool,
}

impl ClipboardGuard {
    fn clear(&mut self) {
        if !self.cleared {
            // Best-effort: nothing useful to do if this fails on shutdown.
            let _ = self.clipboard.clear();
            self.cleared = true;
        }
    }
}

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        self.clear();
    }
}

static INTERRUPTED: OnceLock<Arc<AtomicBool>> = OnceLock::new();

/// Returns a process-wide interrupt flag, installing the Ctrl-C handler on
/// first use. `ctrlc::set_handler` only allows one handler per process, so
/// installing it once and reusing the same flag avoids the second-call
/// silent-failure where a stale handler consumes SIGINT but no live caller
/// reads its flag.
fn interrupt_flag() -> &'static Arc<AtomicBool> {
    INTERRUPTED.get_or_init(|| {
        let flag = Arc::new(AtomicBool::new(false));
        let handler_flag = Arc::clone(&flag);
        let _ = ctrlc::set_handler(move || {
            handler_flag.store(true, Ordering::Relaxed);
        });
        flag
    })
}

pub fn copy_text_to_clipboard(text: &str, duration: Duration) -> anyhow::Result<()> {
    // Install the Ctrl-C handler before any secret enters the clipboard, so
    // SIGINT triggers our handler (which lets the loop exit and Drop run
    // clear()) instead of the default action that _exit()s without unwinding.
    let interrupted = interrupt_flag();
    interrupted.store(false, Ordering::Relaxed);

    let mut guard = ClipboardGuard {
        clipboard: arboard::Clipboard::new()?,
        cleared: false,
    };
    guard.clipboard.set().exclude_from_history().text(text)?;

    // Poll instead of one long sleep so we can respond to Ctrl-C promptly,
    // and clamp each tick to the remaining time so we never oversleep the
    // requested duration.
    let deadline = Instant::now() + duration;
    let tick = Duration::from_millis(100);
    loop {
        if interrupted.load(Ordering::Relaxed) {
            break;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        thread::sleep(remaining.min(tick));
    }

    guard.clear();

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
