use std::ops::Deref;
use std::ops::DerefMut;

use crate::stuff::api_error::ApiError;
use crate::stuff::api_error::ApiResult;

/// Tracks whether any field of some form error `E` has been set
#[derive(Default)]
pub struct FormErrors<E> {
    /// The wrapped form error
    inner: E,

    /// Has any field been set?
    ///
    /// I.e., do we have an error to return?
    modified: bool,
}

impl<E: Default> FormErrors<E> {
    /// Constructs a new `FormErrors`
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks whether any field has been set and returns `Err` if any has.
    pub fn check(self) -> ApiResult<(), E> {
        if self.modified {
            Err(ApiError::FormError(self.inner))
        } else {
            Ok(())
        }
    }

    /// Returns the form error as `Err`.
    pub fn fail<T>(self) -> ApiResult<T, E> {
        Err(ApiError::FormError(self.inner))
    }
}

impl<E> Deref for FormErrors<E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<E> DerefMut for FormErrors<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.modified = true;
        &mut self.inner
    }
}
