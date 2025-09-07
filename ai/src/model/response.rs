use schemars::JsonSchema;

use crate::model::UsageExample;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct Response {
    pub response: String,
    pub japanese_response: String,
}

impl UsageExample for Response {
    fn generate_example() -> String {
        let entity = Self {
            response: "<Chinese response goes here>".to_string(),
            japanese_response:
                "<Japanese response goes here, you need to translate the response into Japanese>"
                    .to_string(),
        };

        serde_json::to_string(&entity).unwrap()
    }
}

