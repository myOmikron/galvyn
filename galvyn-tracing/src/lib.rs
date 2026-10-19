#[doc(hidden)]
pub use tracing as t;
#[doc(hidden)]
pub use tracing_core as tc;
use tracing_core::field::Value;

#[macro_export]
macro_rules! span {
    ($lvl:expr, $name:expr $(, $field:ident = $value:expr)* $(,)?) => {{
        static __CALLSITE: $crate::tc::callsite::DefaultCallsite =
            $crate::tc::callsite::DefaultCallsite::new(&__META);
        static __META: $crate::tc::Metadata<'static> = $crate::tc::metadata! {
            name: $name,
            target: ::std::module_path!(),
            level: $lvl,
            fields: &[$(stringify!($field),)*], // TODO
            callsite: &__CALLSITE,
            kind: $crate::tc::Kind::SPAN,
        };
        let mut values: ::std::vec::Vec<::std::option::Option<&dyn $crate::tc::field::Value>> = vec![];
        $(
            $crate::macro_helper::PushValue::new(&$value).push_values(&mut values);
        )*
        $crate::t::Span::new(&__META, &__META.fields().value_set_all(&values))
    }};
}

#[macro_export]
macro_rules! info_span {
    ($name:expr $(, $($passthrough:tt)*)?) => {
        $crate::span!($crate::t::Level::INFO, $name $(, $($passthrough)*)?)
    };
}

fn test() {
    struct Foo;
    struct Bar;
    impl MultiValue for Foo {
        const FIELDS: &[&str] = &["int", "bool"];
        fn push_values(&self, values: &mut Vec<Option<&dyn Value>>) {
            values.push(Some(&1));
            values.push(Some(&true));
        }
    }

    let span = info_span!("test", foo = Foo);
}

trait MultiValue {
    const FIELDS: &[&str];
    fn push_values(&self, values: &mut Vec<Option<&dyn Value>>);
}
impl<T: MultiValue> MultiValue for &'_ T {
    const FIELDS: &'static [&'static str] = Self::FIELDS;

    fn push_values(&self, values: &mut Vec<Option<&dyn Value>>) {
        T::push_values(self, values)
    }
}
impl<T: MultiValue> MultiValue for &'_ mut T {
    const FIELDS: &'static [&'static str] = Self::FIELDS;

    fn push_values(&self, values: &mut Vec<Option<&dyn Value>>) {
        T::push_values(self, values)
    }
}

#[doc(hidden)]
pub mod macro_helper {
    use std::ops::Deref;

    use tracing_core::field::Value;

    use crate::MultiValue;

    pub struct PushValue<'a, T>(PushValueSpec<'a, T>);
    pub struct PushValueSpec<'a, T>(PushValueFallback<'a, T>);
    pub struct PushValueFallback<'a, T>(&'a T);

    impl<'a, T: MultiValue> PushValueSpec<'a, T> {
        pub fn push_values(&self, values: &mut Vec<Option<&'a dyn Value>>) {
            self.0.0.push_values(values)
        }
    }
    impl<'a, T> PushValueFallback<'a, T> {
        pub fn push_values(&self, values: &mut Vec<Option<&'a dyn Value>>)
        where
            T: Value,
        {
            values.push(Some(self.0));
        }
    }

    impl<'a, T> PushValue<'a, T> {
        pub const fn new(value: &'a T) -> Self {
            PushValue(PushValueSpec(PushValueFallback(value)))
        }
    }
    impl<'a, T> Deref for PushValue<'a, T> {
        type Target = PushValueSpec<'a, T>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<'a, T> Deref for PushValueSpec<'a, T> {
        type Target = PushValueFallback<'a, T>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
}
