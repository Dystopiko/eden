use uuid::Uuid;

const HEAD_ICON_BASE_URL: &str = "https://minotar.net/avatar/";

// You can get an head icon either from an UUID or name
// Reference: https://minotar.net/
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum HeadIconSource<'a> {
    Username(&'a str),
    Uuid(Uuid),
}

#[must_use]
pub fn get_head_icon_url(source: HeadIconSource<'_>) -> String {
    let mut url = HEAD_ICON_BASE_URL.to_string();
    match source {
        HeadIconSource::Username(username) => url.extend(percent_encoding::percent_encode(
            username.as_bytes(),
            percent_encoding::NON_ALPHANUMERIC,
        )),
        HeadIconSource::Uuid(uuid) => {
            // Minotar recommends removing the UUID dash strips
            url.push_str(&uuid.simple().to_string());
        }
    }
    url
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::minecraft::{HeadIconSource, get_head_icon_url};
    use pretty_assertions::assert_eq;
    use uuid::Uuid;

    #[test]
    fn test_get_head_icon_url_from_uuid() {
        let uuid = Uuid::from_str("5c115ca7-3efd-4117-8213-a0aff8ef11e0").unwrap();
        assert_eq!(
            get_head_icon_url(HeadIconSource::Uuid(uuid)),
            "https://minotar.net/avatar/5c115ca73efd41178213a0aff8ef11e0"
        );
    }

    #[test]
    fn test_get_head_icon_url_from_username() {
        assert_eq!(
            get_head_icon_url(HeadIconSource::Username("Notch")),
            "https://minotar.net/avatar/Notch"
        );

        // Bedrock supports usernames with spaces, so the username
        // must be percent-encoded to produce a valid URL.
        assert_eq!(
            get_head_icon_url(HeadIconSource::Username("Ordinary Player")),
            "https://minotar.net/avatar/Ordinary%20Player"
        );
    }
}
