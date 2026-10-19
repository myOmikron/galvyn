use schemars::JsonSchema;
use schemars::Schema;
use schemars::SchemaGenerator as InnerGenerator;
use schemars::generate::SchemaSettings;
use schemars::transform::Transform;
use schemars::transform::transform_subschemas;
use serde_json::Value;
use serde_json::json;

/// State for generating schemas from types implementing [`JsonSchema`]
///
/// If you require the underlying [`SchemaGenerator` from `schemars`](schemars::SchemaGenerator),
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
        Self(InnerGenerator::new(
            SchemaSettings::openapi3().with_transform(ReplaceNullType),
        ))
    }

    /// Generate an openapi schema for the type `T`
    ///
    /// This might do nothing but return a reference to the schema
    /// already added to the generator previously.
    pub fn generate<T: JsonSchema>(&mut self) -> Schema {
        let mut schema = self.0.subschema_for::<T>();
        for v in self.0.transforms_mut() {
            v.transform(&mut schema);
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
        for v in self.0.transforms_mut() {
            v.transform(&mut schema);
        }
        schema
    }

    /// Generate an openapi schema of `"type": "object"`
    ///
    /// Returns `None` if `T` produced a schema of another type.
    ///
    /// This convenience method is used when `T` describes parameters for a handler and not a body.
    pub fn generate_object<T: JsonSchema>(
        &mut self,
    ) -> Option<serde_json::Map<String, serde_json::Value>> {
        let schema = self.generate_refless::<T>().to_object()?;
        if schema.get("type").is_some_and(|x| x == "object") {
            Some(schema)
        } else {
            None
        }
    }

    pub fn into_definitions(mut self) -> serde_json::Map<String, serde_json::Value> {
        self.0.take_definitions(
            // apply-transforms:
            true,
        )
    }
}

/// Restores the representation of `null` used by `schemars` v0.8
///
/// `schemars` v1 represents `null` as `{"nullable": true, "enum": [null]}`
/// and `Option<T>` (with `T` being a `$ref`) as `{"anyOf": [{"$ref": ...}, <null>]}`.
///
/// Both are valid openapi 3.0, but `openapi-generator` doesn't handle them well.
/// For example, it duplicates referenced enums inline (adding a `'null'` string variant in query parameters)
/// and drops the nullability of array items.
///
/// Therefore, this transform rewrites them to `{"nullable": true}`
/// and `{"allOf": [{"$ref": ...}], "nullable": true}` respectively.
///
/// It has to run after `schemars`' own openapi 3.0 transforms.
#[derive(Debug, Clone)]
struct ReplaceNullType;
impl Transform for ReplaceNullType {
    fn transform(&mut self, schema: &mut Schema) {
        transform_subschemas(self, schema);

        let Some(object) = schema.as_object_mut() else {
            return;
        };

        if object.get("nullable") == Some(&Value::Bool(true))
            && !object.contains_key("type")
            && object.get("enum") == Some(&json!([null]))
        {
            object.remove("enum");
        }

        if object.contains_key("allOf") {
            return;
        }
        let Some(Value::Array(any_of)) = object.get("anyOf") else {
            return;
        };
        let null = json!({"nullable": true});
        let reference = match any_of.as_slice() {
            [reference, other] | [other, reference] if *other == null && is_ref(reference) => {
                reference.clone()
            }
            _ => return,
        };
        object.remove("anyOf");
        object.insert("allOf".to_string(), Value::Array(vec![reference]));
        object.insert("nullable".to_string(), Value::Bool(true));
    }
}

/// Checks whether a schema consists of nothing but a `$ref`
fn is_ref(schema: &Value) -> bool {
    schema
        .as_object()
        .is_some_and(|object| object.len() == 1 && object.contains_key("$ref"))
}

pub trait SchemaExt {
    /// Owned version of [`Schema::as_object`] or [`serde_json::Value::as_object`]
    fn to_object(self) -> Option<serde_json::Map<String, serde_json::Value>>;

    /// Owned version of [`Schema::get`] or [`serde_json::Value::get`]
    fn get_owned(self, key: &str) -> Option<serde_json::Value>;
}
impl SchemaExt for Schema {
    fn to_object(self) -> Option<serde_json::Map<String, serde_json::Value>> {
        self.to_value().to_object()
    }

    fn get_owned(self, key: &str) -> Option<serde_json::Value> {
        self.to_value().get_owned(key)
    }
}
impl SchemaExt for serde_json::Value {
    fn to_object(self) -> Option<serde_json::Map<String, serde_json::Value>> {
        if let Self::Object(x) = self {
            Some(x)
        } else {
            None
        }
    }

    fn get_owned(self, key: &str) -> Option<serde_json::Value> {
        self.to_object().and_then(|mut x| x.remove(key))
    }
}

/// Converts a schema into plain json
///
/// Tests should only compare schemas through this function
/// to be independent of `schemars`' internal representation.
#[cfg(test)]
pub(crate) fn schema_to_json(schema: &impl AsJson) -> serde_json::Value {
    schema.as_json().clone()
}
#[cfg(test)]
pub(crate) trait AsJson {
    fn as_json(&self) -> &serde_json::Value;
}
#[cfg(test)]
impl AsJson for serde_json::Value {
    fn as_json(&self) -> &serde_json::Value {
        self
    }
}
#[cfg(test)]
impl AsJson for Schema {
    fn as_json(&self) -> &serde_json::Value {
        self.as_value()
    }
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
                "maybe": {"type": "integer", "format": "uint32", "minimum": 0, "nullable": true},
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
            json!({"type": "integer", "format": "uint64", "minimum": 0})
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
                    {"type": "integer", "format": "uint32", "minimum": 0}
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
            json!({"type": "integer", "format": "uint64", "minimum": 0, "nullable": true})
        );
        assert_eq!(
            schema_to_json(&generator.generate::<Option<Refd>>()),
            json!({"allOf": [{"$ref": "#/components/schemas/Refd"}], "nullable": true})
        );
        assert_eq!(
            schema_to_json(&generator.generate::<Vec<Option<Refd>>>()),
            json!({
                "type": "array",
                "items": {"allOf": [{"$ref": "#/components/schemas/Refd"}], "nullable": true}
            })
        );
    }

    #[test]
    fn option_field_keeps_description() {
        #[derive(JsonSchema)]
        #[allow(dead_code)]
        struct Outer {
            /// Some refd
            refd: Option<Refd>,
        }

        let mut generator = SchemaGenerator::new();
        assert_eq!(
            schema_to_json(&generator.generate_refless::<Outer>()),
            json!({
                "type": "object",
                "properties": {
                    "refd": {
                        "description": "Some refd",
                        "allOf": [{"$ref": "#/components/schemas/Refd"}],
                        "nullable": true
                    }
                }
            })
        );
    }

    #[test]
    fn null_type_in_untagged_enum() {
        let mut generator = SchemaGenerator::new();
        let expected = json!({
            "anyOf": [
                {"type": "integer", "format": "uint32", "minimum": 0},
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
                "description": "A referenceable struct",
                "type": "object",
                "required": ["name", "unit"],
                "properties": {
                    "maybe": {"type": "integer", "format": "uint32", "minimum": 0, "nullable": true},
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
