use openapiv3::ReferenceOr;
use openapiv3::RequestBody;
use openapiv3::Schema;
use openapiv3::SchemaKind;
use serde::de::DeserializeOwned;

use crate::handler_argument::simple_request_body;
use crate::handler_argument::HandlerArgument;
use crate::handler_argument::ShouldBeHandlerArgument;
use crate::handler_argument::SimpleRequestBody;
use crate::internals::SchemaGenerator;
use crate::utils::SchemalessJson;

impl<T: DeserializeOwned> ShouldBeHandlerArgument for SchemalessJson<T> {}
impl<T: DeserializeOwned> HandlerArgument for SchemalessJson<T> {
    fn request_body(_: &mut SchemaGenerator) -> Option<RequestBody> {
        Some(simple_request_body(SimpleRequestBody {
            mime_type: mime::APPLICATION_JSON,
            schema: Some(ReferenceOr::Item(Schema {
                schema_data: Default::default(),
                schema_kind: SchemaKind::Any(Default::default()),
            })),
        }))
    }
}
