// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

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
use zeroize::Zeroizing;

use crate::cli::i18n;

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

/// The two clipboard operations the guard needs at clear-time. Abstracting
/// them lets unit tests substitute a fake that records calls and fails on
/// demand, since `arboard::Clipboard` can't be constructed in headless CI
/// and can't be coaxed into failure.
trait ClipboardBackend {
    fn current_text(&mut self) -> anyhow::Result<String>;
    fn clear(&mut self) -> anyhow::Result<()>;
}

impl ClipboardBackend for arboard::Clipboard {
    fn current_text(&mut self) -> anyhow::Result<String> {
        self.get().text().map_err(anyhow::Error::from)
    }

    fn clear(&mut self) -> anyhow::Result<()> {
        Self::clear(self).map_err(anyhow::Error::from)
    }
}

/// RAII guard: clears the clipboard on drop, but only if it still contains
/// the secret we wrote. If the user has copied something else in the
/// meantime, we leave their new clipboard contents alone. The expected
/// value is held in `Zeroizing` so it's wiped from memory on drop.
struct ClipboardGuard<C: ClipboardBackend> {
    clipboard: C,
    expected: Zeroizing<String>,
    /// Set once `clear_if_unchanged` has been called. When true, `Drop`
    /// skips its fallback so we don't repeat the (potentially blocking)
    /// clipboard read on platforms like X11.
    cleared: bool,
}

impl<C: ClipboardBackend> ClipboardGuard<C> {
    /// Performs the clear-if-unchanged logic and returns the outcome so the
    /// caller can react (e.g. tailor a notification, log a failure).
    fn clear_if_unchanged(&mut self) -> anyhow::Result<ClearOutcome> {
        // Mark first so Drop becomes a no-op even if an early `?` propagates.
        self.cleared = true;
        match self.clipboard.current_text() {
            Ok(current) if current.as_str() == self.expected.as_str() => {
                self.clipboard.clear()?;
                Ok(ClearOutcome::Cleared)
            }
            Ok(_) => Ok(ClearOutcome::Unchanged),
            Err(error) => {
                // Couldn't read the clipboard to compare — clear defensively
                // rather than risk leaving the secret behind.
                i18n::clipboard_read_for_compare_failed(&error);
                self.clipboard.clear()?;
                Ok(ClearOutcome::ForciblyCleared)
            }
        }
    }
}

impl<C: ClipboardBackend> Drop for ClipboardGuard<C> {
    fn drop(&mut self) {
        // Best-effort fallback for panic / early-return paths. The explicit
        // call site sets `cleared = true` so this becomes a no-op there.
        // We log Drop-path failures at debug level so a `-vv` run leaves a
        // breadcrumb when triaging an unwind that wiped the clipboard
        // imperfectly; the user-visible notification path covers happy /
        // explicit-failure outcomes elsewhere.
        if self.cleared {
            return;
        }
        if let Err(error) = self.clear_if_unchanged() {
            i18n::clipboard_drop_clear_failed(&error);
        }
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
            i18n::clipboard_ctrlc_handler_install_failed(&error);
        }
    });
}

/// Copies `text` to the system clipboard for at most `duration`, then clears
/// it.
///
/// Intended for short-lived secrets (passwords, OTP codes). The function
/// blocks until either the timer expires or the user presses Ctrl-C, and
/// only returns once the clear-up step has run.
///
/// # Behaviour
///
/// - The text is placed on the clipboard with `exclude_from_history` set where
///   the platform supports it (macOS, Windows, and Linux clipboard managers
///   that honour the hint — many Linux managers do not).
/// - A process-wide Ctrl-C handler is installed on first call via
///   `ctrlc::set_handler`. Once installed, SIGINT sets a flag rather than
///   terminating the process, so the wait loop can exit cleanly and the
///   clipboard can be cleared. Subsequent calls reuse the same handler.
/// - `duration` is clamped to one year so the wait is always bounded.
/// - On exit, the clipboard is cleared only if it still contains the value we
///   wrote — if the user copied something else in the meantime, their new
///   contents are left alone.
/// - A desktop notification reports the outcome (cleared, untouched, or
///   failure) when `notify` is `true`. Set `notify` to `false` to silence the
///   popup — stderr warnings on clear failure are still emitted regardless.
///   Failure to notify is non-fatal.
/// - The in-memory copy of the secret is held in `Zeroizing` and wiped on drop.
///   The caller's `&str` is *not* wiped — that is the caller's responsibility.
///
/// # Errors
///
/// Returns `Err` if the clipboard handle cannot be opened or the initial
/// `set` fails. Failures during the wait loop or clear step are logged and
/// surfaced through the notification / stderr rather than returned.
pub fn copy_text_to_clipboard(text: &str, duration: Duration, notify: bool) -> anyhow::Result<()> {
    // Install the Ctrl-C handler before any secret enters the clipboard, so
    // SIGINT triggers our handler (which lets the loop exit and Drop run
    // clear()) instead of the default action that _exit()s without unwinding.
    install_interrupt_handler();
    // Reset the flag so a Ctrl-C from a prior invocation in the same process
    // doesn't make us skip the wait. The handler itself is `'static` and
    // remains installed across calls.
    INTERRUPTED.store(false, Ordering::Relaxed);

    // Set the clipboard first so a failed `text(...)` doesn't construct a
    // guard whose Drop would then try to clear something we never wrote.
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set().exclude_from_history().text(text)?;
    let mut guard = ClipboardGuard {
        clipboard,
        expected: Zeroizing::new(text.to_owned()),
        cleared: false,
    };

    // Poll so we respond to Ctrl-C promptly, clamping each tick so we never
    // oversleep the requested duration. Clamping `duration` to `MAX_DEADLINE`
    // both guarantees loop termination and keeps `Instant + Duration` from
    // overflowing on absurd configured timeouts.
    let deadline = Instant::now() + duration.min(MAX_DEADLINE);
    let cancelled = loop {
        if INTERRUPTED.load(Ordering::Relaxed) {
            break true;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break false;
        }
        thread::sleep(remaining.min(POLL_TICK));
    };

    let outcome = guard.clear_if_unchanged();
    drop(guard); // ensures the secret is wiped from memory even on early-return paths

    if let Err(error) = &outcome {
        // Critical path: the user's secret may still be in the clipboard.
        // Emit to stderr in addition to the notification, since a missing
        // notification daemon would otherwise leave the user uninformed.
        i18n::clipboard_clear_failed(error);
        i18n::clipboard_manual_clear_required();
    }
    if notify {
        let (body, timeout) = notification(&outcome, cancelled);
        if let Err(error) = Notification::new()
            .summary("pasejo")
            .body(&body)
            .timeout(timeout)
            .show()
        {
            i18n::clipboard_notification_dispatch_failed(&error);
        }
    }
    Ok(())
}

/// Builds the notification body and timeout for a given clear outcome.
/// Pure: no side effects, no I/O. Logging of the underlying error is the
/// caller's responsibility.
fn notification(outcome: &anyhow::Result<ClearOutcome>, cancelled: bool) -> (String, Timeout) {
    match outcome {
        Ok(ClearOutcome::Cleared) => (
            i18n::clipboard_notification_cleared(cancelled),
            Timeout::Default,
        ),
        Ok(ClearOutcome::Unchanged) => (
            i18n::clipboard_notification_unchanged(cancelled),
            Timeout::Default,
        ),
        Ok(ClearOutcome::ForciblyCleared) => (
            i18n::clipboard_notification_forcibly_cleared(cancelled),
            Timeout::Default,
        ),
        Err(_) => (
            i18n::clipboard_notification_failed(cancelled),
            Timeout::Never,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_deadline_does_not_overflow_instant() {
        // Smoke check: adding the clamp ceiling to "now" must not panic on
        // any platform we support.
        let _ = Instant::now() + MAX_DEADLINE;
    }

    #[test]
    fn duration_max_is_clamped_to_max_deadline() {
        assert_eq!(Duration::MAX.min(MAX_DEADLINE), MAX_DEADLINE);
    }

    #[test]
    fn small_duration_is_not_clamped() {
        let small = Duration::from_secs(45);
        assert_eq!(small.min(MAX_DEADLINE), small);
    }

    #[test]
    fn cleared_body_uses_default_timeout() {
        i18n::init_for_tests();
        let (body, timeout) = notification(&Ok(ClearOutcome::Cleared), false);
        assert_eq!(body, "Clipboard cleared");
        assert!(matches!(timeout, Timeout::Default));
    }

    #[test]
    fn cancelled_suffix_appended_to_cleared() {
        i18n::init_for_tests();
        let (body, _) = notification(&Ok(ClearOutcome::Cleared), true);
        assert_eq!(body, "Clipboard cleared (cancelled)");
    }

    #[test]
    fn cancelled_suffix_appended_to_unchanged() {
        i18n::init_for_tests();
        let (body, _) = notification(&Ok(ClearOutcome::Unchanged), true);
        assert!(body.starts_with("Clipboard left untouched"));
        assert!(body.ends_with(" (cancelled)"));
    }

    #[test]
    fn cancelled_suffix_appended_to_forcibly_cleared() {
        i18n::init_for_tests();
        let (body, _) = notification(&Ok(ClearOutcome::ForciblyCleared), true);
        assert!(body.contains("couldn't verify"));
        assert!(body.ends_with(" (cancelled)"));
    }

    #[test]
    fn cancelled_suffix_appended_to_failure() {
        i18n::init_for_tests();
        let (body, _) = notification(&Err(anyhow::anyhow!("boom")), true);
        assert!(body.starts_with("Failed to clear clipboard!"));
        assert!(body.ends_with(" (cancelled)"));
    }

    #[test]
    fn no_cancelled_suffix_when_not_cancelled() {
        for outcome in [
            Ok(ClearOutcome::Cleared),
            Ok(ClearOutcome::Unchanged),
            Ok(ClearOutcome::ForciblyCleared),
        ] {
            let (body, _) = notification(&outcome, false);
            assert!(
                !body.contains("(cancelled)"),
                "unexpected suffix in {body:?}"
            );
        }
        let (body, _) = notification(&Err(anyhow::anyhow!("boom")), false);
        assert!(
            !body.contains("(cancelled)"),
            "unexpected suffix in {body:?}"
        );
    }

    #[test]
    fn failure_uses_never_timeout() {
        let (_, timeout) = notification(&Err(anyhow::anyhow!("boom")), false);
        assert!(matches!(timeout, Timeout::Never));
    }

    #[test]
    fn success_variants_use_default_timeout() {
        for outcome in [
            Ok(ClearOutcome::Cleared),
            Ok(ClearOutcome::Unchanged),
            Ok(ClearOutcome::ForciblyCleared),
        ] {
            let (_, timeout) = notification(&outcome, false);
            assert!(matches!(timeout, Timeout::Default));
        }
    }

    // ---- ClipboardGuard / Drop-path coverage ----------------------------
    //
    // The Drop fallback only fires when the explicit `clear_if_unchanged`
    // call never ran (panic between guard creation and the explicit call,
    // or an early `?`). The tests below construct a `ClipboardGuard`
    // directly with a fake backend so we can observe whether `clear` was
    // attempted, without needing a real platform clipboard or a panicking
    // production path.

    use std::cell::Cell;
    use std::rc::Rc;

    #[derive(Default)]
    struct FakeStats {
        get_calls: Cell<u32>,
        clear_calls: Cell<u32>,
    }

    struct FakeClipboard {
        text: String,
        fail_get: bool,
        fail_clear: bool,
        stats: Rc<FakeStats>,
    }

    impl ClipboardBackend for FakeClipboard {
        fn current_text(&mut self) -> anyhow::Result<String> {
            self.stats.get_calls.set(self.stats.get_calls.get() + 1);
            if self.fail_get {
                anyhow::bail!("fake get failure");
            }
            Ok(self.text.clone())
        }

        fn clear(&mut self) -> anyhow::Result<()> {
            self.stats.clear_calls.set(self.stats.clear_calls.get() + 1);
            if self.fail_clear {
                anyhow::bail!("fake clear failure");
            }
            self.text.clear();
            Ok(())
        }
    }

    fn guard_with(
        text: &str,
        expected: &str,
        cleared: bool,
    ) -> (Rc<FakeStats>, ClipboardGuard<FakeClipboard>) {
        let stats = Rc::new(FakeStats::default());
        let guard = ClipboardGuard {
            clipboard: FakeClipboard {
                text: text.to_owned(),
                fail_get: false,
                fail_clear: false,
                stats: Rc::clone(&stats),
            },
            expected: Zeroizing::new(expected.to_owned()),
            cleared,
        };
        (stats, guard)
    }

    #[test]
    fn drop_with_cleared_flag_set_does_not_touch_backend() {
        // Mirrors the production happy path: explicit call ran, set cleared
        // = true; Drop must short-circuit so we don't double-clear or do a
        // second blocking read.
        let (stats, guard) = guard_with("secret", "secret", true);
        drop(guard);
        assert_eq!(stats.get_calls.get(), 0);
        assert_eq!(stats.clear_calls.get(), 0);
    }

    #[test]
    fn drop_clears_clipboard_when_text_still_matches() {
        // Models the panic / early-return path where the explicit call
        // never ran. Backend still holds our value, so Drop should clear.
        let (stats, guard) = guard_with("secret", "secret", false);
        drop(guard);
        assert_eq!(stats.get_calls.get(), 1);
        assert_eq!(stats.clear_calls.get(), 1);
    }

    #[test]
    fn drop_leaves_clipboard_untouched_when_user_copied_something_else() {
        // Same path, but the user has copied something else in the
        // meantime. We must not wipe their new clipboard contents.
        let (stats, guard) = guard_with("user-copied-this", "our-secret", false);
        drop(guard);
        assert_eq!(stats.get_calls.get(), 1);
        assert_eq!(stats.clear_calls.get(), 0);
    }

    #[test]
    fn drop_clears_defensively_when_compare_read_fails() {
        // Read-side failure: we can't compare, so we clear to err on the
        // side of not leaking a secret. Both calls happen; the get error
        // is logged via the i18n seam, the clear error (if any) reaches
        // Drop's debug log — but we only assert side-effect counts here,
        // since logger output isn't captured.
        let stats = Rc::new(FakeStats::default());
        let guard = ClipboardGuard {
            clipboard: FakeClipboard {
                text: String::from("secret"),
                fail_get: true,
                fail_clear: false,
                stats: Rc::clone(&stats),
            },
            expected: Zeroizing::new(String::from("secret")),
            cleared: false,
        };
        drop(guard);
        assert_eq!(stats.get_calls.get(), 1);
        assert_eq!(stats.clear_calls.get(), 1);
    }

    #[test]
    fn drop_swallows_clear_failure_without_panicking() {
        // Both reads succeed but clear fails. Drop must not propagate the
        // error (panicking from Drop during unwind would abort the
        // process); the debug log keeps a breadcrumb. We exercise the path
        // here so any future regression that *does* panic shows up as a
        // test failure.
        let stats = Rc::new(FakeStats::default());
        let guard = ClipboardGuard {
            clipboard: FakeClipboard {
                text: String::from("secret"),
                fail_get: false,
                fail_clear: true,
                stats: Rc::clone(&stats),
            },
            expected: Zeroizing::new(String::from("secret")),
            cleared: false,
        };
        drop(guard);
        assert_eq!(stats.get_calls.get(), 1);
        assert_eq!(stats.clear_calls.get(), 1);
    }

    #[test]
    fn explicit_clear_propagates_clear_failure_to_caller() {
        // The non-Drop path: production code calls `clear_if_unchanged`
        // and inspects the Result to drive the user-visible notification.
        // A clear failure must surface as Err so the caller can switch to
        // the "manual clear required" branch.
        let stats = Rc::new(FakeStats::default());
        let mut guard = ClipboardGuard {
            clipboard: FakeClipboard {
                text: String::from("secret"),
                fail_get: false,
                fail_clear: true,
                stats: Rc::clone(&stats),
            },
            expected: Zeroizing::new(String::from("secret")),
            cleared: false,
        };
        let outcome = guard.clear_if_unchanged();
        assert!(outcome.is_err());
        // The cleared flag was flipped before the failure, so a follow-up
        // Drop must be a no-op (no second clear attempt).
        assert!(guard.cleared);
        let calls_before_drop = stats.clear_calls.get();
        drop(guard);
        assert_eq!(stats.clear_calls.get(), calls_before_drop);
    }
}
