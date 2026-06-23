//! Synchronization primitives building on and extending `std::sync` and `tokio::sync`

pub use self::swap_lock::SwapLock;

mod swap_lock;
