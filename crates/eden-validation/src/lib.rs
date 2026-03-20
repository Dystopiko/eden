use std::{collections::HashSet, hash::Hash};
use validator::ValidationError;

pub mod minecraft;

// Memory space is not a concern for this function
pub fn no_duplicated_entry<T: Eq + Hash>(list: &[T]) -> Result<(), ValidationError> {
    let mut seen = HashSet::new();
    for item in list {
        if !seen.insert(item) {
            return Err(ValidationError::new("duplicated_entry"));
        }
    }
    Ok(())
}
