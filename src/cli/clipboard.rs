// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

#[cfg(target_os = "macos")]
pub use arboard::SetExtApple;
#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "android", target_os = "emscripten")),
))]
use arboard::SetExtLinux;
#[cfg(windows)]
pub use arboard::SetExtWindows;
use notify_rust::{Notification, Timeout};
use std::thread;
use std::time::Duration;

pub fn copy_text_to_clipboard(text: &str, duration: Duration) -> anyhow::Result<()> {
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set().exclude_from_history().text(text)?;
    thread::sleep(duration);
    clipboard.clear()?;
    Notification::new()
        .summary("pasejo")
        .body("Clipboard cleared")
        .timeout(Timeout::Default)
        .show()?;
    Ok(())
}
