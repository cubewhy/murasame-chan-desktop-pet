use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct Cli {
    #[arg(short, long)]
    pub base_layer: PathBuf,
    #[arg(short, long)]
    pub top_layer: PathBuf,
    #[arg(short, long)]
    pub metadata: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
}
