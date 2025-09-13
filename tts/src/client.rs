use std::path::Path;

use bytes::Bytes;
use serde_json::json;

pub struct TtsClient {
    client: reqwest::Client,
    base_url: String,
}

impl TtsClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    pub async fn generate_tts(
        &self,
        text: &str,
        text_lang: &str,
        ref_audio_path: &Path,
        ref_audio_text: &str,
    ) -> Result<Bytes, reqwest::Error> {
        let payload = json!({
            "text": text,
            "text_lang": text_lang,
            "ref_audio_path": ref_audio_path.to_string_lossy(),
            "aux_ref_audio_paths": [],
            "prompt_text": ref_audio_text,
            "prompt_lang": "ja",
            "top_k": 15,
            "top_p": 1,
            "temperature": 1,
            "text_split_method": "cut0",
            "batch_size": 1,
            "batch_threshold": 0.75,
            "split_bucket": true,
            "speed_factor": 1.0,
            "streaming_mode": false,
            "seed": -1,
            "parallel_infer": true,
            "repetition_penalty": 1.35,
            "sample_steps": 32,
            "super_sampling": false,
        });

        // send the request
        let res = self
            .client
            .post(format!("{}/tts", self.base_url))
            .json(&payload)
            .send()
            .await?
            .bytes()
            .await?;

        Ok(res)
    }
}
