use std::env;

pub fn get_env(name: &str) -> anyhow::Result<String> {
    env::var(name).map_err(|_| anyhow::anyhow!("Environment variable {name} not found"))
}
