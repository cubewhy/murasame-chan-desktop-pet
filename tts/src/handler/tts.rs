use actix_web::{Responder, ResponseError, http::StatusCode, web};

use crate::{TtsClient, config::RefAudioConfig};

#[derive(serde::Deserialize, Debug)]
pub struct GenerateTtsModel {
    text: String,
}

#[derive(thiserror::Error, Debug)]
pub enum TtsError {
    #[error("Failed to send request {0}")]
    Request(#[from] reqwest::Error),
}

impl ResponseError for TtsError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            TtsError::Request(_error) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(skip(tts_client, ref_audio_config))]
pub async fn generate_tts(
    body: web::Json<GenerateTtsModel>,
    tts_client: web::Data<TtsClient>,
    ref_audio_config: web::Data<RefAudioConfig>,
) -> Result<impl Responder, TtsError> {
    // TODO: replace with another eror type
    let text = body.text.as_ref();

    let voice_bytes = tts_client
        .generate_tts(text, "ja", &ref_audio_config.path, &ref_audio_config.text)
        .await?;

    Ok(voice_bytes)
}
