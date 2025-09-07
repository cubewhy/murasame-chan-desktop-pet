use async_trait::async_trait;

pub mod gemini;

#[async_trait]
pub trait LLM {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn chat(&mut self, message: &str) -> Result<String, Self::Error>;
}
