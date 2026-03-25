use eden_database::Timestamp;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EncodedSettings {
    pub allow_guests: bool,
    pub updated_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization_of_encoded_settings() {
        let settings = EncodedSettings {
            allow_guests: true,
            updated_at: Timestamp::from_secs(123456).unwrap(),
        };
        insta::assert_json_snapshot!(settings);
    }
}
