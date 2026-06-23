//! Synchronization primitives building on and extending `std::sync` and `tokio::sync`

pub use self::swap_lock::SwapLock;
pub use self::typed_semaphore::OwnedTypedSemaphorePermit;
pub use self::typed_semaphore::TypedSemaphore;
pub use self::typed_semaphore::TypedSemaphorePermit;

mod swap_lock;
mod typed_semaphore;
