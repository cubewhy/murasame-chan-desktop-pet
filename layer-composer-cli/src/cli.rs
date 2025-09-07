use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    RenderSingle {
        #[arg(short, long)]
        base_layer: PathBuf,
        #[arg(short, long)]
        top_layer: PathBuf,
        #[arg(short, long)]
        metadata: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
    Render {
        #[arg(long)]
        model: PathBuf,
        #[arg(long)]
        output: PathBuf,
        layers: Vec<String>,
    },
    ModelInfo {
        path: PathBuf,
    },
}
