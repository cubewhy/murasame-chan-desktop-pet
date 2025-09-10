use std::collections::{BTreeMap, HashMap};

use dynfmt::Format;
use layer_composer::ModelTrait;

use crate::{
    dataset::Dataset,
    model::{UsageExample, response::AIResponse},
};

pub struct SystemPromptTemplate {
    character_name: String,
    user_title: String,
    dataset: Dataset,
}

impl SystemPromptTemplate {
    pub fn new(
        character_name: impl Into<String>,
        user_title: impl Into<String>,
        dataset: Dataset,
    ) -> Self {
        Self {
            character_name: character_name.into(),
            user_title: user_title.into(),
            dataset,
        }
    }

    pub fn format_with_template<'a>(
        &'a self,
        template: &'a str,
        layers: &Option<BTreeMap<i32, String>>,
    ) -> Result<String, anyhow::Error>
    {

        // placeholders: {character_name}, {user_title}, {example_output}, {dataset}
        let mut map: HashMap<&str, String> = HashMap::new();
        map.insert("character_name", self.character_name.clone());
        map.insert("user_title", self.user_title.clone());
        map.insert("example_output", AIResponse::generate_example());

        let mut layer_descriptions = Vec::new();
        if let Some(layers) = layers {
            for (i, desc) in layers.iter() {
                layer_descriptions.push(format!("{}: {}", i, desc));
            }
        }
        map.insert("layers", layer_descriptions.join("\n"));
        map.insert("dataset", self.dataset.to_prompt());

        dynfmt::SimpleCurlyFormat
            .format(template, &map)
            .map(|s| s.into_owned())
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        dataset::{Dataset, Dialogue},
        model::{UsageExample, response::AIResponse},
        prompt::SystemPromptTemplate,
    };

    #[test]
    fn format_prompt() {
        let example_dataset = {
            let mut dialogues = Vec::new();
            dialogues.push(Dialogue::new("test", "ok"));
            dialogues.push(Dialogue::new("test", "itworks"));
            Dataset::new(dialogues, true, |_| true)
        };
        let prompt = SystemPromptTemplate {
            character_name: "test".to_string(),
            user_title: "test_user".to_string(),
            dataset: example_dataset,
        };

        let outcome = prompt.format_with_template("You're {character_name}, the user's title is {user_title}\nYour response must match the following schema: {example_output}\n<dataset>\n{dataset}\n</dataset>", &None).unwrap();
        assert_eq!(
            outcome,
            format!(
                "You're test, the user's title is test_user\nYour response must match the following schema: {}\n<dataset>\nok\nitworks\n</dataset>",
                AIResponse::generate_example()
            )
        );
    }
}
