use serde::{Deserialize, Serialize};
use std::fmt;

/// A wrapper value that allows to keep sensitive data from
/// accidentally leaked through the console or logs.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Sensitive<T>(T);

impl<T> Sensitive<T> {
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn take(self) -> T {
        self.0
    }
}

impl<T: fmt::Debug> fmt::Debug for Sensitive<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        // we're not going to show the type information inside because one
        // call of `std::any::type_name` can make the entire crate dirty.
        // so it saves time on compilation.
        write!(f, "<redacted>")
    }
}

impl<T: fmt::Display> fmt::Display for Sensitive<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

impl std::borrow::Borrow<str> for Sensitive<String> {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl<T> std::ops::Deref for Sensitive<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::Sensitive;
    use std::fmt;

    #[allow(unused)]
    #[derive(Debug)]
    struct Password(String);

    impl fmt::Display for Password {
        fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
            panic!("password got leaked!")
        }
    }

    #[test]
    fn fmt_debug() {
        let pass = Sensitive::new(Password("12345678".into()));
        assert_eq!(format!("{pass:?}"), "<redacted>");
    }

    #[test]
    fn fmt_display() {
        let pass = Sensitive::new(Password("12345678".into()));
        assert_eq!(format!("{pass}"), "<redacted>");
    }
}
