#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
pub struct Dialogue {
    pub character: String,
    pub content: String,
}

impl Dialogue {
    pub fn new(character: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            character: character.into(),
            content: content.into(),
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
pub struct SourceSet {
    dialogues: Vec<Dialogue>,
    hide_character_name: bool,
}

impl SourceSet {
    pub fn new(
        dialogues: Vec<Dialogue>,
        hide_character_name: bool,
        filter: impl Fn(&Dialogue) -> bool,
    ) -> Self {
        Self {
            dialogues: dialogues.into_iter().filter(filter).collect(),
            hide_character_name,
        }
    }

    pub fn from_json(
        json_string: &str,
        hide_character_name: bool,
    ) -> Result<Self, serde_json::Error> {
        // parse json
        let json: Vec<json_model::Dialogue> = serde_json::from_str(json_string)?;
        let dialogues = json
            .into_iter()
            .map(|ele| Dialogue::new(ele.character, ele.text))
            .collect();

        Ok(Self {
            dialogues,
            hide_character_name,
        })
    }

    pub fn to_prompt(&self) -> String {
        let mut outcome = String::new();
        for (i, dia) in self.dialogues.iter().enumerate() {
            if !self.hide_character_name {
                outcome.push_str(&dia.character);
                outcome.push_str(": ");
            }
            outcome.push_str(&dia.content);
            if i != self.dialogues.len() - 1 {
                outcome.push_str("\n");
            }
        }

        outcome
    }
}

mod json_model {
    #[derive(serde::Deserialize)]
    #[allow(unused)]
    pub struct Dialogue {
        pub character: String,
        pub text: String,
    }
}

#[cfg(test)]
mod tests {
    use crate::source_set::{Dialogue, SourceSet};

    #[test]
    fn parse_source_set_from_json() {
        let json = r#"[{"character":"test","text":"itworks"}]"#;
        let source_set = SourceSet::from_json(json, false).unwrap();

        let expected = {
            let mut dialogues = Vec::new();
            dialogues.push(Dialogue::new("test", "itworks"));
            SourceSet::new(dialogues, false, |_| true)
        };

        assert_eq!(source_set, expected);
    }
}
