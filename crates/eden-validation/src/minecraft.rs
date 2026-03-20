use validator::ValidationError;

// Based from: https://www.minecraftforum.net/forums/minecraft-java-edition/suggestions/3007464-minecraft-username-rules
pub fn validate_username(name: &str) -> Result<(), ValidationError> {
    const MIN_CHARS: usize = 3;
    const MAX_CHARS: usize = 16;

    let valid = (MIN_CHARS..=MAX_CHARS).contains(&name.len())
        && name.chars().all(|v| v.is_ascii_alphanumeric() || v == '_');

    if !valid {
        return Err(ValidationError::new("invalid_mc_username"));
    }

    Ok(())
}
