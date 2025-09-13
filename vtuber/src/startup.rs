use std::{borrow::Cow, sync::Arc};

use ai::{SystemPromptRenderer, gemini::Gemini};
use tokio::sync::{broadcast, mpsc};
use tts_client::TtsClient;

use crate::{
    bus::{InEvent, UiEvent},
    config::AppConfig,
};

pub async fn run() -> anyhow::Result<()> {
    let config = AppConfig::from_env()?;
    // TODO: create gui, start workers
    Ok(())
}

fn init_llm<'a>(config: &'a AppConfig) -> Result<Gemini<'a>, anyhow::Error> {
    let system_prompt_renderer = SystemPromptRenderer::new(
        &config.ai.character_name,
        config.ai.user_title.as_deref().unwrap_or("<unknown>"),
        &config.ai.dataset,
    );
    let system_prompt = system_prompt_renderer.format_with_template(
        &config.ai.system_instruction_template,
        Some(
            config
                .render
                .model
                .layer_descriptions()
                .iter()
                .map(|(k, v)| (*k, v.description.to_owned()))
                .collect(),
        ),
    )?;
    let mut llm = Gemini::new(
        &config.ai.api_key,
        &config.ai.model,
        Some(Cow::Owned(system_prompt)),
    );
    llm.set_thinking(config.ai.thinking);
    llm.set_json_schema::<Vec<ai::AIResponseModel>>();
    Ok(llm)
}

pub async fn spawn_ai_pipeline(
    mut in_rx: mpsc::Receiver<InEvent>,
    ui_tx: broadcast::Sender<UiEvent>,
    app_config: &'static AppConfig,
) -> anyhow::Result<()> {
    let model = Arc::new(app_config.render.model.clone());
    let mut llm = init_llm(app_config)?;
    let tts_client = TtsClient::new(app_config.tts.base_url.as_str());
    tokio::spawn(async move {
        while let Some(evt) = in_rx.recv().await {
            match evt {
                InEvent::Comment(comment_event) => {
                    let _ = ui_tx.send(UiEvent::NewComment(comment_event.clone()));
                    let _ = ui_tx.send(UiEvent::AiThinking);

                    // Generate response
                    let responses =
                        match ai::chat(&comment_event.text, &mut llm, Some(model.clone())).await {
                            Ok(r) => r,
                            Err(err) => {
                                let _ = ui_tx.send(UiEvent::Error(err.to_string()));
                                continue;
                            }
                        };

                    for res in responses {
                        // Generate voice
                        match tts_client.generate(&res.japanese_response).await {
                            Ok(tts_out) => {
                                let _ = ui_tx.send(UiEvent::AiReply {
                                    text: res.response.clone(),
                                    layers: res.layers.clone(),
                                    voice: tts_out,
                                });
                            }
                            Err(e) => {
                                let _ = ui_tx.send(UiEvent::Error(e.to_string()));
                            }
                        }
                    }
                }
            }
        }
    });

    Ok(())
}
