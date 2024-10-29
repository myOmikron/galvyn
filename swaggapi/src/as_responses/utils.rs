use openapiv3::MediaType;
use openapiv3::ReferenceOr;
use openapiv3::Responses;
use openapiv3::Schema;
use openapiv3::SchemaKind;
use openapiv3::StatusCode;
use serde::Serialize;

use crate::as_responses::simple_responses;
use crate::as_responses::AsResponses;
use crate::as_responses::SimpleResponse;
use crate::internals::SchemaGenerator;
use crate::utils::SchemalessJson;

impl<T: Serialize> AsResponses for SchemalessJson<T> {
    fn responses(_: &mut SchemaGenerator) -> Responses {
        simple_responses([
            SimpleResponse {
                status_code: StatusCode::Code(200),
                mime_type: mime::APPLICATION_JSON,
                description: "Some json data".to_string(),
                media_type: Some(MediaType {
                    schema: Some(ReferenceOr::Item(Schema {
                        schema_data: Default::default(),
                        schema_kind: SchemaKind::Any(Default::default()),
                    })),
                    ..Default::default()
                }),
            },
            // TODO add error
        ])
    }
}
