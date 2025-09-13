use std::{env, fs, path::PathBuf};

pub struct AppConfig {
    pub ref_audio: RefAudioConfig,
    pub servlet: ServletConfig,
    pub tts: TtsConfig,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        Ok(Self {
            ref_audio: RefAudioConfig::from_env()?,
            servlet: ServletConfig::from_env()?,
            tts: TtsConfig::from_env()?,
        })
    }
}

pub struct TtsConfig {
    pub base_url: String,
}

impl TtsConfig {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        Ok(Self {
            base_url: env::var("GPTSOVITS_API_BASE_URL")?,
        })
    }
}

pub struct ServletConfig {
    address: String,
}

impl ServletConfig {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        Ok(Self {
            address: env::var("TTS_ADDRESS").unwrap_or_else(|_| "127.0.0.1:20888".to_string()),
        })
    }
}

pub struct RefAudioConfig {
    pub path: PathBuf,
    pub text: String,
}

impl RefAudioConfig {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        Ok(Self {
            path: fs::canonicalize(env::var("TTS_REF_AUDIO").unwrap_or_else(|_| "./resources/ref_audio.ogg".to_string()))?,
            text: env::var("TTS_REF_TEXT").unwrap_or_else(|_| "ふむ、おぬしが我輩のご主人か?".to_string())
        })
    }
}
