//! `futures_core::stream` implementations for `Either`

use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_lite::Stream;

use crate::misc::either::Either;

impl<L, R> Stream for Either<L, R>
where
    L: Stream,
    R: Stream,
{
    type Item = Either<L::Item, R::Item>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.as_pin_mut() {
            Either::Left(x) => x.poll_next(cx).map(|x| x.map(Either::Left)),
            Either::Right(x) => x.poll_next(cx).map(|x| x.map(Either::Right)),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Either::Left(x) => x.size_hint(),
            Either::Right(x) => x.size_hint(),
        }
    }
}
