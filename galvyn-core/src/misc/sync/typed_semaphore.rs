use std::ops::Deref;
use std::sync::Arc;

use tokio::sync::AcquireError;
#[cfg(doc)]
use tokio::sync::OwnedSemaphorePermit;
use tokio::sync::Semaphore;
#[cfg(doc)]
use tokio::sync::SemaphorePermit;
use tokio::sync::TryAcquireError;

/// A [`Semaphore`] which guards the access to same value of `T`
///
/// Most `Mutex` implementation wrap some value `T` and only expose it through its `MutexGuard`.
/// This API design is somewhat necessary for rust but proved to be a really useful concept.
/// (You can't accidentally access a value which should have been guarded.)
///
/// This type applies this concept to semaphores.
/// While not necessary, it can still be useful
#[derive(Debug)]
pub struct TypedSemaphore<T> {
    semaphore: Semaphore,
    value: T,
}

impl<T> TypedSemaphore<T> {
    /// Creates a new semaphore with a value to guard and the initial number of permits.
    ///
    /// See [`Semaphore::new`]
    pub fn new(value: T, permits: usize) -> Self {
        Self {
            semaphore: Semaphore::new(permits),
            value,
        }
    }

    /// Creates a new semaphore with a `value` to guard and the initial number of `permits`.
    ///
    /// See [`Semaphore::const_new`]
    pub const fn const_new(value: T, permits: usize) -> Self {
        Self {
            semaphore: Semaphore::const_new(permits),
            value,
        }
    }

    /// Returns the current number of available permits.
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Adds `n` new permits to the semaphore.
    ///
    /// See [`Semaphore::add_permits`]
    pub fn add_permits(&self, n: usize) {
        self.semaphore.add_permits(n)
    }

    /// Decrease a semaphore’s permits by a maximum of `n`.
    ///
    /// See [`Semaphore::forget_permits`]
    pub fn forget_permits(&self, n: usize) -> usize {
        self.semaphore.forget_permits(n)
    }

    /// Acquires a permit from the semaphore.
    ///
    /// See [`Semaphore::acquire`]
    pub async fn acquire(&self) -> Result<TypedSemaphorePermit<'_, T>, AcquireError> {
        self.semaphore.acquire().await?.forget();
        Ok(TypedSemaphorePermit {
            semaphore: self,
            permits: 1,
        })
    }

    /// Acquires `n` permits from the semaphore.
    ///
    /// See [`Semaphore::acquire_many`]
    pub async fn acquire_many(&self, n: u32) -> Result<TypedSemaphorePermit<'_, T>, AcquireError> {
        self.semaphore.acquire_many(n).await?.forget();
        Ok(TypedSemaphorePermit {
            semaphore: self,
            permits: n,
        })
    }

    /// Tries to acquire a permit from the semaphore.
    ///
    /// See [`Semaphore::try_acquire`]
    pub fn try_acquire(&self) -> Result<TypedSemaphorePermit<'_, T>, TryAcquireError> {
        self.semaphore.try_acquire()?.forget();
        Ok(TypedSemaphorePermit {
            semaphore: self,
            permits: 1,
        })
    }

    /// Tries to acquire `n` permits from the semaphore.
    ///
    /// See [`Semaphore::try_acquire_many`]
    pub fn try_acquire_many(&self, n: u32) -> Result<TypedSemaphorePermit<'_, T>, TryAcquireError> {
        self.semaphore.try_acquire_many(n)?.forget();
        Ok(TypedSemaphorePermit {
            semaphore: self,
            permits: n,
        })
    }

    /// Acquires a permit from the semaphore.
    ///
    /// See [`Semaphore::acquire`]
    pub async fn acquire_owned(
        self: Arc<Self>,
    ) -> Result<OwnedTypedSemaphorePermit<T>, AcquireError> {
        self.semaphore.acquire().await?.forget();
        Ok(OwnedTypedSemaphorePermit {
            semaphore: self,
            permits: 1,
        })
    }

    /// Acquires `n` permits from the semaphore.
    ///
    /// See [`Semaphore::acquire_many`]
    pub async fn acquire_many_owned(
        self: Arc<Self>,
        n: u32,
    ) -> Result<OwnedTypedSemaphorePermit<T>, AcquireError> {
        self.semaphore.acquire_many(n).await?.forget();
        Ok(OwnedTypedSemaphorePermit {
            semaphore: self,
            permits: n,
        })
    }

    /// Tries to acquire a permit from the semaphore.
    ///
    /// See [`Semaphore::try_acquire`]
    pub fn try_acquire_owned(
        self: Arc<Self>,
    ) -> Result<OwnedTypedSemaphorePermit<T>, TryAcquireError> {
        self.semaphore.try_acquire()?.forget();
        Ok(OwnedTypedSemaphorePermit {
            semaphore: self,
            permits: 1,
        })
    }

    /// Tries to acquire `n` permits from the semaphore.
    ///
    /// See [`Semaphore::try_acquire_many`]
    pub fn try_acquire_many_owned(
        self: Arc<Self>,
        n: u32,
    ) -> Result<OwnedTypedSemaphorePermit<T>, TryAcquireError> {
        self.semaphore.try_acquire_many(n)?.forget();
        Ok(OwnedTypedSemaphorePermit {
            semaphore: self,
            permits: n,
        })
    }

    /// Closes the semaphore.
    ///
    /// See [`Semaphore::close`]
    pub fn close(&self) {
        self.semaphore.close();
    }

    /// Returns true if the semaphore is closed.
    ///
    /// See [`Semaphore::is_closed`]
    pub fn is_closed(&self) -> bool {
        self.semaphore.is_closed()
    }

    /// Borrows the underlying semaphore
    pub fn semaphore(&self) -> &Semaphore {
        &self.semaphore
    }

    /// Borrows the guarded value without acquiring the semaphore.
    ///
    /// This defeats this types entire purpose but exists as an escape hatch, if you really need it.
    /// Its name is overly long and explicit to avoid accidental use and make it obvious in reviews.
    pub fn value_without_acquiring_the_semaphore(&self) -> &T {
        &self.value
    }
}

#[must_use]
#[clippy::has_significant_drop]
#[derive(Debug)]
pub struct TypedSemaphorePermit<'a, T> {
    semaphore: &'a TypedSemaphore<T>,
    permits: u32,
}

impl<T> TypedSemaphorePermit<'_, T> {
    /// Forgets the permit **without** releasing it back to the semaphore.
    ///
    /// See [`SemaphorePermit::forget`]
    pub fn forget(mut self) {
        self.permits = 0;
    }

    /// Merge two `Self` instances together, consuming `other` without releasing the permits it holds.
    ///
    /// See [`SemaphorePermit::merge`]
    pub fn merge(&mut self, mut other: Self) {
        assert!(
            std::ptr::eq(self.semaphore, other.semaphore),
            "merging permits from different semaphore instances"
        );
        self.permits += other.permits;
        other.permits = 0;
    }

    /// Splits n permits from `self` and returns a new `Self` instance that holds `n` permits.
    ///
    /// See [`SemaphorePermit::split`]
    pub fn split(&mut self, n: usize) -> Option<Self> {
        let n = u32::try_from(n).ok()?;

        if n > self.permits {
            return None;
        }

        self.permits -= n;

        Some(Self {
            semaphore: self.semaphore,
            permits: n,
        })
    }

    /// Returns the number of permits held by self.
    pub fn num_permits(&self) -> usize {
        self.permits as usize
    }
}

impl<T> Drop for TypedSemaphorePermit<'_, T> {
    fn drop(&mut self) {
        self.semaphore.add_permits(self.permits as usize);
    }
}

impl<T> Deref for TypedSemaphorePermit<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.semaphore.value
    }
}

#[must_use]
#[clippy::has_significant_drop]
#[derive(Debug)]
pub struct OwnedTypedSemaphorePermit<T> {
    semaphore: Arc<TypedSemaphore<T>>,
    permits: u32,
}

impl<T> OwnedTypedSemaphorePermit<T> {
    /// Forgets the permit **without** releasing it back to the semaphore.
    ///
    /// See [`OwnedSemaphorePermit::forget`]
    pub fn forget(mut self) {
        self.permits = 0;
    }

    /// Merge two `Self` instances together, consuming `other` without releasing the permits it holds.
    ///
    /// See [`OwnedSemaphorePermit::merge`]
    pub fn merge(&mut self, mut other: Self) {
        assert!(
            Arc::ptr_eq(&self.semaphore, &other.semaphore),
            "merging permits from different semaphore instances"
        );
        self.permits += other.permits;
        other.permits = 0;
    }

    /// Splits n permits from `self` and returns a new `Self` instance that holds `n` permits.
    ///
    /// See [`OwnedSemaphorePermit::split`]
    pub fn split(&mut self, n: usize) -> Option<Self> {
        let n = u32::try_from(n).ok()?;

        if n > self.permits {
            return None;
        }

        self.permits -= n;

        Some(Self {
            semaphore: Arc::clone(&self.semaphore),
            permits: n,
        })
    }

    /// Returns the number of permits held by self.
    pub fn num_permits(&self) -> usize {
        self.permits as usize
    }
}

impl<T> Drop for OwnedTypedSemaphorePermit<T> {
    fn drop(&mut self) {
        self.semaphore.add_permits(self.permits as usize);
    }
}

impl<T> Deref for OwnedTypedSemaphorePermit<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.semaphore.value
    }
}
