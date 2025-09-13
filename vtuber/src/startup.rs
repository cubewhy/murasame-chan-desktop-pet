use std::borrow::Cow;

use ai::{SystemPromptRenderer, chat::chat, gemini::Gemini};

use crate::config::AppConfig;

pub async fn run() -> anyhow::Result<()> {
    let config = AppConfig::from_env()?;
    let mut llm = init_llm(&config)?;

    println!(
        "{:?}",
        chat("test...", &mut llm, Some(&config.render.model)).await?
    );
    Ok(())
}

fn init_llm<'a>(config: &'a AppConfig) -> Result<Gemini<'a>, anyhow::Error> {
    let system_prompt_renderer = SystemPromptRenderer::new(
        &config.ai.character_name,
        &config
            .ai
            .user_title
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("<unknown>"),
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
