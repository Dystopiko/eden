use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PatchSettings {
    pub allow_guests: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::PatchSettings;

    #[test]
    fn test_serialization_of_patch_settings() {
        let patch = PatchSettings {
            allow_guests: Some(true),
        };
        insta::assert_json_snapshot!(patch);
    }
}
