use regex::Regex;
use std::sync::LazyLock;

// https://docs.discord.com/developers/reference#message-formatting
static MESSAGE_TAG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<(?:@[!&]?\d+|#\d+|a?:[\w]+:\d+|t:-?\d+(?::[tTdDfFR])?|\/[\w]+(?: [\w]+){0,2}:\d+|id:(?:customize|browse|guide|linked-roles(?::\d+)?))>").expect("should parse this regex successfully")
});

#[must_use]
pub fn has_message_tags(tag: &str) -> bool {
    MESSAGE_TAG_REGEX.is_match(tag)
}
