// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use pasejo_protocol::Millis;

/// The server's clock. Tests move it forward to expire sessions, grants and
/// windows without waiting.
#[derive(Debug, Default)]
pub struct Clock {
    offset: AtomicI64,
}

impl Clock {
    #[must_use]
    pub fn now(&self) -> Millis {
        let real = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| {
                i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
            });
        u64::try_from(real.saturating_add(self.offset.load(Ordering::Relaxed))).unwrap_or(0)
    }

    pub fn advance(&self, millis: i64) {
        self.offset.fetch_add(millis, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advancing_moves_the_clock_forward() {
        let clock = Clock::default();
        let before = clock.now();
        clock.advance(3_600_000);
        assert!(clock.now() >= before + 3_600_000);
    }
}
