use galvyn_core::re_exports::rorm::internal::field::Field;
use galvyn_core::re_exports::rorm::Model;
use webauthn_rs::prelude::Uuid;

#[cfg(feature = "__local-user")]
pub trait LocalUserAccess<U>
where
    U: Model,
    <U as Model>::Primary: Field<Type = Uuid>,
{
    fn get_primary(&self) -> Uuid {}
}
