//! The on-device model serves one request at a time, so whole model workflows (suggesting,
//! filing a document, checking a connection) hold this lock instead of interleaving.
use std::sync::{Mutex, MutexGuard};

static MODEL: Mutex<()> = Mutex::new(());

pub(crate) fn hold() -> MutexGuard<'static, ()> {
    MODEL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
