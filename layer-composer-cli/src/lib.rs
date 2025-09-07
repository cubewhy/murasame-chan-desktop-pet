use std::{fs::File, path::PathBuf};

use clap::{CommandFactory, Parser};
use layer_composer::{LayerMetadata, Model, compose_layers};
use zip::ZipArchive;

use crate::cli::Cli;

mod cli;

pub fn run() -> anyhow::Result<()> {
    // parse command
    let args = Cli::parse();

    match args.command {
        Some(cli::Commands::RenderSingle {
            base_layer,
            top_layer,
            metadata,
            output,
        }) => {
            render_single(&base_layer, &top_layer, &metadata, &output)?;
        }
        Some(cli::Commands::ModelInfo { path }) => {
            model_info(&path)?;
        }
        Some(cli::Commands::Render { model, output, layers }) => {
            render(&model, &output, &layers)?;
        }
        None => {
            Cli::command().print_long_help()?;
        }
    }

    Ok(())
}

fn render(model: &PathBuf, output: &PathBuf, layers: &Vec<String>) -> anyhow::Result<()> {
    // parse the model
    let mut zip = ZipArchive::new(File::open(model)?)?;
    let mut model = Model::from_zip(&mut zip)?;

    let outcome_image = model.render(layers)?;
    // save the image
    outcome_image.save(output)?;

    Ok(())
}

fn model_info(path: &PathBuf) -> anyhow::Result<()> {
    // Open zip file
    let mut zip = ZipArchive::new(File::open(path)?)?;
    let model = layer_composer::parse_model_manifest(&mut zip)?;

    println!("{model:?}");

    Ok(())
}

fn render_single(
    base_layer: &PathBuf,
    top_layer: &PathBuf,
    metadata: &PathBuf,
    output: &PathBuf,
) -> anyhow::Result<()> {
    println!(
        "Rendering {} and {}",
        base_layer.to_string_lossy(),
        top_layer.to_string_lossy()
    );
    // open images
    let base = image::open(base_layer)?;
    let top = image::open(top_layer)?;

    // parse metadata
    let metadata: LayerMetadata = serde_json::from_reader(File::open(metadata)?)?;

    // compose images
    let result = compose_layers(&base, &top, &metadata);
    result.save(output)?;
    println!("Success!");

    Ok(())
}
