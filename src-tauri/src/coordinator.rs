//! Serializes whole model workflows across entry points; the on-device model runs one at a time.
use std::sync::{Mutex, MutexGuard};

static PROCESSING: Mutex<()> = Mutex::new(());
pub(crate) fn serial() -> MutexGuard<'static, ()> {
    PROCESSING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
