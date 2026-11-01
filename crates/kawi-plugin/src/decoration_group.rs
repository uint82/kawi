pub const MAX_DECORATION_GROUP_LEN: usize = 64;
pub const MAX_DECORATION_GROUPS: usize = 64;
pub const RESERVED_DECORATION_GROUPS: [&str; 6] =
    ["accent", "info", "success", "warning", "error", "muted"];

pub fn validate_decoration_group(group: &str) -> Result<(), String> {
    if group.is_empty() || group.len() > MAX_DECORATION_GROUP_LEN {
        return Err(format!(
            "decoration group must be 1 to {MAX_DECORATION_GROUP_LEN} characters"
        ));
    }
    if !group.chars().all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '-' | '_')
    }) {
        return Err(format!(
            "decoration group {group} may contain only lowercase letters, digits, hyphens, and underscores"
        ));
    }
    if !group
        .starts_with(|character: char| character.is_ascii_lowercase() || character.is_ascii_digit())
    {
        return Err(format!(
            "decoration group {group} must start with a letter or digit"
        ));
    }
    if RESERVED_DECORATION_GROUPS.contains(&group) {
        return Err(format!(
            "decoration group {group} is reserved for a built-in style"
        ));
    }
    Ok(())
}
