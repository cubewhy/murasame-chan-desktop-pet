use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    Render {
        #[arg(short, long)]
        base_layer: PathBuf,
        #[arg(short, long)]
        top_layer: PathBuf,
        #[arg(short, long)]
        metadata: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
    ModelInfo {
        path: PathBuf,
    },
}
