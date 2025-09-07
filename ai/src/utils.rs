use schemars::{generate::SchemaSettings, JsonSchema};
use serde_json::Value as JsonValue;

/// Build an inlined OpenAPI-like schema from a Rust type `T`.
pub fn inlined_openapi_schema_for<T: JsonSchema>() -> JsonValue {
    // Draft7 + inline all subschemas => no $defs/$ref
    let mut settings = SchemaSettings::draft07();
    settings.inline_subschemas = true;
    let generator = settings.into_generator();
    let root = generator.into_root_schema_for::<T>();

    // Take only the inner schema object (not the RootSchema), so no "$schema".
    root.to_value()
}

/// Remove JSON Schema meta keys and other unsupported fields for Gemini `response_schema`.
pub fn sanitize_for_gemini_response_schema(mut v: JsonValue) -> JsonValue {
    fn walk(node: &mut JsonValue) {
        match node {
            JsonValue::Object(map) => {
                // Drop JSON Schema meta keys and definitions.
                map.remove("$schema");
                map.remove("$id");
                map.remove("$defs");
                map.remove("definitions");

                // Gemini subset: prefers `anyOf`; rewrite `oneOf`/`allOf` if present.
                if let Some(one) = map.remove("oneOf") {
                    map.insert("anyOf".to_string(), one);
                }
                if let Some(all) = map.remove("allOf") {
                    map.insert("anyOf".to_string(), all);
                }

                // Optional: `additionalProperties` is not listed as supported; drop to be safe.
                map.remove("additionalProperties");

                // Recurse
                for (_k, v) in map.iter_mut() {
                    walk(v);
                }
            }
            JsonValue::Array(arr) => {
                for v in arr {
                    walk(v);
                }
            }
            _ => {}
        }
    }
    walk(&mut v);
    v
}
