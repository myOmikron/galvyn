//! `std::future` implementations for `Either`

use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use pin_project_lite::pin_project;

use crate::misc::either::Either;

impl<L, R> IntoFuture for Either<L, R>
where
    L: IntoFuture,
    R: IntoFuture,
{
    type Output = Either<L::Output, R::Output>;
    type IntoFuture = Fut<L::IntoFuture, R::IntoFuture>;

    fn into_future(self) -> Self::IntoFuture {
        match self {
            Either::Left(x) => Fut {
                inner: Either::Left(x.into_future()),
            },
            Either::Right(x) => Fut {
                inner: Either::Right(x.into_future()),
            },
        }
    }
}

pin_project! {
    pub struct Fut<L, R> {
        #[pin]
        pub inner: Either<L, R>,
    }
}

impl<L, R> Future for Fut<L, R>
where
    L: Future,
    R: Future,
{
    type Output = Either<L::Output, R::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().inner.as_pin_mut() {
            Either::Left(x) => x.poll(cx).map(Either::Left),
            Either::Right(x) => x.poll(cx).map(Either::Right),
        }
    }
}
