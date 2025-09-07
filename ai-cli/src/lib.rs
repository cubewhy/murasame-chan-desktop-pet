use std::{fs::File, io::{Read, Write}};

use ai::{Dataset, LLM, Prompt, gemini::Gemini};
use clap::Parser;

use crate::cli::Cli;

mod cli;

pub async fn run() -> anyhow::Result<()> {
    let args = Cli::parse();
    // format system instruction
    let dataset = Dataset::from_reader(&mut File::open(args.dataset)?, false)?;
    let character_name = args.character_name;
    let prompt = Prompt::new(character_name.to_string(), &args.title, dataset);
    let mut template = String::new();
    File::open(args.template)?.read_to_string(&mut template)?;

    let system_instruction = prompt.format_with_template(&template)?;
    // create llm instance
    // TODO: add support for other llms
    let mut llm = Gemini::new(&args.gemini_api_key, &args.model, Some(&system_instruction));
    llm.set_thinking(args.thinking);
    // apply response schema
    llm.set_json_schema::<Vec<ai::Response>>();

    loop {
        print!(">>> ");
        std::io::stdout().flush()?;
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf)?;
        if buf.is_empty() {
            continue;
        }
        let responses: Vec<ai::Response> = serde_json::from_str(&llm.chat(&buf).await?)?;
        for res in responses {
            println!("{}", res.response);
        }
    }
}
