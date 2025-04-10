use schemars::r#gen::SchemaGenerator as InnerGenerator;
use schemars::r#gen::SchemaSettings;
use schemars::schema::{InstanceType, ObjectValidation, Schema, SchemaObject, SingleOrVec};
use schemars::visit::{Visitor, visit_schema_object};
use schemars::{JsonSchema, Map};
use serde_json::Value;

/// State for generating schemas from types implementing [`JsonSchema`]
///
/// If you require the underlying [`SchemaGenerator` from `schemars`](schemars::r#gen::SchemaGenerator),
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
