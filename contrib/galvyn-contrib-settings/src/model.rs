use galvyn_core::re_exports::rorm::Model;
use galvyn_core::re_exports::rorm::fields::types::Json;
use galvyn_core::re_exports::rorm::fields::types::MaxStr;
use galvyn_core::re_exports::schemars::_serde_json::value::RawValue;
use galvyn_core::re_exports::uuid::Uuid;

#[derive(Model)]
pub struct GalvynSettings {
    #[rorm(primary_key)]
    pub uuid: Uuid,

    #[rorm(unique)]
    pub key: MaxStr<255>,

    pub value: Json<Box<RawValue>>,
}
