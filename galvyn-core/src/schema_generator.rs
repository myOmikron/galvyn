use schemars::JsonSchema;
use schemars::Map;
use schemars::r#gen::SchemaGenerator as InnerGenerator;
use schemars::r#gen::SchemaSettings;
use schemars::schema::InstanceType;
use schemars::schema::ObjectValidation;
use schemars::schema::Schema;
use schemars::schema::SchemaObject;
use schemars::schema::SingleOrVec;
use schemars::visit::Visitor;
use schemars::visit::visit_schema_object;
use serde_json::Value;

/// State for generating schemas from types implementing [`JsonSchema`]
///
/// If you require the underlying [`SchemaGenerator` from `schemars`](schemars::gen::SchemaGenerator),
/// you can use [`AsRef`] and [`AsMut`] to gain access.
pub struct SchemaGenerator(InnerGenerator);
impl AsRef<InnerGenerator> for SchemaGenerator {
    fn as_ref(&self) -> &InnerGenerator {
        &self.0
    }
}
impl AsMut<InnerGenerator> for SchemaGenerator {
    fn as_mut(&mut self) -> &mut InnerGenerator {
        &mut self.0
    }
}
impl Default for SchemaGenerator {
    fn default() -> Self {
        Self::new()
    }
}
impl SchemaGenerator {
    /// Constructs a new schema generator
    pub fn new() -> Self {
        let mut settings = SchemaSettings::openapi3();
        settings.visitors.push(Box::new(ReplaceNullType));
        Self(InnerGenerator::new(settings))
    }

    /// Generate an openapi schema for the type `T`
    ///
    /// This might do nothing but return a reference to the schema
    /// already added to the generator previously.
    pub fn generate<T: JsonSchema>(&mut self) -> Schema {
        let mut schema = self.0.subschema_for::<T>();
        for v in self.0.visitors_mut() {
            v.visit_schema(&mut schema);
        }
        schema
    }

    /// Generate an openapi schema for the type `T`
    ///
    /// Unlike [`SchemaGenerator::generate`], this method tries to return the actual schema instead
    /// of a reference.
    ///
    /// This depends on the implementor of `JsonSchema` for `T` to follow the behavior
    /// outlined in `JsonSchema`'s docs.
    /// Namely, [`JsonSchema::json_schema`] **should not** return a `$ref` schema.
    pub fn generate_refless<T: JsonSchema>(&mut self) -> Schema {
        let mut schema = T::json_schema(&mut self.0);
        for v in self.0.visitors_mut() {
            v.visit_schema(&mut schema);
        }
        schema
    }

    /// Generate an openapi schema of `"type": "object"`
    ///
    /// Returns `None` if `T` produced a schema of another type.
    ///
    /// This convenience method is used when `T` describes parameters for a handler and not a body.
    pub fn generate_object<T: JsonSchema>(&mut self) -> Option<Box<ObjectValidation>> {
        let schema = self.generate_refless::<T>();
        match schema {
            Schema::Object(SchemaObject {
                object: Some(object),
                ..
            }) => Some(object),
            _ => None,
        }
    }

    pub fn into_definitions(mut self) -> Map<String, Schema> {
        let mut definitions = self.0.take_definitions();
        for v in self.0.visitors_mut() {
            for s in definitions.values_mut() {
                v.visit_schema(s);
            }
        }
        definitions
    }
}

#[derive(Debug, Clone)]
struct ReplaceNullType;
impl Visitor for ReplaceNullType {
    fn visit_schema_object(&mut self, schema: &mut SchemaObject) {
        #[expect(clippy::collapsible_if, reason = "Only allowed on newer rust versions")]
        if let Some(SingleOrVec::Single(boxed)) = &schema.instance_type {
            if **boxed == InstanceType::Null {
                schema.instance_type = None;
                schema
                    .extensions
                    .insert("nullable".to_string(), Value::Bool(true));
            }
        }
        visit_schema_object(self, schema);
    }
}

/// Converts a schema into plain json
///
/// Tests should only compare schemas through this function
/// to be independent of `schemars`' internal representation.
#[cfg(test)]
pub(crate) fn schema_to_json(schema: &Schema) -> Value {
    serde_json::to_value(schema).expect("Schemas should always be serializable")
}

#[cfg(test)]
mod tests {
    use schemars::JsonSchema;
    use serde_json::json;

    use super::SchemaGenerator;
    use super::schema_to_json;

    /// A referenceable struct
    #[derive(JsonSchema)]
    #[allow(dead_code)]
    struct Refd {
        /// Some name
        name: String,
        maybe: Option<u32>,
        unit: (),
    }

    #[derive(JsonSchema)]
    #[serde(untagged)]
    #[allow(dead_code)]
    enum Untagged {
        Num(u32),
        Nothing,
    }

    #[derive(JsonSchema)]
    #[allow(dead_code)]
    enum Plain {
        A,
        B,
    }

    fn refd_json() -> serde_json::Value {
        json!({
            "description": "A referenceable struct",
            "type": "object",
            "required": ["name", "unit"],
            "properties": {
                "maybe": {"type": "integer", "format": "uint32", "minimum": 0.0, "nullable": true},
                "name": {"description": "Some name", "type": "string"},
                "unit": {"nullable": true}
            }
        })
    }

    #[test]
    fn generate_struct_is_ref() {
        let mut generator = SchemaGenerator::new();
        assert_eq!(
            schema_to_json(&generator.generate::<Refd>()),
            json!({"$ref": "#/components/schemas/Refd"})
        );

        let definitions = generator.into_definitions();
        assert_eq!(definitions.len(), 1);
        assert_eq!(schema_to_json(&definitions["Refd"]), refd_json());
    }

    #[test]
    fn generate_refless_struct_is_inline() {
        let mut generator = SchemaGenerator::new();
        assert_eq!(
            schema_to_json(&generator.generate_refless::<Refd>()),
            refd_json()
        );
        assert!(generator.into_definitions().is_empty());
    }

    #[test]
    fn generate_primitives() {
        let mut generator = SchemaGenerator::new();
        assert_eq!(
            schema_to_json(&generator.generate::<String>()),
            json!({"type": "string"})
        );
        assert_eq!(
            schema_to_json(&generator.generate::<u64>()),
            json!({"type": "integer", "format": "uint64", "minimum": 0.0})
        );
        assert_eq!(
            schema_to_json(&generator.generate::<Vec<String>>()),
            json!({"type": "array", "items": {"type": "string"}})
        );
        assert_eq!(
            schema_to_json(&generator.generate::<(uuid::Uuid, u32)>()),
            json!({
                "type": "array",
                "items": [
                    {"type": "string", "format": "uuid"},
                    {"type": "integer", "format": "uint32", "minimum": 0.0}
                ],
                "maxItems": 2,
                "minItems": 2
            })
        );
        assert!(generator.into_definitions().is_empty());
    }

    #[test]
    fn null_type_is_replaced_by_nullable() {
        let mut generator = SchemaGenerator::new();
        assert_eq!(
            schema_to_json(&generator.generate::<()>()),
            json!({"nullable": true})
        );
    }

    #[test]
    fn option_is_nullable() {
        let mut generator = SchemaGenerator::new();
        assert_eq!(
            schema_to_json(&generator.generate::<Option<String>>()),
            json!({"type": "string", "nullable": true})
        );
        assert_eq!(
            schema_to_json(&generator.generate::<Option<u64>>()),
            json!({"type": "integer", "format": "uint64", "minimum": 0.0, "nullable": true})
        );
        assert_eq!(
            schema_to_json(&generator.generate::<Option<Refd>>()),
            json!({"allOf": [{"$ref": "#/components/schemas/Refd"}], "nullable": true})
        );
    }

    #[test]
    fn null_type_in_untagged_enum() {
        let mut generator = SchemaGenerator::new();
        let expected = json!({
            "anyOf": [
                {"type": "integer", "format": "uint32", "minimum": 0.0},
                {"nullable": true}
            ]
        });
        assert_eq!(
            schema_to_json(&generator.generate_refless::<Untagged>()),
            expected
        );

        assert_eq!(
            schema_to_json(&generator.generate::<Untagged>()),
            json!({"$ref": "#/components/schemas/Untagged"})
        );
        let definitions = generator.into_definitions();
        assert_eq!(schema_to_json(&definitions["Untagged"]), expected);
    }

    #[test]
    fn null_type_is_replaced_in_definitions() {
        let mut generator = SchemaGenerator::new();
        generator.generate::<Option<Refd>>();
        let definitions = generator.into_definitions();
        assert_eq!(schema_to_json(&definitions["Refd"]), refd_json());
    }

    #[test]
    fn generate_object_for_struct() {
        let mut generator = SchemaGenerator::new();
        let object = generator
            .generate_object::<Refd>()
            .expect("Refd is an object");
        assert_eq!(
            serde_json::to_value(object).unwrap(),
            json!({
                "required": ["name", "unit"],
                "properties": {
                    "maybe": {"type": "integer", "format": "uint32", "minimum": 0.0, "nullable": true},
                    "name": {"description": "Some name", "type": "string"},
                    "unit": {"nullable": true}
                }
            })
        );
    }

    #[test]
    fn generate_object_for_non_objects() {
        let mut generator = SchemaGenerator::new();
        assert!(generator.generate_object::<String>().is_none());
        assert!(generator.generate_object::<Vec<String>>().is_none());
        assert!(generator.generate_object::<Plain>().is_none());
        assert!(generator.generate_object::<Option<Refd>>().is_none());
    }
}
