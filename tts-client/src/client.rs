use bytes::Bytes;
use serde_json::json;

pub struct TtsClient {
    base_url: String,
    client: reqwest::Client,
}

impl TtsClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn generate(&self, text: &str) -> Result<Bytes, reqwest::Error> {
        // generate body
        let body = json!({
            "text": text
        });
        self.client
            .post(format!("{}/tts/generate", self.base_url))
            .json(&body)
            .send()
            .await?
            .bytes()
            .await
    }
}
