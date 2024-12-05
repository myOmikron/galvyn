//! Thin wrapper around an [`RwLock`] with a specialized API.
//!
//! Think [`Cell`](std::cell::Cell) but sync.

use std::mem;
use std::sync::LockResult;
use std::sync::RwLock;

/// Thin wrapper around an [`RwLock`] with a specialized API:
///
/// Clone the contained value with [`SwapLock::get`] or swap it with [`SwapLock::swap`].
///
/// Think [`Cell`](std::cell::Cell) but sync.
#[derive(Debug, Default)]
pub struct SwapLock<T: Clone>(RwLock<T>);
impl<T: Clone> SwapLock<T> {
    /// Constructs a new `SwapLock`
    pub const fn new(value: T) -> Self {
        Self(RwLock::new(value))
    }

    /// Clones the current value
    pub fn get(&self) -> T {
        self.handle_poison(self.0.read()).clone()
    }

    /// Swaps the current value for a new one
    pub fn swap(&self, mut value: T) -> T {
        let mut guard = self.handle_poison(self.0.write());
        mem::swap(&mut *guard, &mut value);
        value
    }

    fn handle_poison<Guard>(&self, result: LockResult<Guard>) -> Guard {
        // We can safely ignore poisoning
        // because the `swap` can't panic and the `clone` really shouldn't modify the cloned value
        result.unwrap_or_else(|poison| {
            self.0.clear_poison();
            poison.into_inner()
        })
    }
}
