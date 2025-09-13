use std::sync::Arc;

use layer_composer::Model;

use crate::{AIResponseModel, LLM};

#[derive(Debug, Clone)]
pub struct AIResponse {
    pub response: String,
    pub japanese_response: String,
    pub layers: Vec<String>,
}

pub async fn chat(
    text: &str,
    llm: &mut impl LLM,
    model: Option<Arc<Model>>,
) -> anyhow::Result<Vec<AIResponse>> {
    let responses: Vec<AIResponseModel> = serde_json::from_str(&llm.chat(text).await?)?;

    Ok(responses
        .into_iter()
        .map(move |res| AIResponse {
            response: res.response,
            japanese_response: res.japanese_response,
            layers: res
                .layers
                .iter()
                .filter_map(|i| {
                    Some(
                        model
                            .clone()
                            .as_deref()?
                            .layer_descriptions()
                            .get(i)?
                            .name
                            .to_string(),
                    )
                })
                .collect(),
        })
        .collect())
}
