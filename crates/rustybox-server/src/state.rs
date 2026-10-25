//! Shared application state: one mutex-guarded store behind an `Arc`.
//!
//! The lock is a plain `std::sync::Mutex` — no handler holds it across an
//! `.await`, so the runtime is never blocked. A poisoned lock (a panic while
//! holding it) is recovered instead of propagating: the in-memory ideas are
//! still consistent enough to serve, and losing them to a poisoned mutex
//! would be strictly worse.

use std::sync::{Arc, Mutex, MutexGuard};

use rustybox_core::Store;

pub struct AppState {
    store: Mutex<Store>,
}

impl AppState {
    pub fn new(store: Store) -> Self {
        AppState {
            store: Mutex::new(store),
        }
    }

    /// Lock the store, recovering from poisoning.
    pub fn lock(&self) -> MutexGuard<'_, Store> {
        self.store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub type SharedState = Arc<AppState>;

/// Persist after a mutation; on failure, tell the user rather than pretend.
/// Returns the flash payload to attach when the save failed (None = saved).
pub fn persist(store: &Store) -> Option<&'static str> {
    if let Err(e) = store.save() {
        eprintln!("rustybox: save failed: {e}");
        return Some("save_failed");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poisoned_mutex_is_recovered() {
        let state = AppState::new(Store::empty("/tmp/state-test.json"));
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = state.lock();
            panic!("poison the mutex");
        }));
        // The next lock must succeed instead of panicking on poison.
        assert!(state.lock().ideas.is_empty());
    }
}
