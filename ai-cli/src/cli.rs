use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct Cli {
    #[arg(long, env)]
    pub gemini_api_key: String,
    pub model: String,
    #[arg(long)]
    pub dataset: PathBuf,
    #[arg(long, default_value = "User")]
    pub title: String,
    #[arg(long)]
    pub template: PathBuf,
    #[arg(long)]
    pub character_name: String,
    #[arg(long)]
    pub thinking: bool,
}
