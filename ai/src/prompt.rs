use std::collections::{BTreeMap, HashMap};

use dynfmt::Format;

use crate::{
    dataset::Dataset,
    model::{UsageExample, response::AIResponseModel},
};

pub struct SystemPromptRenderer<'a> {
    character_name: &'a str,
    user_title: &'a str,
    dataset: &'a Dataset,
}

impl<'a> SystemPromptRenderer<'a> {
    pub fn new(
        character_name: &'a str,
        user_title: &'a str,
        dataset: &'a Dataset,
    ) -> Self {
        Self {
            character_name,
            user_title,
            dataset,
        }
    }

    pub fn format_with_template(
        &'a self,
        template: &'a str,
        layers: Option<BTreeMap<i32, String>>,
    ) -> Result<String, anyhow::Error>
    {

        // placeholders: {character_name}, {user_title}, {example_output}, {dataset}
        let mut map: HashMap<&str, String> = HashMap::new();
        map.insert("character_name", self.character_name.to_string());
        map.insert("user_title", self.user_title.to_string());
        map.insert("example_output", AIResponseModel::generate_example());

        let mut layer_descriptions = Vec::new();
        if let Some(layers) = layers {
            for (i, desc) in layers.into_iter() {
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
        model::{UsageExample, response::AIResponseModel},
        prompt::SystemPromptRenderer,
    };

    #[test]
    fn format_prompt() {
        let example_dataset = {
            let mut dialogues = Vec::new();
            dialogues.push(Dialogue::new("test", "ok"));
            dialogues.push(Dialogue::new("test", "itworks"));
            Dataset::new(dialogues, true, |_| true)
        };
        let prompt = SystemPromptRenderer {
            character_name: "test",
            user_title: "test_user",
            dataset: &example_dataset,
        };

        let outcome = prompt.format_with_template("You're {character_name}, the user's title is {user_title}\nYour response must match the following schema: {example_output}\n<dataset>\n{dataset}\n</dataset>", None).unwrap();
        assert_eq!(
            outcome,
            format!(
                "You're test, the user's title is test_user\nYour response must match the following schema: {}\n<dataset>\nok\nitworks\n</dataset>",
                AIResponseModel::generate_example()
            )
        );
    }
}
