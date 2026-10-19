use rorm::Row;
use rorm::conditions::Value;
use rorm::crud::decoder::Decoder;
use rorm::db::row::RowError;
use rorm::db::sql::value::NullType;
use rorm::fields::proxy;
use rorm::fields::proxy::FieldProxy;
use rorm::fields::proxy::FieldProxyImpl;
use rorm::fields::traits::FieldColumns;
use rorm::fields::traits::FieldType;
use rorm::fields::traits::into_value::IntoValue;
use rorm::fields::traits::simple::SimpleFieldEq;
use rorm::fields::traits::simple::SimpleFieldIn;
use rorm::fields::traits::simple::SimpleFieldLike;
use rorm::fields::traits::simple::SimpleFieldOrd;
use rorm::internal::field::Field;
use rorm::internal::field::decoder::FieldDecoder;
use rorm::internal::field::fake_field::FakeField;
use rorm::internal::query_context::QueryContext;

use crate::misc::redacted::Redacted;

impl<T: FieldType> FieldType for Redacted<T> {
    type Columns = T::Columns;
    const NULL: FieldColumns<Self, NullType> = T::NULL;

    fn into_values<'a>(self) -> FieldColumns<Self, Value<'a>> {
        self.sensitive.into_values()
    }

    fn as_values(&self) -> FieldColumns<Self, Value<'_>> {
        self.sensitive.as_values()
    }

    type Decoder = RedactedDecoder<T>;
    type GetNames = T::GetNames;
    type GetAnnotations = T::GetAnnotations;
    type Check = T::Check;
}

impl<'a, T: IntoValue<'a>> IntoValue<'a> for Redacted<T> {
    fn into_value(self) -> Value<'a> {
        self.sensitive.into_value()
    }
}

impl<Rhs, T: SimpleFieldEq<Rhs>> SimpleFieldEq<Redacted<Rhs>> for Redacted<T> {}
impl<Rhs, T: SimpleFieldOrd<Rhs>> SimpleFieldOrd<Redacted<Rhs>> for Redacted<T> {}
impl<Rhs, T: SimpleFieldLike<Rhs>> SimpleFieldLike<Redacted<Rhs>> for Redacted<T> {}
impl<Rhs, T: SimpleFieldIn<Rhs>> SimpleFieldIn<Redacted<Rhs>> for Redacted<T> {}

pub struct RedactedDecoder<T: FieldType>(T::Decoder);
impl<T: FieldType> Decoder for RedactedDecoder<T> {
    type Result = Redacted<T>;

    fn by_name<'index>(&'index self, row: &'_ Row) -> Result<Self::Result, RowError<'index>> {
        self.0.by_name(row).map(Redacted::new)
    }

    fn by_index<'index>(&'index self, row: &'_ Row) -> Result<Self::Result, RowError<'index>> {
        self.0.by_index(row).map(Redacted::new)
    }
}
impl<T: FieldType> FieldDecoder for RedactedDecoder<T> {
    fn new<I>(ctx: &mut QueryContext, _: FieldProxy<I>) -> Self
    where
        I: FieldProxyImpl<Field: Field<Type = Self::Result>>,
    {
        Self(<T::Decoder as FieldDecoder>::new::<(
            FakeField<T, I::Field>,
            I::Path,
        )>(ctx, proxy::new()))
    }
}
