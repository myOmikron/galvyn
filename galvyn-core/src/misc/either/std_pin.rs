//! Pin projection for [`Either`]
//!
//! This is a somewhat hand-rolled version of what `pin-project-lite` would generate.
//! It has a cleaner API since it uses itself as projection type
//! and `pin-project-lite` does not support enums with tuple variants.

use std::pin::Pin;

use crate::misc::either::Either;

impl<L, R> Either<L, R> {
    /// Projects a `Pin<&Either<L, R>` into a `Either<Pin<&L>, Pin<&R>>`
    pub fn as_pin_ref(self: Pin<&Self>) -> Either<Pin<&L>, Pin<&R>> {
        unsafe {
            // SAFETY: TODO
            match self.get_ref() {
                Either::Left(x) => Either::Left(Pin::new_unchecked(x)),
                Either::Right(x) => Either::Right(Pin::new_unchecked(x)),
            }
        }
    }

    /// Projects a `Pin<&mut Either<L, R>` into a `Either<Pin<&mut L>, Pin<&mut R>>`
    pub fn as_pin_mut(self: Pin<&mut Self>) -> Either<Pin<&mut L>, Pin<&mut R>> {
        unsafe {
            // SAFETY: TODO
            match self.get_unchecked_mut() {
                Either::Left(x) => Either::Left(Pin::new_unchecked(x)),
                Either::Right(x) => Either::Right(Pin::new_unchecked(x)),
            }
        }
    }
}

impl<L, R> Unpin for Either<L, R>
where
    L: Unpin,
    R: Unpin,
{
}

// The following hack is taken from `pin-project-lite`,
// it prevents the accidental implementation of `Drop` for `Either`
#[allow(dead_code)]
trait MustNotImplDrop {}
#[allow(drop_bounds)]
impl<T: Drop> MustNotImplDrop for T {}
impl<L, R> MustNotImplDrop for Either<L, R> {}
