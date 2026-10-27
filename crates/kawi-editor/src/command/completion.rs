use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandCompletion {
    option_values: BTreeMap<String, Vec<String>>,
    boolean_options: Vec<String>,
    help_topics: Vec<String>,
}

impl CommandCompletion {
    pub fn new(
        option_values: BTreeMap<String, Vec<String>>,
        boolean_options: Vec<String>,
        help_topics: Vec<String>,
    ) -> Self {
        Self {
            option_values,
            boolean_options,
            help_topics,
        }
    }

    pub fn option_values(&self, name: &str) -> &[String] {
        self.option_values
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn boolean_options(&self) -> &[String] {
        &self.boolean_options
    }

    pub fn help_topics(&self) -> &[String] {
        &self.help_topics
    }
}
