use error_stack::Report;
use thiserror::Error;

const MIN_CHARS: usize = 3;
const MAX_CHARS: usize = 16;

// Floodgate accepts prefixes in any characters but we only accept these
// characters so we can enforce server admins to have better prefixes or
// submit a PR to Eden devs.
static ACCEPTED_FLOODGATE_PREFIXES: &[char] = &['$', '.'];

#[derive(Debug, Error)]
pub enum InvalidMcUsername {
    /// This type of error is enforced for Java users.
    #[error("Invalid Minecraft username")]
    Java,

    /// This type of error is enforced for Bedrock users.
    ///
    /// It allows to inform them that they can contact the server admins
    /// if they have something wrong with their username that Eden won't
    /// allow to it.
    #[error(
        "Invalid Minecraft username. It is preferred to contact your server \
        administrators so they can link your account manually."
    )]
    Bedrock,
}

const fn is_valid_username_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

// Based from: https://www.minecraftforum.net/forums/minecraft-java-edition/suggestions/3007464-minecraft-username-rules
pub fn validate_username(name: &str, bedrock: bool) -> Result<(), Report<InvalidMcUsername>> {
    macro_rules! default_err {
        () => {
            return if bedrock {
                Err(Report::new(InvalidMcUsername::Bedrock))
            } else {
                Err(Report::new(InvalidMcUsername::Java))
            }
        };
    }

    if !(MIN_CHARS..=MAX_CHARS).contains(&name.len()) {
        default_err!()
    }

    let mut chars = name.chars();
    if bedrock {
        let first = chars.next().expect("should have at least one character");
        let has_valid_prefix =
            ACCEPTED_FLOODGATE_PREFIXES.contains(&first) || is_valid_username_char(first);

        if bedrock && !has_valid_prefix {
            default_err!()
        }
    }

    if !chars.all(is_valid_username_char) {
        default_err!()
    }

    Ok(())
}
