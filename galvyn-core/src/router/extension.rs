use std::any::Any;
use std::fmt;
use std::ops::ControlFlow;

/// A collection of type-erased [`RouteExtension`]s
///
/// This collection stores at most one instance of each type implementing `RouteExtension`.
///
/// If [`RouteExtensions::insert`] is called twice with the same `RouteExtension` type
/// then the second instance is merged into the first one using [`RouteExtension::merge`].
#[derive(Debug, Default)]
pub struct RouteExtensions {
    // This could be changed to a map if performance would require it
    extensions: Vec<Box<dyn DynRouteExtension>>,
}

impl RouteExtensions {
    /// Inserts a `RouteExtension`
    ///
    /// If there is already an instance of type `T` in this collection
    /// then the `value` will be merged into the first instance using [`RouteExtension::merge`].
    pub fn insert<T: RouteExtension>(&mut self, value: T) {
        for ext in &mut self.extensions {
            if let Some(ext) = ext.as_mut_dyn().downcast_mut::<T>() {
                ext.merge(&value);
                return;
            }
        }
        self.extensions.push(Box::new(value) as _);
    }

    /// Merges another `RouteExtensions` into `self`.
    ///
    /// If both collections contain an instance of the same `RouteExtension` type
    /// then the instance of `other` will be merged into the instance of `self`
    /// using [`RouteExtension::merge`].
    pub fn merge(&mut self, other: &Self) {
        for other_ext in &other.extensions {
            for self_ext in &mut self.extensions {
                if self_ext.merge_maybe(&**other_ext).is_break() {
                    break;
                }
            }
            self.extensions.push(other_ext.clone_boxed());
        }
    }

    /// Retrieves a specific `RouteExtension`.
    pub fn get<T: RouteExtension>(&self) -> Option<&T> {
        for ext in &self.extensions {
            if let Some(ext) = ext.as_dyn().downcast_ref::<T>() {
                return Some(ext);
            }
        }
        None
    }

    /// Iterates over all `RouteExtension`s.
    pub fn iter(&self) -> impl Iterator<Item = &(dyn Any + Send + Sync + 'static)> {
        self.extensions.iter().map(|ext| ext.as_dyn())
    }
}

/// An arbitrary type which represents additional metadata attached to a [`Route`](super::Route)
pub trait RouteExtension: fmt::Debug + Clone + Send + Sync + 'static {
    /// Merges another instance into `self`
    fn merge(&mut self, other: &Self);
}

/// Auto-implemented clone of [`RouteExtension`] which is dyn compatible.
///
/// This trait is tailored to the usage of `RouteExtension` in [`RouteExtensions`].
trait DynRouteExtension: fmt::Debug + Send + Sync + 'static {
    /// Merges `other` into `self` if `other` is of type `Self`
    fn merge_maybe(&mut self, other: &dyn DynRouteExtension) -> ControlFlow<()>;

    fn clone_boxed(&self) -> Box<dyn DynRouteExtension>;

    /// Casts `self` into a `&dyn Any`
    fn as_dyn(&self) -> &(dyn Any + Send + Sync + 'static);

    /// Casts `self` into a `&mut dyn Any`
    fn as_mut_dyn(&mut self) -> &mut (dyn Any + Send + Sync + 'static);
}

impl<T: RouteExtension> DynRouteExtension for T {
    fn merge_maybe(&mut self, other: &dyn DynRouteExtension) -> ControlFlow<()> {
        match other.as_dyn().downcast_ref::<Self>() {
            Some(other) => {
                self.merge(other);
                ControlFlow::Break(())
            }
            None => ControlFlow::Continue(()),
        }
    }

    fn clone_boxed(&self) -> Box<dyn DynRouteExtension> {
        Box::new(self.clone()) as _
    }

    fn as_dyn(&self) -> &(dyn Any + Send + Sync + 'static) {
        self as _
    }

    fn as_mut_dyn(&mut self) -> &mut (dyn Any + Send + Sync + 'static) {
        self as _
    }
}
