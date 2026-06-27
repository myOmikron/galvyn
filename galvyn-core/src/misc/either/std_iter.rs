//! Iterator implementations for `Either`

use std::iter::FusedIterator;

use crate::misc::either::Either;

impl<L, R> IntoIterator for Either<L, R>
where
    L: IntoIterator,
    R: IntoIterator,
{
    type Item = Either<L::Item, R::Item>;
    type IntoIter = Iter<L::IntoIter, R::IntoIter>;
    fn into_iter(self) -> Self::IntoIter {
        Iter(match self {
            Either::Left(x) => Either::Left(x.into_iter()),
            Either::Right(x) => Either::Right(x.into_iter()),
        })
    }
}

pub struct Iter<L, R>(pub Either<L, R>);

impl<L, R> Iterator for Iter<L, R>
where
    L: Iterator,
    R: Iterator,
{
    type Item = Either<L::Item, R::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            Either::Left(x) => x.next().map(Either::Left),
            Either::Right(x) => x.next().map(Either::Right),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            Either::Left(x) => x.size_hint(),
            Either::Right(x) => x.size_hint(),
        }
    }
}

impl<L, R> DoubleEndedIterator for Iter<L, R>
where
    L: DoubleEndedIterator,
    R: DoubleEndedIterator,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            Either::Left(x) => x.next_back().map(Either::Left),
            Either::Right(x) => x.next_back().map(Either::Right),
        }
    }
}

impl<L, R> FusedIterator for Iter<L, R>
where
    L: FusedIterator,
    R: FusedIterator,
{
}

impl<L, R> ExactSizeIterator for Iter<L, R>
where
    L: ExactSizeIterator,
    R: ExactSizeIterator,
{
}
