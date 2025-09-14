use std::borrow::Cow;

use crate::{
    LLM,
    utils::{inlined_openapi_schema_for, sanitize_for_gemini_response_schema},
};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde_json::Value as JsonValue;

pub struct Gemini<'a> {
    api_key: &'a str,
    model: &'a str,
    system_prompt: Option<Cow<'a, str>>,
    chat_history: Vec<Message>,
    generation_config: GenerationConfig,
}

pub enum Role {
    User,
    Model,
}

pub struct Message {
    role: Role,
    parts: Vec<MessagePart>,
}

pub enum MessagePart {
    Text { text: String },
}

pub struct GenerationConfig {
    thinking_config: ThinkingConfig,
    temperature: f32,

    response_mime_type: Option<String>,
    response_schema: Option<JsonValue>,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            thinking_config: ThinkingConfig::default(),
            temperature: 1.7,
            response_mime_type: None,
            response_schema: None,
        }
    }
}

pub struct ThinkingConfig {
    thinking_budget: i32,
}

impl Default for ThinkingConfig {
    fn default() -> Self {
        Self {
            thinking_budget: -1,
        }
    }
}

impl<'a> Gemini<'a> {
    pub fn new(api_key: &'a str, model: &'a str, system_prompt: Option<Cow<'a, str>>) -> Self {
        Self {
            api_key,
            model,
            system_prompt,
            chat_history: Vec::new(),
            generation_config: GenerationConfig::default(),
        }
    }

    pub fn set_thinking(&mut self, state: bool) {
        if state {
            self.generation_config.thinking_config.thinking_budget = -1;
        } else {
            self.generation_config.thinking_config.thinking_budget = 0;
        }
    }

    /// Force JSON output with a custom JSON Schema (as raw serde_json::Value).
    pub fn set_json_schema_value(&mut self, schema: serde_json::Value) {
        self.generation_config.response_mime_type = Some("application/json".to_string());
        self.generation_config.response_schema = Some(schema);
    }

    /// Clear structured output (back to free-form text).
    pub fn clear_json_schema(&mut self) {
        self.generation_config.response_mime_type = None;
        self.generation_config.response_schema = None;
    }

    /// Configure `response_schema` from Rust type `T` (OpenAPI subset).
    pub fn set_json_schema<T>(&mut self)
    where
        T: JsonSchema,
    {
        let schema_value = inlined_openapi_schema_for::<T>();
        let schema_value = sanitize_for_gemini_response_schema(schema_value);

        self.generation_config.response_mime_type = Some("application/json".to_string());
        self.generation_config.response_schema = Some(schema_value);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GeminiError {
    #[error("Missing GEMINI_API_KEY env var")]
    MissingApiKey,
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("Gemini API error {status}: {body}")]
    Api {
        status: reqwest::StatusCode,
        body: String,
    },
}

#[async_trait]
impl LLM for Gemini<'_> {
    type Error = GeminiError;

    async fn chat(&mut self, message: &str) -> Result<String, Self::Error> {
        use json_model::*;

        let mut contents = self.chat_history.iter().map(to_content).collect::<Vec<_>>();
        contents.push(Content {
            role: Some("user".into()),
            parts: vec![Part {
                text: message.to_string(),
            }],
        });

        let system_instruction = self.system_prompt.as_ref().map(|sys| Content {
            role: None,
            parts: vec![Part {
                text: sys.to_string(),
            }],
        });

        let mut gen_cfg = GenerationConfigPayload {
            temperature: Some(self.generation_config.temperature),
            thinking_config: None,
            response_mime_type: self.generation_config.response_mime_type.clone(),
            response_schema: self.generation_config.response_schema.clone(),
        };
        if self.generation_config.thinking_config.thinking_budget >= 0 {
            gen_cfg.thinking_config = Some(ThinkingConfigPayload {
                thinking_budget: self.generation_config.thinking_config.thinking_budget,
            });
        }

        let req_body = GenerateContentRequest {
            contents,
            system_instruction,
            generation_config: Some(gen_cfg),
            _phantom: std::marker::PhantomData,
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let client = reqwest::Client::new();
        let resp = client.post(&url).json(&req_body).send().await?;
        let status = resp.status();
        let body = resp.text().await?;

        if !status.is_success() {
            return Err(GeminiError::Api { status, body });
        }

        let parsed: GenerateContentResponse = serde_json::from_str(&body)?;
        let answer = parsed
            .candidates
            .as_ref()
            .and_then(|cands| cands.first())
            .and_then(|c| c.content.as_ref())
            .and_then(|c| c.parts.as_ref())
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(|p| p.text.to_owned()) // TODO: avoid copy p.text
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();

        // update local history
        self.chat_history.push(Message {
            role: Role::User,
            parts: vec![MessagePart::Text {
                text: message.to_string(),
            }],
        });
        self.chat_history.push(Message {
            role: Role::Model,
            parts: vec![MessagePart::Text {
                text: answer.clone(),
            }],
        });

        Ok(answer)
    }
}

mod json_model {
    use serde::{Deserialize, Serialize};

    use crate::gemini::{Message, MessagePart, Role};

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    pub struct Part {
        pub text: String,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    pub struct Content {
        // role: "user" | "model" | "system"
        #[serde(skip_serializing_if = "Option::is_none")]
        pub role: Option<String>,
        pub parts: Vec<Part>,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    pub struct ThinkingConfigPayload {
        pub thinking_budget: i32,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    pub struct GenerationConfigPayload {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub temperature: Option<f32>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub thinking_config: Option<ThinkingConfigPayload>,

        // New: structured outputs
        #[serde(skip_serializing_if = "Option::is_none")]
        pub response_mime_type: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub response_schema: Option<serde_json::Value>,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    pub struct GenerateContentRequest<'a> {
        pub contents: Vec<Content>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub system_instruction: Option<Content>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub generation_config: Option<GenerationConfigPayload>,

        #[serde(skip)]
        pub _phantom: std::marker::PhantomData<&'a ()>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct GenerateContentResponse {
        pub candidates: Option<Vec<Candidate>>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct Candidate {
        pub content: Option<ContentResp>,
        // finish_reason / safety_ratings ...
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct ContentResp {
        pub parts: Option<Vec<PartResp>>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct PartResp {
        pub text: Option<String>,
    }

    pub fn to_content(msg: &Message) -> Content {
        let role = match msg.role {
            Role::User => Some("user".to_string()),
            Role::Model => Some("model".to_string()),
        };
        let parts = msg
            .parts
            .iter()
            .map(|p| match p {
                MessagePart::Text { text } => Part { text: text.clone() },
            })
            .collect::<Vec<_>>();
        Content { role, parts }
    }
}
