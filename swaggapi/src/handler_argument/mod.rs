//! The [`HandlerArgument`] trait, its implementations and utilises for implementing it.

mod impls;
mod utils;

use indexmap::IndexMap;
use mime::Mime;
use openapiv3::MediaType;
use openapiv3::Parameter;
use openapiv3::ReferenceOr;
use openapiv3::RequestBody;
use openapiv3::Schema;

use crate::internals::SchemaGenerator;

/// Marker trait
pub trait ShouldBeHandlerArgument {}

/// A type used as argument to a handler which can be described
/// by a [request body object](https://spec.openapis.org/oas/v3.0.3#request-body-object)
/// or a [parameter object](https://spec.openapis.org/oas/v3.0.3#parameter-object)
///
/// This type should be implemented by everything which implements
/// [`FromRequest`](::axum::extract::FromRequest) / [`FromRequestParts`](::axum::extract::FromRequestParts) when using [axum](::axum) or
/// [`FromRequest`] when using [actix]
pub trait HandlerArgument: ShouldBeHandlerArgument {
    /// Get the [request body object](https://spec.openapis.org/oas/v3.0.3#request-body-object) describing `Self`
    ///
    /// Should return `None` if `Self` doesn't consume the request body
    fn request_body(_gen: &mut SchemaGenerator) -> Option<RequestBody> {
        None
    }

    /// Get the [parameter objects](https://spec.openapis.org/oas/v3.0.3#parameter-object) describing `Self`
    ///
    /// Should return an empty `Vec` if `Self` doesn't parse any parameters
    fn parameters(_gen: &mut SchemaGenerator, _path: &[&str]) -> Vec<Parameter> {
        Vec::new()
    }
}

/// Struct representation of a [`HandlerArgument`]
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct HandlerArgumentFns {
    pub(crate) request_body: fn(&mut SchemaGenerator) -> Option<RequestBody>,
    pub(crate) parameters: fn(&mut SchemaGenerator, &[&str]) -> Vec<Parameter>,
}

/// Helper function for building a simple [`RequestBody`]
pub fn simple_request_body(request_body: SimpleRequestBody) -> RequestBody {
    RequestBody {
        content: IndexMap::<_, _>::from_iter([(
            request_body.mime_type.to_string(),
            MediaType {
                schema: request_body.schema,
                ..Default::default()
            },
        )]),
        required: true,
        ..Default::default()
    }
}

/// Describes the response for a specific status code
pub struct SimpleRequestBody {
    /// The request body's mime type
    pub mime_type: Mime,

    /// Optional schema
    pub schema: Option<ReferenceOr<Schema>>,
}

#[doc(hidden)]
pub mod macro_helper {
    use std::marker::PhantomData;
    use std::ops::Deref;

    use super::HandlerArgument;
    use super::HandlerArgumentFns;
    use super::ShouldBeHandlerArgument;

    /// Feed the result of `probe.get_handler_argument` to this function to check
    /// if [`HandlerArgument`] is implemented for a [`ShouldBeHandlerArgument`] type
    pub const fn check_handler_argument<T: HandlerArgument>(_: fn() -> PhantomData<T>) {}
    pub const fn get_handler_argument_fns<T: HandlerArgument, IsHandler: Boolean>(
        _: fn() -> PhantomData<T>,
        _: fn() -> IsHandler,
    ) -> Option<HandlerArgumentFns> {
        if IsHandler::VALUE {
            Some(HandlerArgumentFns {
                request_body: T::request_body,
                parameters: T::parameters,
            })
        } else {
            None
        }
    }

    impl<T> TraitProbe<T>
    where
        T: ShouldBeHandlerArgument,
    {
        pub fn should_be_handler_argument(&self) -> bool {
            true
        }

        pub fn get_handler_argument(&self) -> PhantomData<T> {
            PhantomData
        }
    }

    impl<T> TraitProbe<T>
    where
        T: HandlerArgument,
    {
        pub fn is_handler_argument(&self) -> True {
            True
        }
    }

    impl Else {
        pub fn should_be_handler_argument(&self) -> bool {
            false
        }

        pub fn get_handler_argument(&self) -> PhantomData<NotAnArgument> {
            PhantomData
        }

        pub fn is_handler_argument(&self) -> False {
            False
        }
    }

    pub struct TraitProbe<T>(PhantomData<T>);

    impl<T> TraitProbe<T> {
        pub const fn new() -> Self {
            Self(PhantomData)
        }
    }

    pub struct Else;

    impl<T> Deref for TraitProbe<T> {
        type Target = Else;

        fn deref(&self) -> &Self::Target {
            static ELSE: Else = Else;
            &ELSE
        }
    }

    pub struct NotAnArgument;

    impl ShouldBeHandlerArgument for NotAnArgument {}

    impl HandlerArgument for NotAnArgument {}

    pub struct True;
    pub struct False;
    pub trait Boolean {
        const VALUE: bool;
        fn value(&self) -> bool {
            Self::VALUE
        }
    }
    impl Boolean for True {
        const VALUE: bool = true;
    }
    impl Boolean for False {
        const VALUE: bool = false;
    }
}
