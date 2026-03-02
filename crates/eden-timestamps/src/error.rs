use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct TimestampParseError {
    pub(crate) kind: TimestampParseErrorType,
    pub(crate) source: Option<Box<dyn Error + Send + Sync>>,
}

impl TimestampParseError {
    #[must_use]
    pub const fn kind(&self) -> &TimestampParseErrorType {
        &self.kind
    }

    pub fn into_source(self) -> Option<Box<dyn Error + Send + Sync>> {
        self.source
    }

    pub fn into_parts(
        self,
    ) -> (
        TimestampParseErrorType,
        Option<Box<dyn Error + Send + Sync>>,
    ) {
        (self.kind, self.source)
    }
}

/// Type of [`TimestampParseError`] that occurred
#[derive(Debug)]
pub enum TimestampParseErrorType {
    /// Format of the input datetime is invalid and not prescribed from [RFC 3339].
    ///
    /// [RFC 3339]: https://www.rfc-editor.org/rfc/rfc3339
    Format,

    /// Value of a field is not in an acceptable range.
    Range,
}

impl fmt::Display for TimestampParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            TimestampParseErrorType::Format => {
                f.write_str("provided value is not in a RFC 3339 format")
            }
            TimestampParseErrorType::Range => {
                f.write_str("the value of a field is not in an allowed range")
            }
        }
    }
}

impl Error for TimestampParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_ref()
            .map(|source| &**source as &(dyn Error + 'static))
    }
}
