//! Either a "left" value of type `L` or a "right" value of type `R`.

use std::error::Error;
use std::fmt;
use std::ops::Deref;
use std::ops::DerefMut;

mod futures_stream;
mod std_future;
mod std_io;
mod std_iter;
mod std_pin;
mod tokio_io;

/// Either a "left" value of type `L` or a "right" value of type `R`.
///
/// Think `Result` but without the semantic difference between `Ok` and `Err`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Either<L, R> {
    /// The "left" value of type `L`
    Left(L),

    /// The "right" value of type `R`
    Right(R),
}

impl<L, R> Either<L, R> {
    /// Converts from `&Either<L, R>` to `Either<&L, &R>`.
    ///
    /// Produces a new `Either`, containing a reference into the original, leaving the original in place.
    pub const fn as_ref(&self) -> Either<&L, &R> {
        match self {
            Either::Left(x) => Either::Left(x),
            Either::Right(x) => Either::Right(x),
        }
    }

    /// Converts from `&mut Either<L, R>` to `Either<&mut L, &mut R>`.
    pub const fn as_mut(&mut self) -> Either<&mut L, &mut R> {
        match self {
            Either::Left(x) => Either::Left(x),
            Either::Right(x) => Either::Right(x),
        }
    }

    /// Flips "left" and "right"
    pub fn flip(self) -> Either<R, L> {
        match self {
            Either::Left(x) => Either::Right(x),
            Either::Right(x) => Either::Left(x),
        }
    }

    /// Returns `true` if the either is `Left`
    pub const fn is_left(&self) -> bool {
        matches!(self, Self::Left(_))
    }

    /// Returns `true` if the either is `Left` and its value satisfies `check`
    pub fn is_left_and(&self, check: impl FnOnce(&L) -> bool) -> bool {
        matches!(self, Self::Left(x) if check(x))
    }

    /// Returns `true` if the either is `Left` or the `Right` value satisfies `check`
    pub fn is_left_or(&self, check: impl FnOnce(&R) -> bool) -> bool {
        matches!(self, Self::Left(_)) || self.is_right_and(check)
    }

    /// Borrows the `Either` into an `Option` which is `Some` if the `Either` was `Left`.
    pub const fn as_left(&self) -> Option<&L> {
        match self {
            Either::Left(x) => Some(x),
            Either::Right(_) => None,
        }
    }

    /// Borrows the `Either` into an `Option` which is `Some` if the `Either` was `Left`.
    pub const fn as_left_mut(&mut self) -> Option<&mut L> {
        match self {
            Either::Left(x) => Some(x),
            Either::Right(_) => None,
        }
    }

    /// Converts the `Either` into an `Option` which is `Some` if the `Either` was `Left`.
    ///
    /// # Examples
    ///
    /// ```
    /// let x: Either<&str, &str> = Either::Left("foo");
    /// assert_eq!(x.left(), Some("foo"));
    ///
    /// let y: Either<&str, &str> = Either::Right("bar");
    /// assert_eq!(y.left(), None);
    /// ```
    pub fn left(self) -> Option<L> {
        match self {
            Either::Left(x) => Some(x),
            Either::Right(_) => None,
        }
    }

    /// Converts the `Either` into a `Result` which is `Ok` if the `Either` was `Left`.
    ///
    /// # Examples
    ///
    /// ```
    /// let x: Either<&str, &str> = Either::Left("foo");
    /// assert_eq!(x.left_ok(), Ok("foo"));
    ///
    /// let y: Either<&str, &str> = Either::Right("bar");
    /// assert_eq!(y.left_ok(), Err("bar"));
    /// ```
    pub fn left_ok(self) -> Result<L, R> {
        match self {
            Either::Left(x) => Ok(x),
            Either::Right(x) => Err(x),
        }
    }

    /// Returns the contained `Left` value, consuming the `self` value.
    ///
    /// # Panics
    ///
    /// If the value is `Right`, with a panic message provided by the `Right`’s value.
    ///
    /// # Example
    ///
    /// ```
    /// let x: Either<&str, &str> = Either::Left("foo");
    /// assert_eq!(x.unwrap_left(), "foo");
    /// ```
    pub fn unwrap_left(self) -> L
    where
        R: fmt::Debug,
    {
        match self {
            Either::Left(x) => x,
            Either::Right(x) => panic!("Called `Either::unwrap_left` on a `Right` value: {x:?}"),
        }
    }

    /// Returns `true` if the either is `Right`
    pub const fn is_right(&self) -> bool {
        matches!(self, Self::Right(_))
    }

    /// Returns `true` if the either is `Right` and its value satisfies `check`
    pub fn is_right_and(&self, check: impl FnOnce(&R) -> bool) -> bool {
        matches!(self, Self::Right(x) if check(x))
    }

    /// Returns `true` if the either is `Right` or the `Left` value satisfies `check`
    pub fn is_right_or(&self, check: impl FnOnce(&L) -> bool) -> bool {
        matches!(self, Self::Right(_)) || self.is_left_and(check)
    }

    /// Borrows the `Either` as an `Option` which is `Some` if the `Either` was `Right`.
    pub const fn as_right(&self) -> Option<&R> {
        match self {
            Either::Left(_) => None,
            Either::Right(x) => Some(x),
        }
    }

    /// Borrows the `Either` as an `Option` which is `Some` if the `Either` was `Right`.
    pub const fn as_right_mut(&mut self) -> Option<&mut R> {
        match self {
            Either::Left(_) => None,
            Either::Right(x) => Some(x),
        }
    }

    /// Converts the `Either` into an `Option` which is `Some` if the `Either` was `Right`.
    ///
    /// # Examples
    ///
    /// ```
    /// let x: Either<&str, &str> = Either::Right("foo");
    /// assert_eq!(x.right(), Some("foo"));
    ///
    /// let y: Either<&str, &str> = Either::Left("bar");
    /// assert_eq!(y.right(), None);
    /// ```
    pub fn right(self) -> Option<R> {
        match self {
            Either::Left(_) => None,
            Either::Right(x) => Some(x),
        }
    }

    /// Converts the `Either` into a `Result` which is `Ok` if the `Either` was `Right`.
    ///
    /// # Examples
    ///
    /// ```
    /// let x: Either<&str, &str> = Either::Right("foo");
    /// assert_eq!(x.right_ok(), Ok("foo"));
    ///
    /// let y: Either<&str, &str> = Either::Left("bar");
    /// assert_eq!(y.right_ok(), Err("bar"));
    /// ```
    pub fn right_ok(self) -> Result<R, L> {
        match self {
            Either::Left(x) => Err(x),
            Either::Right(x) => Ok(x),
        }
    }

    /// Returns the contained `Right` value, consuming the `self` value.
    ///
    /// # Panics
    ///
    /// If the value is `Left`, with a panic message provided by the `Left`’s value.
    ///
    /// # Example
    ///
    /// ```
    /// let x: Either<&str, &str> = Either::Right("foo");
    /// assert_eq!(x.unwrap_right(), "foo");
    /// ```
    pub fn unwrap_right(self) -> R
    where
        L: fmt::Debug,
    {
        match self {
            Either::Left(x) => panic!("Called `Either::unwrap_right` on a `Left` value: {x:?}"),
            Either::Right(x) => x,
        }
    }
}

impl<T> Either<T, T> {
    /// Unwraps the `Either` if both `Left` and `Right` would be of the same type
    pub fn into_inner(self) -> T {
        match self {
            Either::Left(x) => x,
            Either::Right(x) => x,
        }
    }
}

impl<L, R> Either<&L, &R> {
    /// Converts the `Either<&L, &R>` into an `Either<L, R>` by cloning its value
    pub fn cloned(self) -> Either<L, R>
    where
        L: Clone,
        R: Clone,
    {
        match self {
            Either::Left(x) => Either::Left(x.clone()),
            Either::Right(x) => Either::Right(x.clone()),
        }
    }

    /// Converts the `Either<&L, &R>` into an `Either<L, R>` by copying its value
    pub fn copied(self) -> Either<L, R>
    where
        L: Copy,
        R: Copy,
    {
        match self {
            Either::Left(x) => Either::Left(*x),
            Either::Right(x) => Either::Right(*x),
        }
    }
}

impl<L, R> Either<&mut L, &mut R> {
    /// Converts the `Either<&mut L, &mut R>` into an `Either<L, R>` by cloning its value
    pub fn cloned(self) -> Either<L, R>
    where
        L: Clone,
        R: Clone,
    {
        match self {
            Either::Left(x) => Either::Left(x.clone()),
            Either::Right(x) => Either::Right(x.clone()),
        }
    }

    /// Converts the `Either<&mut L, &mut R>` into an `Either<L, R>` by copying its value
    pub fn copied(self) -> Either<L, R>
    where
        L: Copy,
        R: Copy,
    {
        match self {
            Either::Left(x) => Either::Left(*x),
            Either::Right(x) => Either::Right(*x),
        }
    }
}

impl<L, R> fmt::Display for Either<L, R>
where
    L: fmt::Display,
    R: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Either::Left(x) => x.fmt(f),
            Either::Right(x) => x.fmt(f),
        }
    }
}
impl<L, R> Error for Either<L, R>
where
    L: Error + 'static,
    R: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Either::Left(x) => Some(x),
            Either::Right(x) => Some(x),
        }
    }
}

impl<L, R, T> AsRef<T> for Either<L, R>
where
    L: AsRef<T>,
    R: AsRef<T>,
{
    fn as_ref(&self) -> &T {
        match self {
            Either::Left(x) => x.as_ref(),
            Either::Right(x) => x.as_ref(),
        }
    }
}

impl<L, R, T> AsMut<T> for Either<L, R>
where
    L: AsMut<T>,
    R: AsMut<T>,
{
    fn as_mut(&mut self) -> &mut T {
        match self {
            Either::Left(x) => x.as_mut(),
            Either::Right(x) => x.as_mut(),
        }
    }
}

impl<L, R, T> Deref for Either<L, R>
where
    L: Deref<Target = T>,
    R: Deref<Target = T>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Either::Left(x) => x.deref(),
            Either::Right(x) => x.deref(),
        }
    }
}

impl<L, R, T> DerefMut for Either<L, R>
where
    L: DerefMut<Target = T>,
    R: DerefMut<Target = T>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Either::Left(x) => x.deref_mut(),
            Either::Right(x) => x.deref_mut(),
        }
    }
}
