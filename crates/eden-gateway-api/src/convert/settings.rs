use eden_database::Settings;

use crate::settings::EncodedSettings;

impl From<Settings> for EncodedSettings {
    fn from(value: Settings) -> Self {
        Self {
            allow_guests: value.allow_guests,
            updated_at: value.updated_at.unwrap_or(value.created_at),
        }
    }
}
