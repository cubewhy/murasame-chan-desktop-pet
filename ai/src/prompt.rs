use std::{borrow::Cow, collections::HashMap};

use dynfmt::Format;

use crate::{
    model::{UsageExample, response::Response},
    source_set::SourceSet,
};

pub struct Prompt {
    character_name: String,
    user_title: String,
    source_set: SourceSet,
}

impl Prompt {
    pub fn new(character_name: String, user_title: String, source_set: SourceSet) -> Self {
        Self {
            character_name,
            user_title,
            source_set,
        }
    }

    pub fn format_with_template<'a>(
        &'a self,
        template: &'a str,
    ) -> Result<Cow<'a, str>, dynfmt::Error<'a>> {
        // placeholders: character_name, user_title, {example_output}, {source_set}
        let mut map = HashMap::new();
        map.insert("character_name", self.character_name.clone());
        map.insert("user_title", self.user_title.clone());
        map.insert("example_output", Response::generate_example());
        map.insert("source_set", self.source_set.to_prompt());

        dynfmt::SimpleCurlyFormat.format(template, map)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        model::{UsageExample, response::Response},
        prompt::Prompt,
        source_set::{Dialogue, SourceSet},
    };

    #[test]
    fn format_prompt() {
        let example_source_set = {
            let mut dialogues = Vec::new();
            dialogues.push(Dialogue::new("test", "ok"));
            dialogues.push(Dialogue::new("test", "itworks"));
            SourceSet::new(dialogues, true, |_| true)
        };
        let prompt = Prompt {
            character_name: "test".to_string(),
            user_title: "test_user".to_string(),
            source_set: example_source_set,
        };

        let outcome = prompt.format_with_template("You're {character_name}, the user's title is {user_title}\nYour response must match the following schema: {example_output}\n<sourceset>\n{source_set}\n</sourceset>").unwrap();
        assert_eq!(
            outcome,
            format!(
                "You're test, the user's title is test_user\nYour response must match the following schema: {}\n<sourceset>\nok\nitworks\n</sourceset>",
                Response::generate_example()
            )
        );
    }
}
