use schemars::JsonSchema;

use crate::model::UsageExample;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct AIResponseModel {
    pub response: String,
    pub japanese_response: String,
    pub layers: Vec<i32>,
}

impl UsageExample for AIResponseModel {
    fn generate_example() -> String {
        let entity = Self {
            response: "<Chinese response goes here>".to_string(),
            japanese_response:
                "<Japanese response goes here, you need to translate the response into Japanese>"
                    .to_string(),
            layers: vec![1, 2, 3],
        };

        serde_json::to_string(&entity).unwrap()
    }
}

