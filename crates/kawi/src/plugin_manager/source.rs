use std::fmt;

const GITHUB_PREFIX: &str = "https://github.com/";
const GITHUB_SHORT_PREFIX: &str = "github.com/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginSpec {
    pub(crate) owner: String,
    pub(crate) repo: String,
    pub(crate) reference: Option<String>,
}

impl PluginSpec {
    pub(crate) fn parse(input: &str) -> Result<Self, String> {
        let input = input.trim().trim_end_matches('/');
        if input.is_empty() {
            return Err("plugin source must not be empty".to_string());
        }
        let input = input
            .strip_prefix(GITHUB_PREFIX)
            .or_else(|| input.strip_prefix(GITHUB_SHORT_PREFIX))
            .unwrap_or(input);
        let (path, reference) = match input.split_once('@') {
            Some((path, reference)) => (path, Some(validate_reference(reference)?)),
            None => (input, None),
        };
        let path = path.strip_suffix(".git").unwrap_or(path);
        let mut parts = path.split('/');
        let (Some(owner), Some(repo), None) = (parts.next(), parts.next(), parts.next()) else {
            return Err("plugin source must be owner/repo".to_string());
        };
        validate_component(owner, "owner")?;
        validate_component(repo, "repository")?;
        Ok(Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
            reference,
        })
    }

    pub(crate) fn source(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }

    pub(crate) fn label(&self) -> String {
        match &self.reference {
            Some(reference) => format!("{}@{reference}", self.source()),
            None => self.source(),
        }
    }
}

impl fmt::Display for PluginSpec {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label())
    }
}

fn validate_component(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if value.starts_with('.') || value.ends_with('.') {
        return Err(format!("{label} must not start or end with a dot"));
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'))
    {
        return Err(format!("{label} contains an invalid character"));
    }
    Ok(())
}

fn validate_reference(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Err("reference must not be empty".to_string());
    }
    if value.starts_with('-') || value.contains("..") {
        return Err("reference is not a valid git reference".to_string());
    }
    if !value.chars().all(|character| {
        character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '/')
    }) {
        return Err("reference contains an invalid character".to_string());
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_short_source() {
        assert_eq!(
            PluginSpec::parse("owner/repo").unwrap(),
            PluginSpec {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                reference: None,
            }
        );
    }

    #[test]
    fn parses_url_tag_and_git_suffix() {
        assert_eq!(
            PluginSpec::parse("https://github.com/owner/repo.git@v1.2.3").unwrap(),
            PluginSpec {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                reference: Some("v1.2.3".to_string()),
            }
        );
    }

    #[test]
    fn rejects_paths_and_invalid_components() {
        assert!(PluginSpec::parse("owner/repo/tree/main").is_err());
        assert!(PluginSpec::parse("../repo").is_err());
        assert!(PluginSpec::parse("owner/repo@bad ref").is_err());
    }
}
