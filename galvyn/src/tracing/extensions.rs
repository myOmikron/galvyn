//! Extension traits for some of `tracing`'s types

use std::fmt::Debug;

use tracing::field::Field;
use tracing::field::Visit;
use tracing::span;
use tracing::Event;
use tracing::Span;

/// Additional `record_...` methods for [`Span`] and [`Event`]
///
/// `Span` does not implement this trait directly because it is just a handle to the actual span data.
/// It is implemented on [`span::Attributes`] and [`span::Record`]
pub trait RecordExt {
    /// Visit all the fields of type `bool` with the provided closure
    fn record_bool(&self, visitor: impl FnMut(&Field, bool));
}
impl<'a> RecordExt for Event<'a> {
    fn record_bool(&self, visitor: impl FnMut(&Field, bool)) {
        self.record(&mut VisitBool(visitor));
    }
}
impl<'a> RecordExt for span::Attributes<'a> {
    fn record_bool(&self, visitor: impl FnMut(&Field, bool)) {
        self.record(&mut VisitBool(visitor));
    }
}
impl<'a> RecordExt for span::Record<'a> {
    fn record_bool(&self, visitor: impl FnMut(&Field, bool)) {
        self.record(&mut VisitBool(visitor));
    }
}

struct VisitBool<F>(F);
impl<F: FnMut(&Field, bool)> Visit for VisitBool<F> {
    fn record_bool(&mut self, field: &Field, value: bool) {
        (self.0)(field, value);
    }
    fn record_debug(&mut self, _field: &Field, _value: &dyn Debug) {}
}
