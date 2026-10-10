//! Native change notifications wake the existing settling/reconciliation logic.
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
const SETTLE_POLL: Duration = Duration::from_secs(3);
const RECONCILE: Duration = Duration::from_secs(60);

pub struct FolderWatch {
    root: String,
    watcher: Option<RecommendedWatcher>,
    changed: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    last_scan: Instant,
}
fn scan_due(elapsed: Duration, changed: bool, pending: bool, fallback: bool) -> bool {
    elapsed >= RECONCILE || (elapsed >= SETTLE_POLL && (changed || pending || fallback))
}
impl FolderWatch {
    pub fn new() -> Self {
        Self {
            root: String::new(),
            watcher: None,
            changed: Arc::new(AtomicBool::new(false)),
            failed: Arc::new(AtomicBool::new(false)),
            last_scan: Instant::now(),
        }
    }
    pub fn due(&mut self, root: &str, pending: bool) -> bool {
        if self.root != root {
            self.watcher = None;
            self.root = root.into();
            // A new subscription has its own flags: late callbacks cannot wake another folder.
            self.changed = Arc::new(AtomicBool::new(!root.is_empty()));
            self.failed = Arc::new(AtomicBool::new(false));
            self.last_scan = Instant::now() - SETTLE_POLL;
            if !root.is_empty() {
                let changed = self.changed.clone();
                let failed = self.failed.clone();
                self.watcher =
                    notify::recommended_watcher(
                        move |result: notify::Result<Event>| match result {
                            Ok(event) if !matches!(event.kind, EventKind::Access(_)) => {
                                changed.store(true, Ordering::Release);
                            }
                            Err(_) => {
                                failed.store(true, Ordering::Release);
                                changed.store(true, Ordering::Release);
                            }
                            _ => {}
                        },
                    )
                    .and_then(|mut watcher| {
                        watcher.watch(Path::new(root), RecursiveMode::Recursive)?;
                        Ok(watcher)
                    })
                    .ok();
            }
        }
        if root.is_empty() {
            return false;
        }
        if self.failed.load(Ordering::Acquire) {
            self.watcher = None;
        }
        if scan_due(
            self.last_scan.elapsed(),
            self.changed.load(Ordering::Acquire),
            pending,
            self.watcher.is_none(),
        ) {
            self.changed.store(false, Ordering::Release);
            self.last_scan = Instant::now();
            return true;
        }
        false
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idle_native_watch_avoids_polling_but_reconciles() {
        assert!(!scan_due(Duration::from_secs(3), false, false, false));
        assert!(!scan_due(Duration::from_secs(59), false, false, false));
        assert!(scan_due(Duration::from_secs(60), false, false, false));
    }
    #[test]
    fn events_settling_and_failed_subscription_all_trigger_scan() {
        assert!(!scan_due(Duration::from_secs(2), true, true, true));
        for (changed, pending, fallback) in [
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            assert!(scan_due(Duration::from_secs(3), changed, pending, fallback));
        }
    }
    #[test]
    fn stopping_watch_clears_subscription_and_pending_wakeup() {
        let directory = tempfile::tempdir().unwrap();
        let mut watcher = FolderWatch::new();
        assert!(watcher.due(directory.path().to_str().unwrap(), false));
        assert!(!watcher.due("", true));
        assert!(watcher.watcher.is_none());
        assert!(!watcher.changed.load(Ordering::Acquire));
    }
}
