use galvyn_core::re_exports::schemars::_serde_json::value::RawValue;
use galvyn_core::re_exports::uuid::Uuid;
use rorm::Model;
use rorm::fields::types::Json;
use rorm::fields::types::MaxStr;

#[derive(Model)]
pub struct GalvynSettings {
    #[rorm(primary_key)]
    pub uuid: Uuid,

    #[rorm(unique)]
    pub key: MaxStr<255>,

    pub value: Json<Box<RawValue>>,
}
