use rustrict::Censor;
use std::borrow::Cow;
use std::str::Chars;

use crate::is_maybe_word;

pub use rustrict::Type as RustrictType;

/// Returns a string that may be censored by iterating over words and check whether
/// each word is considered as profane based on the censor configuration provided
/// by `modifier`.
#[must_use]
pub fn censor<'s>(
    text: &'s str,
    modifier: impl for<'a> Fn(&'a mut Censor<Chars<'s>>) -> &'a mut Censor<Chars<'s>>,
) -> Cow<'s, str> {
    let mut censor = rustrict::Censor::from_str("");
    modifier(&mut censor);

    let mut output: Option<String> = None;
    let mut last = 0;

    for word in text.split_whitespace().filter(|word| is_maybe_word(word)) {
        censor.reset(word.chars());
        let censored = censor.censor();
        if censored != word {
            // Find the byte offset of this word in the original text
            let word_start = word.as_ptr() as usize - text.as_ptr() as usize;
            let word_end = word_start + word.len();

            let out = output.get_or_insert_with(|| String::with_capacity(text.len()));
            out.push_str(&text[last..word_start]);
            out.extend(std::iter::repeat('*').take(word.len()));
            last = word_end;
        }
    }

    match output {
        Some(mut out) => {
            out.push_str(&text[last..]);
            Cow::Owned(out)
        }
        None => Cow::Borrowed(text),
    }
}

/// Returns an iterator over words in `content` that are identified as
/// profane based on the censor configuration provided by `modifier`.
///
/// Words are pre-filtered by [`is_maybe_word`] before being passed to the
/// censor, skipping tokens that are unlikely to be natural language such as
/// URLs, Discord mentions, punctuation-only tokens, and purely numeric tokens.
///
/// # Note
///
/// The returned iterator is lazy — no processing occurs until it is consumed.
/// If you need to collect all bad words at once, use [`.collect::<Vec<_>>()`].
#[must_use]
pub fn find_bad_words<'s>(
    content: &'s str,
    modifier: impl for<'a> Fn(&'a mut Censor<Chars<'s>>) -> &'a mut Censor<Chars<'s>>,
) -> impl Iterator<Item = Cow<'s, str>> {
    // "" is served as a placeholder
    let mut censor = rustrict::Censor::from_str("");
    modifier(&mut censor);

    content
        .split_whitespace()
        .filter(|word| is_maybe_word(word))
        .filter_map(move |word| {
            censor.reset(word.chars());

            let is_censored = censor.censor() != word;
            is_censored.then(|| {
                if word.chars().any(|v| v.is_ascii_uppercase()) {
                    Cow::Owned(word.to_lowercase())
                } else {
                    Cow::Borrowed(word)
                }
            })
        })
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rustrict::Type;
    use std::sync::LazyLock;

    use crate::swearing::{censor, find_bad_words};

    static THRESHOLD: LazyLock<Type> =
        LazyLock::new(|| Type::OFFENSIVE | Type::PROFANE | Type::SEVERE);

    static CASES: &[(&str, &[&str])] = &[
        ("How fucking dare you!", &["fucking"]),
        ("Shit bitch", &["shit", "bitch"]),
        ("shit bitch", &["shit", "bitch"]),
        // customization should work these cases below
        ("No bad words here!", &[]),
        ("hate you", &[]),
        ("balls", &[]),
    ];

    #[test]
    fn test_censor() {
        let mut results = Vec::new();
        for (text, ..) in CASES {
            let output = censor(text, |c| c.with_censor_threshold(*THRESHOLD));
            let missed_words = find_bad_words(&output, |c| c.with_censor_threshold(*THRESHOLD))
                .filter(|v| {
                    let mut iter = v.chars();
                    let starts_with_non_censor = iter.next().map(|v| v != '*').unwrap_or(true);
                    let censored_in_succeeding_letters = iter.all(|v| v == '*');
                    !starts_with_non_censor || !censored_in_succeeding_letters
                })
                .collect::<Vec<_>>();

            assert!(
                missed_words.is_empty(),
                "{text:?} failed to censor the following words: {missed_words:?}; output = {output:?}"
            );
            results.push(output);
        }

        insta::assert_debug_snapshot!(results);
    }

    #[test]
    fn test_basic_usage() {
        for (text, expected_bad_words) in CASES {
            let output = find_bad_words(text, |c| c.with_censor_threshold(*THRESHOLD))
                .map(|v| v.to_string())
                .collect::<Vec<_>>();

            assert_eq!(expected_bad_words, &output, "{text:?} case failed");
        }
    }
}
