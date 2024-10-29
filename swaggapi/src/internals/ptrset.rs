use std::cmp::Ordering;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::{fmt, ptr};

/// A [`HashSet<&'a T>`](HashSet) which compares its elements using the pointers' addresses
pub struct PtrSet<'a, T: ?Sized>(pub HashSet<ComparePtr<'a, T>>);

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for PtrSet<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a, T: ?Sized> PtrSet<'a, T> {
    /// Creates an empty `PtrSet`
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    /// Adds a value to the set.
    pub fn insert(&mut self, value: &'a T) -> bool {
        self.0.insert(ComparePtr(value))
    }

    /// An iterator visiting all elements in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = &'a T> + '_ {
        self.0.iter().map(|x| x.0)
    }
}

impl<'a, T: ?Sized> Extend<&'a T> for PtrSet<'a, T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.0.extend(iter.into_iter().map(ComparePtr))
    }
}

impl<'a, T: ?Sized> FromIterator<&'a T> for PtrSet<'a, T> {
    fn from_iter<I: IntoIterator<Item = &'a T>>(iter: I) -> Self {
        Self(HashSet::from_iter(iter.into_iter().map(ComparePtr)))
    }
}

/// Wrapper around `&'a T` which delegates comparison traits to the reference's pointer address
/// instead of the referenced instance of `T`.
pub struct ComparePtr<'a, T: ?Sized>(pub &'a T);
impl<'a, T: ?Sized> ComparePtr<'a, T> {
    fn into_ptr(self) -> *const T {
        self.0
    }
}
impl<'a, T: ?Sized> Eq for ComparePtr<'a, T> {}
impl<'a, T: ?Sized> PartialEq for ComparePtr<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.0, other.0)
    }
}
impl<'a, T: ?Sized> Hash for ComparePtr<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.into_ptr().hash(state)
    }
}
impl<'a, T: ?Sized> Ord for ComparePtr<'a, T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.into_ptr().cmp(&other.into_ptr())
    }
}
impl<'a, T: ?Sized> PartialOrd for ComparePtr<'a, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for ComparePtr<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl<'a, T: ?Sized> Copy for ComparePtr<'a, T> {}
impl<'a, T: ?Sized> Clone for ComparePtr<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}
