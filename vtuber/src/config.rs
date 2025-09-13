use std::{
    fs::{self, File},
    io::Read,
};

use ai::Dataset;
use layer_composer::Model;
use zip::ZipArchive;

use crate::utils::get_env;

pub struct AppConfig {
    pub tts: TtsConfig,
    pub ai: AiConfig,
    pub render: RenderConfig,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            tts: TtsConfig::from_env()?,
            ai: AiConfig::from_env()?,
            render: RenderConfig::from_env()?,
        })
    }
}

pub struct TtsConfig {
    pub generate_api: String,
}

impl TtsConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            generate_api: get_env("VTUBER_TTS_GENERATE_API")?,
        })
    }
}

pub struct AiConfig {
    pub model: String,
    pub api_key: String,
    pub thinking: bool,
    pub dataset: Dataset,
    pub system_instruction_template: String,

    pub character_name: String,
    pub user_title: Option<String>,
}

impl AiConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let dataset_path = fs::canonicalize(get_env("VTUBER_AI_DATASET")?)?;
        let dataset = Dataset::from_reader(&mut File::open(dataset_path)?, false)?;

        let system_instruction_template_path =
            fs::canonicalize(get_env("VTUBER_AI_SYSTEM_INSTRUCTION_TEMPLATE")?)?;
        let mut system_instruction_template = String::new();
        // read system instruction template
        File::open(&system_instruction_template_path)?
            .read_to_string(&mut system_instruction_template)?;

        Ok(Self {
            model: get_env("VTUBER_AI_MODEL")?,
            thinking: get_env("VTUBER_AI_THINKING")?.parse()?,
            api_key: get_env("GEMINI_API_KEY")?,
            character_name: get_env("VTUBER_AI_CHARACTER_NAME")?,
            user_title: get_env("VTUBER_AI_USER_TITLE").ok(),
            dataset,
            system_instruction_template,
        })
    }
}

pub struct RenderConfig {
    pub model: Box<dyn layer_composer::ModelTrait>,
}

impl RenderConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let model_path = fs::canonicalize(get_env("VTUBER_RENDER_MODEL")?)?;
        let zip = ZipArchive::new(File::open(model_path)?)?;
        let model = Box::new(Model::from_zip(zip)?);
        Ok(Self { model })
    }
}
