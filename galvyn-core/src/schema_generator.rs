use std::mem;

use schemars::JsonSchema;
use schemars::Map;
use schemars::r#gen::SchemaGenerator as InnerGenerator;
use schemars::r#gen::SchemaSettings;
use schemars::schema::{ObjectValidation, Schema, SchemaObject};

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
    /// Generate an openapi schema for the type `T`
    ///
    /// This might do nothing but return a reference to the schema
    /// already added to the generator previously.
    pub fn generate<T: JsonSchema>(&mut self) -> Schema {
        self.0.subschema_for::<T>()
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
        T::json_schema(&mut self.0)
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

    /// Run some code `func` with a `SchemaGenerator` modifying `&mut definitions`
    ///
    /// This function is used inside the page builder
    /// when invoking [`AsResponses`](crate::as_responses::AsResponses)
    /// and [`HandlerArgument`](crate::handler_argument::HandlerArgument).
    ///
    /// This builder has to be `Sync` and therefore can't contain a `InnerGenerator`
    /// directly which is not.
    /// (The contained `visitors` are trait objects without `Sync` bound)
    ///
    /// To work around this, the builder only stores `Map<String, Schema>`
    /// and passes a `&mut` to this function to modify it.
    ///
    /// This requires some cleanup which is guaranteed by running a `FnOnce`
    /// instead of giving ownership of `SchemaGenerator` directly.
    pub fn employ<T>(
        definitions: &mut Map<String, Schema>,
        func: impl FnOnce(&mut Self) -> T,
    ) -> T {
        // Construct new empty generator
        let mut settings = SchemaSettings::openapi3();
        settings.visitors = Vec::new();
        let mut generator = Self(InnerGenerator::new(settings));

        // Give the `definitions` to the generator for him to extend
        *generator.as_mut().definitions_mut() = mem::take(definitions);

        // Run the `func` with the generator
        let output = func(&mut generator);

        // Take the (potentially modified) `definitions` back
        *definitions = generator.as_mut().take_definitions();

        output
    }
}
