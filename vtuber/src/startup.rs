use std::{borrow::Cow, net::TcpListener, sync::Arc};

use ai::{SystemPromptRenderer, gemini::Gemini};
use tokio::sync::{broadcast, mpsc};
use tts_client::TtsClient;

use crate::{
    bus::{Bus, FrontendHandle, InEvent, UiEvent},
    config::AppConfig,
    gui,
    server::create_server,
};

pub async fn run() -> anyhow::Result<()> {
    let config = AppConfig::from_env()?;
    // start workers
    let config = Box::leak(Box::new(config));
    let frontend_handle = start_orchestrator(config).await?;

    // start gui
    gui::run_gui(frontend_handle.ui_rx, config).map_err(|e| anyhow::anyhow!("Gui error: {e}"))?;
    Ok(())
}

async fn start_orchestrator(cfg: &'static AppConfig) -> anyhow::Result<FrontendHandle> {
    let bus = Bus::new(1024);

    spawn_http_server(cfg.server.addr.clone(), bus.in_tx.clone()).await?;
    spawn_ai_pipeline(bus.in_rx, bus.ui_tx.clone(), cfg).await?;

    Ok(FrontendHandle { ui_rx: bus.ui_rx })
}

async fn spawn_http_server(addr: String, in_tx: mpsc::Sender<InEvent>) -> anyhow::Result<()> {
    tokio::spawn(async move {
        let listener = TcpListener::bind(addr)?;
        // Create the server
        let server = create_server(listener, in_tx)?;

        // Run the server
        server.await?;

        Ok::<(), anyhow::Error>(())
    });
    Ok(())
}

fn init_llm<'a>(config: &'a AppConfig) -> Result<Gemini<'a>, anyhow::Error> {
    let user_title = config.ai.user_title.to_owned().unwrap_or_else(|| {
        config
            .ai
            .user_title
            .to_owned()
            .unwrap_or_else(|| "<unknown>".to_string())
    });
    let system_prompt_renderer =
        SystemPromptRenderer::new(&config.ai.character_name, &user_title, &config.ai.dataset);
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

async fn spawn_ai_pipeline(
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
                    log::info!(
                        "Received comment from user {}: {}",
                        comment_event.user,
                        comment_event.text
                    );
                    // send events
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

                    log::info!("AI responsed with {} messages", responses.len());

                    for res in responses {
                        // Generate voice
                        log::info!("Generate voice for text {}", &res.japanese_response);
                        match tts_client.generate(&res.japanese_response).await {
                            Ok(tts_out) => {
                                log::info!("Send reply to frontend");
                                let _ = ui_tx.send(UiEvent::AiReply {
                                    text: res.response,
                                    layers: res.layers,
                                    voice: tts_out,
                                });
                            }
                            Err(e) => {
                                log::error!("Failed to invoke tts: {e}");
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
