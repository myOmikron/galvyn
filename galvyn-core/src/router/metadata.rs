//! A trait and collection for user-defined metadata that can be attached to a `Route`.

use std::any::Any;
use std::fmt;
use std::ops::ControlFlow;

/// A trait for user-defined metadata that can be attached to a `Route`.
///
/// # How to use
/// 1. A third-party (non-`galvyn-core`) library defines custom metadata by implementing this trait.
/// 2. This metadata can be associated with routes by the application author.
///     - Generally by calling [`GalvynRouter::metadata`](crate::GalvynRouter::metadata)
///     - A library is recommended to provide an extension trait for [`GalvynRouter`](crate::GalvynRouter)
///       which implements convenience methods for attaching their metadata to the router.
///     - It is recommended that a library provides an extension trait for [`GalvynRouter`](crate::GalvynRouter).
///       This trait should provide convenient methods for attaching the library's metadata to the router.
/// 3. The library provides some useful feature by inspecting all routes' metadata.
///
/// # Example
///
/// The `openapi` module of `galvyn` implements the `OpenapiMetadata` struct which implements `RouteMetadata`.
/// It associates tags, as used in openapi, with routes.
/// The trait `OpenapiRouterExt` implements the method `openapi_tag`
/// which hides the entire metadata system from the application author:
///
/// ```no_run
/// use galvyn::Galvyn;
/// use galvyn::core::GalvynRouter;
/// use galvyn::openapi::OpenapiRouterExt;
/// use galvyn::openapi::OpenapiMetadata;
///
/// Galvyn::new()
///     .init_modules()
///     .await?
///     .add_routes(
///         GalvynRouter::new()
///             // With extension trait
///             .openapi_tag("My Tag")
///     )
///     .add_routes(
///         GalvynRouter::new()
///             // Without extension trait
///             .metadata(OpenapiMetadata {
///                 tags: vec!["Another Tag"],
///                 ..Default::default()
///             })
///     )
///     .start("127.0.0.1:8080".parse()?)
///     .await?;
///
/// // The outputed json will include the tags defined above with their
/// // associated routes. (This code doesn't include any routes)
/// galvyn::openapi::get_openapi();
/// ```
pub trait RouteMetadata: fmt::Debug + Clone + Send + Sync + 'static {
    /// Merges another instance into `self`
    fn merge(&mut self, other: &Self);
}

/// A set of type-erased [`RouteMetadata`].
///
/// This set stores at most one instance of each type implementing `RouteMetadata`.
///
/// If [`RouteMetadataSet::insert`] is called twice with the same metadata type
/// then the second instance is merged into the first one using [`RouteMetadata::merge`].
#[derive(Debug, Default)]
pub struct RouteMetadataSet {
    // This could be changed to a map if performance would require it
    extensions: Vec<Box<dyn DynRouteMetadata>>,
}

impl RouteMetadataSet {
    /// Inserts some [`RouteMetadata`].
    ///
    /// If there is already an instance of type `T` in this set
    /// then the `value` will be merged into the first instance using [`RouteMetadata::merge`].
    pub fn insert<T: RouteMetadata>(&mut self, value: T) {
        for ext in &mut self.extensions {
            if let Some(ext) = ext.as_mut_dyn().downcast_mut::<T>() {
                ext.merge(&value);
                return;
            }
        }
        self.extensions.push(Box::new(value) as _);
    }

    /// Merges another `RouteMetadataSet` into `self`.
    ///
    /// If both sets contain metadata of the same type
    /// then the instance of `other` will be merged into the instance of `self`
    /// using [`RouteMetadata::merge`].
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

    /// Retrieves some `RouteMetadata` of a specific type.
    pub fn get<T: RouteMetadata>(&self) -> Option<&T> {
        for ext in &self.extensions {
            if let Some(ext) = ext.as_dyn().downcast_ref::<T>() {
                return Some(ext);
            }
        }
        None
    }
}

/// Auto-implemented clone of [`RouteMetadata`] which is dyn compatible.
///
/// This trait is tailored to the usage of `RouteMetadata` in [`RouteMetadataSet`].
trait DynRouteMetadata: fmt::Debug + Send + Sync + 'static {
    /// Merges `other` into `self` if `other` is of type `Self`
    fn merge_maybe(&mut self, other: &dyn DynRouteMetadata) -> ControlFlow<()>;

    /// Clones `self` into a new `Box`
    fn clone_boxed(&self) -> Box<dyn DynRouteMetadata>;

    /// Casts `self` into a `&dyn Any`
    fn as_dyn(&self) -> &(dyn Any + Send + Sync + 'static);

    /// Casts `self` into a `&mut dyn Any`
    fn as_mut_dyn(&mut self) -> &mut (dyn Any + Send + Sync + 'static);
}

impl<T: RouteMetadata> DynRouteMetadata for T {
    fn merge_maybe(&mut self, other: &dyn DynRouteMetadata) -> ControlFlow<()> {
        match other.as_dyn().downcast_ref::<Self>() {
            Some(other) => {
                self.merge(other);
                ControlFlow::Break(())
            }
            None => ControlFlow::Continue(()),
        }
    }

    fn clone_boxed(&self) -> Box<dyn DynRouteMetadata> {
        Box::new(self.clone()) as _
    }

    fn as_dyn(&self) -> &(dyn Any + Send + Sync + 'static) {
        self as _
    }

    fn as_mut_dyn(&mut self) -> &mut (dyn Any + Send + Sync + 'static) {
        self as _
    }
}
