use std::marker::PhantomData;
use std::ops::Deref;

pub trait HasMetadata<M> {
    fn metadata() -> M;
}
pub trait ShouldHaveMetadata<M> {}

#[doc(hidden)]
#[macro_export]
macro_rules! get_metadata {
    ($M:ty, $T:ty) => {{
        $crate::macro_utils::type_metadata::If::<$M, $T>::new().get_metadata()
    }};
}
pub use crate::get_metadata;

impl<M, T> If<M, T>
where
    T: ShouldHaveMetadata<M>,
{
    pub fn get_metadata(&self) -> Option<M>
    where
        T: HasMetadata<M>,
    {
        Some(T::metadata())
    }
}

impl<M> Else<M> {
    pub fn get_metadata(&self) -> Option<M> {
        None
    }
}

pub struct If<M, T> {
    metadata: PhantomData<M>,
    r#type: PhantomData<T>,
    r#else: Else<M>,
}
impl<M, T> If<M, T> {
    pub const fn new() -> Self {
        Self {
            metadata: PhantomData,
            r#type: PhantomData,
            r#else: Else {
                metadata: PhantomData,
            },
        }
    }
}
pub struct Else<M> {
    metadata: PhantomData<M>,
}
impl<M, T> Deref for If<M, T> {
    type Target = Else<M>;

    fn deref(&self) -> &Self::Target {
        &self.r#else
    }
}
