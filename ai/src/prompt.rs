use std::{borrow::Cow, collections::HashMap};

use dynfmt::Format;

use crate::{
    model::{UsageExample, response::Response},
    dataset::Dataset,
};

pub struct Prompt {
    character_name: String,
    user_title: String,
    dataset: Dataset,
}

impl Prompt {
    pub fn new(character_name: String, user_title: String, dataset: Dataset) -> Self {
        Self {
            character_name,
            user_title,
            dataset,
        }
    }

    pub fn format_with_template<'a>(
        &'a self,
        template: &'a str,
    ) -> Result<Cow<'a, str>, dynfmt::Error<'a>> {
        // placeholders: {character_name}, {user_title}, {example_output}, {dataset}
        let mut map = HashMap::new();
        map.insert("character_name", self.character_name.clone());
        map.insert("user_title", self.user_title.clone());
        map.insert("example_output", Response::generate_example());
        map.insert("dataset", self.dataset.to_prompt());

        dynfmt::SimpleCurlyFormat.format(template, map)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        model::{UsageExample, response::Response},
        prompt::Prompt,
        dataset::{Dataset, Dialogue}
    };

    #[test]
    fn format_prompt() {
        let example_dataset = {
            let mut dialogues = Vec::new();
            dialogues.push(Dialogue::new("test", "ok"));
            dialogues.push(Dialogue::new("test", "itworks"));
            Dataset::new(dialogues, true, |_| true)
        };
        let prompt = Prompt {
            character_name: "test".to_string(),
            user_title: "test_user".to_string(),
            dataset: example_dataset,
        };

        let outcome = prompt.format_with_template("You're {character_name}, the user's title is {user_title}\nYour response must match the following schema: {example_output}\n<dataset>\n{dataset}\n</dataset>").unwrap();
        assert_eq!(
            outcome,
            format!(
                "You're test, the user's title is test_user\nYour response must match the following schema: {}\n<dataset>\nok\nitworks\n</dataset>",
                Response::generate_example()
            )
        );
    }
}
