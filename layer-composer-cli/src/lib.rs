use std::fs::File;

use clap::Parser;
use layer_composer::{compose_layers, LayerMetadata};

use crate::cli::Cli;

mod cli;

pub fn run() -> anyhow::Result<()> {
    // parse command
    let args = Cli::parse();

    // open images
    let base = image::open(args.base_layer)?;
    let top = image::open(args.top_layer)?;

    // parse metadata
    let metadata: LayerMetadata = serde_json::from_reader(File::open(args.metadata)?)?;

    // compose images
    let result = compose_layers(&base, &top, &metadata);
    result.save(args.output)?;
    println!("Success!");

    Ok(())
}
