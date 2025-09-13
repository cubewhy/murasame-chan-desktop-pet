use std::{collections::BTreeMap, fs::File, io::Read};

use ai::{Dataset, LLM, SystemPromptRenderer, chat::chat, gemini::Gemini};
use clap::Parser;
use layer_composer::{Model, ModelTrait};
use rustyline::error::ReadlineError;
use zip::ZipArchive;

use crate::cli::Cli;

mod cli;

pub async fn run() -> anyhow::Result<()> {
    let args = Cli::parse();
    // format system instruction
    let dataset = Dataset::from_reader(&mut File::open(args.dataset)?, false)?;
    let character_name = args.character_name;
    let prompt = SystemPromptRenderer::new(character_name.to_string(), &args.title, dataset);
    let mut template = String::new();
    File::open(args.template)?.read_to_string(&mut template)?;

    let model = args
        .model
        .map(|path| -> anyhow::Result<Box<dyn ModelTrait>> {
            let file = File::open(path)?;
            let zip = ZipArchive::new(file)?;
            let model = Model::from_zip(zip)?;
            Ok(Box::new(model))
        })
        .transpose()
        .map_err(|_err| anyhow::anyhow!("failed to open the model"))?;

    let system_instruction = prompt.format_with_template(
        &template,
        model.as_ref().map(|m| {
            m.layer_descriptions()
                .iter()
                .map(|desc| (*desc.0, desc.1.description.to_owned()))
                .collect::<BTreeMap<_, _>>()
        }),
    )?;

    // create llm instance
    let mut llm = Gemini::new(
        &args.gemini_api_key,
        &args.ai_model,
        Some(&system_instruction),
    );
    llm.set_thinking(args.thinking);

    // apply response schema
    llm.set_json_schema::<Vec<ai::AIResponseModel>>();

    loop {
        let mut rl = rustyline::DefaultEditor::new()?;
        let readline = rl.readline(">>> ");
        let line = match readline {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                line
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl-C
                println!("^C");
                break;
            }
            Err(ReadlineError::Eof) => {
                // Ctrl-D
                println!("^D");
                break;
            }
            Err(err) => {
                eprintln!("error: {:?}", err);
                break;
            }
        };
        let responses = chat(&line, &mut llm, model.as_ref()).await?;
        for res in responses {
            println!(
                "{} (ja: {}) (layers: {})",
                res.response,
                res.japanese_response,
                res.layers.join(", ")
            );
        }
    }

    Ok(())
}
