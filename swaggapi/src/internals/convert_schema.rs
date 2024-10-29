use ::openapiv3 as openapi;
use ::schemars::schema as schemars;
use log::warn;
use serde::de::value::StrDeserializer;
use serde::de::DeserializeOwned;

/// Convert a [schemars](::schemars)'s [`Schema`](::schemars::schema::Schema) to a [openapiv3]'s [`Schema`](openapiv3::Schema)
pub fn convert_schema(schema: schemars::Schema) -> openapi::ReferenceOr<openapi::Schema> {
    match schema {
        schemars::Schema::Object(object) => convert_schema_object(object),
        schemars::Schema::Bool(boolean) => {
            // Empty schema which would match everything
            let mut schema = openapi::ReferenceOr::Item(openapi::Schema {
                schema_data: Default::default(),
                schema_kind: openapi::SchemaKind::Any(openapi::AnySchema::default()),
            });

            if !boolean {
                // Wrap with not to match nothing
                schema = openapi::ReferenceOr::Item(openapi::Schema {
                    schema_data: Default::default(),
                    schema_kind: openapi::SchemaKind::Not {
                        not: Box::new(schema),
                    },
                });
            }

            schema
        }
    }
}

fn convert_schema_object(schema: schemars::SchemaObject) -> openapi::ReferenceOr<openapi::Schema> {
    let schemars::SchemaObject {
        metadata,
        instance_type,
        format,
        enum_values,
        const_value,
        subschemas,
        number,
        string,
        array,
        object,
        reference,
        extensions,
    } = schema;
    if let Some(reference) = reference {
        return openapi::ReferenceOr::Reference { reference };
    }

    if const_value.is_some() {
        warn!("Can't convert `const_value`");
    }

    let mut schema_data = metadata
        .map(|metadata| {
            let schemars::Metadata {
                id: _, // TODO maybe external_docs
                title,
                description,
                default,
                deprecated,
                read_only,
                write_only,
                examples,
            } = *metadata;
            if examples.len() > 1 {
                warn!("Only the first of the schema's examples will be used");
            }
            openapi::SchemaData {
                nullable: false,
                read_only,
                write_only,
                deprecated,
                example: examples.into_iter().next(),
                title,
                description,
                default,
                extensions: extensions.into_iter().collect(),
                // always empty because unique to openapi
                external_docs: None,
                discriminator: None,
            }
        })
        .unwrap_or_default();

    let mut kinds = Vec::new();
    if let Some(subschemas) = subschemas {
        let schemars::SubschemaValidation {
            all_of,
            any_of,
            one_of,
            not,
            if_schema,
            then_schema,
            else_schema,
        } = *subschemas;
        if let Some(all_of) = all_of {
            kinds.push(openapi::SchemaKind::AllOf {
                all_of: all_of.into_iter().map(convert_schema).collect(),
            });
        }
        if let Some(any_of) = any_of {
            kinds.push(openapi::SchemaKind::AnyOf {
                any_of: any_of.into_iter().map(convert_schema).collect(),
            });
        }
        if let Some(one_of) = one_of {
            kinds.push(openapi::SchemaKind::OneOf {
                one_of: one_of.into_iter().map(convert_schema).collect(),
            });
        }
        if let Some(not) = not {
            kinds.push(openapi::SchemaKind::Not {
                not: Box::new(convert_schema(*not)),
            });
        }
        if if_schema.is_some() {
            warn!("Can't convert `if_schema`");
        }
        if then_schema.is_some() {
            warn!("Can't convert `then_schema`");
        }
        if else_schema.is_some() {
            warn!("Can't convert `else_schema`");
        }
    }
    if let Some(instance_type) = instance_type {
        let InstanceTypes {
            is_null,
            is_boolean,
            is_object,
            is_array,
            is_number,
            is_string,
            is_integer,
        } = InstanceTypes::parse(instance_type);

        let mut enum_values = enum_values.unwrap_or_default();
        let mut types = Vec::new();
        if is_boolean {
            types.push(openapi::Type::Boolean(convert_boolean_type(
                &mut enum_values,
            )));
        }
        if is_object {
            types.push(openapi::Type::Object(convert_object_type(
                object,
                &mut enum_values,
            )));
        }
        if is_array {
            types.extend(
                convert_array_type(array, &mut enum_values)
                    .into_iter()
                    .map(openapi::Type::Array),
            );
        }
        if is_number {
            types.push(openapi::Type::Number(convert_number_type(
                number.clone(),
                format.as_deref(),
                &mut enum_values,
            )));
        }
        if is_string {
            types.push(openapi::Type::String(convert_string_type(
                string,
                format.as_deref(),
                &mut enum_values,
            )));
        }
        if is_integer {
            types.push(openapi::Type::Integer(convert_integer_type(
                number,
                format.as_deref(),
                &mut enum_values,
            )));
        }

        if matches!(types.len(), 0 | 1) {
            schema_data.nullable = is_null;
        }
        match types.len() {
            0 => {}
            1 => kinds.push(openapi::SchemaKind::Type(
                types.pop().expect("Length should be one"),
            )),
            _ => kinds.push(openapi::SchemaKind::OneOf {
                one_of: types
                    .into_iter()
                    .map(|typ| {
                        openapi::ReferenceOr::Item(openapi::Schema {
                            schema_data: openapi::SchemaData::default(),
                            schema_kind: openapi::SchemaKind::Type(typ),
                        })
                    })
                    .collect(),
            }),
        }
    }

    openapi::ReferenceOr::Item(match kinds.len() {
        0 => openapi::Schema {
            schema_data,
            schema_kind: openapi::SchemaKind::Any(openapi::AnySchema::default()),
        },
        1 => openapi::Schema {
            schema_data,
            schema_kind: kinds.pop().expect("Length should be one"),
        },
        _ => openapi::Schema {
            schema_data,
            schema_kind: openapi::SchemaKind::AllOf {
                all_of: kinds
                    .into_iter()
                    .map(|schema_kind| {
                        openapi::ReferenceOr::Item(openapi::Schema {
                            schema_data: openapi::SchemaData::default(),
                            schema_kind,
                        })
                    })
                    .collect(),
            },
        },
    })
}

#[derive(Default)]
struct InstanceTypes {
    is_null: bool,
    is_boolean: bool,
    is_object: bool,
    is_array: bool,
    is_number: bool,
    is_string: bool,
    is_integer: bool,
}
impl InstanceTypes {
    pub fn parse(input: schemars::SingleOrVec<schemars::InstanceType>) -> Self {
        let mut output = Self::default();
        let mut set = |instance_type: schemars::InstanceType| match instance_type {
            schemars::InstanceType::Null => {
                output.is_null = true;
                if output.is_null {
                    warn!("Instance type `Null` is specified multiple times");
                }
            }
            schemars::InstanceType::Boolean => {
                output.is_boolean = true;
                if output.is_null {
                    warn!("Instance type `Boolean` is specified multiple times");
                }
            }
            schemars::InstanceType::Object => {
                output.is_object = true;
                if output.is_null {
                    warn!("Instance type `Object` is specified multiple times");
                }
            }
            schemars::InstanceType::Array => {
                output.is_array = true;
                if output.is_null {
                    warn!("Instance type `Array` is specified multiple times");
                }
            }
            schemars::InstanceType::Number => {
                output.is_number = true;
                if output.is_null {
                    warn!("Instance type `Number` is specified multiple times");
                }
            }
            schemars::InstanceType::String => {
                output.is_string = true;
                if output.is_null {
                    warn!("Instance type `String` is specified multiple times");
                }
            }
            schemars::InstanceType::Integer => {
                output.is_integer = true;
                if output.is_null {
                    warn!("Instance type `Integer` is specified multiple times");
                }
            }
        };
        match input {
            schemars::SingleOrVec::Single(instance_type) => set(*instance_type),
            schemars::SingleOrVec::Vec(instance_types) => instance_types.into_iter().for_each(set),
        }
        output
    }
}

fn convert_object_type(
    input: Option<Box<schemars::ObjectValidation>>,
    enums: &mut [serde_json::Value],
) -> openapi::ObjectType {
    let schemars::ObjectValidation {
        max_properties,
        min_properties,
        required,
        properties,
        pattern_properties,
        additional_properties,
        property_names,
    } = *input.unwrap_or_default();

    if !pattern_properties.is_empty() {
        warn!("Can't convert `pattern_properties`");
    }
    if property_names.is_some() {
        warn!("Can't convert `property_names`");
    }
    if enums.iter().any(|value| value.is_object()) {
        warn!("Can't convert `enum` for type `object`");
    }

    openapi::ObjectType {
        properties: properties
            .into_iter()
            .map(|(key, schema)| (key, box_reference_or(convert_schema(schema))))
            .collect(),
        required: required.into_iter().collect(),
        additional_properties: additional_properties.map(|additional_properties| {
            match *additional_properties {
                schemars::Schema::Bool(boolean) => openapi::AdditionalProperties::Any(boolean),
                schemars::Schema::Object(object) => {
                    openapi::AdditionalProperties::Schema(Box::new(convert_schema_object(object)))
                }
            }
        }),
        min_properties: min_properties.map(convert_u32),
        max_properties: max_properties.map(convert_u32),
    }
}

fn convert_array_type(
    input: Option<Box<schemars::ArrayValidation>>,
    enums: &mut [serde_json::Value],
) -> Vec<openapi::ArrayType> {
    let schemars::ArrayValidation {
        items,
        additional_items,
        max_items,
        min_items,
        unique_items,
        contains,
    } = *input.unwrap_or_default();

    if additional_items.is_some() {
        warn!("Can't convert `additional_items`");
    }
    let max_items = max_items.map(convert_u32);
    let min_items = min_items.map(convert_u32);
    let unique_items = unique_items.unwrap_or(false);
    if contains.is_some() {
        warn!("Can't convert `contains`");
    }
    if enums.iter().any(|value| value.is_array()) {
        warn!("Can't convert `enum` for type `array`");
    }

    let mut arrays = Vec::new();
    let mut push_array = |item: schemars::Schema| {
        arrays.push(openapi::ArrayType {
            items: Some(box_reference_or(convert_schema(item))),
            min_items,
            max_items,
            unique_items,
        })
    };
    match items {
        None => push_array(schemars::Schema::Bool(true)),
        Some(schemars::SingleOrVec::Single(item)) => push_array(*item),
        Some(schemars::SingleOrVec::Vec(items)) => items.into_iter().for_each(push_array),
    }
    arrays
}

fn convert_number_type(
    input: Option<Box<schemars::NumberValidation>>,
    format: Option<&str>,
    enums: &mut [serde_json::Value],
) -> openapi::NumberType {
    let schemars::NumberValidation {
        multiple_of,
        mut maximum,
        exclusive_maximum,
        mut minimum,
        exclusive_minimum,
    } = *input.unwrap_or_default();

    if let Some(exclusive_maximum) = exclusive_maximum {
        if maximum.is_some() {
            warn!("`maximum` and `exclusive_maximum` are both set");
        }
        maximum = Some(exclusive_maximum);
    }
    if let Some(exclusive_minimum) = exclusive_minimum {
        if minimum.is_some() {
            warn!("`minimum` and `exclusive_minimum` are both set");
        }
        minimum = Some(exclusive_minimum);
    }

    let mut enumeration = Vec::new();
    for value in enums {
        match value {
            serde_json::Value::Null => enumeration.push(None),
            serde_json::Value::Number(number) => {
                if let Some(number) = number.as_f64() {
                    enumeration.push(Some(number));
                }
            }
            _ => {}
        }
    }

    openapi::NumberType {
        format: convert_format(format),
        multiple_of,
        exclusive_minimum: exclusive_minimum.is_some(),
        exclusive_maximum: exclusive_maximum.is_some(),
        minimum,
        maximum,
        enumeration,
    }
}

fn convert_string_type(
    input: Option<Box<schemars::StringValidation>>,
    format: Option<&str>,
    enums: &mut [serde_json::Value],
) -> openapi::StringType {
    let schemars::StringValidation {
        max_length,
        min_length,
        pattern,
    } = *input.unwrap_or_default();

    let mut enumeration = Vec::new();
    for value in enums {
        match value {
            serde_json::Value::Null => enumeration.push(None),
            serde_json::Value::String(string) => enumeration.push(Some(std::mem::take(string))),
            _ => {}
        }
    }

    openapi::StringType {
        format: convert_format(format),
        pattern,
        enumeration,
        min_length: min_length.map(convert_u32),
        max_length: max_length.map(convert_u32),
    }
}

fn convert_integer_type(
    input: Option<Box<schemars::NumberValidation>>,
    format: Option<&str>,
    enums: &mut [serde_json::Value],
) -> openapi::IntegerType {
    fn convert_f64(input: f64) -> i64 {
        if input.fract() != 0.0 {
            warn!("Integer type has decimal constraints");
        }
        input as i64
    }

    let openapi::NumberType {
        format: _,
        multiple_of,
        exclusive_minimum,
        exclusive_maximum,
        minimum,
        maximum,
        enumeration: _,
    } = convert_number_type(input, None, &mut []);

    let mut enumeration = Vec::new();
    for value in enums {
        match value {
            serde_json::Value::Null => enumeration.push(None),
            serde_json::Value::Number(number) => {
                if let Some(number) = number.as_i64() {
                    enumeration.push(Some(number));
                }
            }
            _ => {}
        }
    }

    openapi::IntegerType {
        format: convert_format(format),
        multiple_of: multiple_of.map(convert_f64),
        exclusive_minimum,
        exclusive_maximum,
        minimum: minimum.map(convert_f64),
        maximum: maximum.map(convert_f64),
        enumeration,
    }
}

fn convert_boolean_type(enums: &mut [serde_json::Value]) -> openapi::BooleanType {
    let mut enumeration = Vec::new();
    for value in enums {
        match value {
            serde_json::Value::Null => enumeration.push(None),
            serde_json::Value::Bool(bool) => {
                enumeration.push(Some(*bool));
            }
            _ => {}
        }
    }
    openapi::BooleanType { enumeration }
}

fn convert_format<T: DeserializeOwned>(input: Option<&str>) -> openapi::VariantOrUnknownOrEmpty<T> {
    match input {
        Some(format) => T::deserialize(StrDeserializer::new(format))
            .map(openapi::VariantOrUnknownOrEmpty::Item)
            .unwrap_or_else(|_: serde::de::value::Error| {
                openapi::VariantOrUnknownOrEmpty::Unknown(format.to_string())
            }),
        None => openapi::VariantOrUnknownOrEmpty::Empty,
    }
}

fn convert_u32(input: u32) -> usize {
    match input.try_into() {
        Ok(output) => output,
        Err(_) => {
            warn!("Couldn't convert u32 lossless to usize");
            usize::MAX
        }
    }
}

fn box_reference_or<T>(input: openapi::ReferenceOr<T>) -> openapi::ReferenceOr<Box<T>> {
    match input {
        openapi::ReferenceOr::Item(item) => openapi::ReferenceOr::Item(Box::new(item)),
        openapi::ReferenceOr::Reference { reference } => {
            openapi::ReferenceOr::Reference { reference }
        }
    }
}
