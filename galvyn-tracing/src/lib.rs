use std::sync::OnceLock;

#[doc(hidden)]
pub use tracing as t;
#[doc(hidden)]
pub use tracing_core as tc;
use tracing_core::Callsite;
use tracing_core::Interest;
use tracing_core::Kind;
use tracing_core::Level;
use tracing_core::Metadata;
use tracing_core::field::Value;

pub type FieldName = const_str::ConstString<255>;

use tracing_core::callsite::DefaultCallsite;

#[macro_export]
macro_rules! span {
    ($lvl:expr, $name:expr $(, $field:ident = $value:expr)* $(,)?) => {{
        static __CALLSITE: $crate::tc::callsite::DefaultCallsite =
            $crate::tc::callsite::DefaultCallsite::new(&__META);
        static __FIELDS: ::std::sync::LazyLock<&'static [&'static str]> = ::std::sync::LazyLock::new(|| {
            todo!()
        });
        static __META: $crate::tc::Metadata<'static> = $crate::tc::metadata! {
            name: $name,
            target: ::std::module_path!(),
            level: $lvl,
            fields: &__FIELDS,
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
    use std::marker::PhantomData;
    use std::mem::ManuallyDrop;
    use std::ops::Deref;

    use tracing_core::field::Value;

    use crate::FieldName;
    use crate::MultiValue;
    use crate::const_str::ConstString;

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

    struct LateBind<'a, T>(PhantomData<&'a T>);
    impl<'a, T> LateBind<'a, T> {
        pub fn bind(&self, _value: &'a T) {}

        pub const fn push_names<const N: usize, W>(
            out: &mut [FieldName],
            idx: &mut usize,
            name: &'static str,
            _value: fn(T) -> W,
        ) {
        }
    }
}

pub mod const_str {

    /// A UTF-8–encoded, growable string for const expressions.
    ///
    /// ## Required invariant
    /// Every instance, which is observable from the public API,
    /// - must have valid utf8 in its bytes.
    /// - must have a valid utf8 boundary at len.
    #[derive(Copy, Clone)]
    pub struct ConstString<const MAX_LEN: usize> {
        len: u8,
        bytes: [u8; MAX_LEN],
    }

    impl<const MAX_LEN: usize> ConstString<MAX_LEN> {
        /// Creates a new empty `ConstString`.
        pub const fn new() -> Self {
            if MAX_LEN > u8::MAX as usize {
                panic!("ConstString only supports a MAX_LENGTH of at most 255");
            }
            Self {
                len: 0,
                bytes: [0; MAX_LEN],
            }
        }

        /// Returns the string's length
        pub const fn len(&self) -> usize {
            self.len as usize
        }

        /// Extracts a string slice containing the entire `ConstString`.
        pub const fn as_str(&self) -> &str {
            let bytes = self.as_bytes();

            // SAFETY:
            // `push_str` only adds a complete `str` or aborts.
            // Therefore whenever you've got an instance of `ConstString`, its bytes must form valid utf8
            unsafe { std::str::from_utf8_unchecked(bytes) }
        }

        /// Extracts a byte slice containing the entire `ConstString`.
        pub const fn as_bytes<'a>(&'a self) -> &'a [u8] {
            let mut len = self.len();
            if len > self.bytes.len() {
                len = self.bytes.len();
            }

            // SAFETY: - `&self.bytes` is a valid reference
            //         - `len` is not larger than `self.bytes.len()`
            //         - the lifetime `'a` is bound to the input and output
            unsafe { std::slice::from_raw_parts::<'a, u8>(self.bytes.as_ptr(), len) }
        }

        /// Appends a given string slice onto the end of this `ConstString`,
        ///
        /// # Errors
        /// if the resulting string would be larger than `MAX_LEN`.
        pub const fn try_push_str(&mut self, string: &str) -> Result<(), ()> {
            let mut bytes = string.as_bytes();
            while let [byte, remaining @ ..] = bytes {
                bytes = remaining;

                if self.len() == MAX_LEN {
                    return Err(());
                }

                self.bytes[self.len()] = *byte;
                self.len += 1;
            }
            Ok(())
        }
    }
}

static __CALLSITE: GalvynCallsite = GalvynCallsite::new();

struct GalvynCallsite {
    metadata: OnceLock<Metadata<'static>>,
}
struct GalvynCallsiteInner {}

impl GalvynCallsite {
    pub const fn new() -> Self {
        Self {
            metadata: OnceLock::new(),
        }
    }

    pub fn init(&'static self) {}
}
impl Callsite for GalvynCallsite {
    fn set_interest(&self, interest: Interest) {
        todo!()
    }

    fn metadata(&self) -> &Metadata<'_> {
        todo!()
    }
}
