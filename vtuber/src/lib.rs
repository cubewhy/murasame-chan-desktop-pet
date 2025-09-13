use ai::{SystemPromptRenderer, chat::chat, gemini::Gemini};

use crate::config::AppConfig;

pub mod config;
pub(crate) mod utils;

pub async fn run() -> anyhow::Result<()> {
    let config = AppConfig::from_env()?;
    // Render ai system instruction
    let system_prompt_renderer = SystemPromptRenderer::new(
        config.ai.character_name,
        config
            .ai
            .user_title
            .unwrap_or_else(|| "<undefined>".to_string()),
        config.ai.dataset,
    );

    // Open render model
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
    let mut llm = Gemini::new(&config.ai.api_key, &config.ai.model, Some(&system_prompt));
    llm.set_thinking(config.ai.thinking);
    // apply response schema
    llm.set_json_schema::<Vec<ai::AIResponseModel>>();

    println!("{:?}", chat("test...", &mut llm, Some(&Box::new(config.render.model))).await?);
    Ok(())
}
