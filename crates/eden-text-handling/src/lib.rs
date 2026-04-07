use regex::Regex;
use std::sync::LazyLock;

pub mod generator;
pub mod markdown;
pub mod swearing;

#[must_use]
pub fn space_out_by_letter(word: &str) -> String {
    debug_assert!(is_maybe_word(word));
    word.chars().fold(String::new(), |mut acc, c| {
        if !acc.is_empty() {
            acc.push(' ');
        }

        if c.is_ascii_alphanumeric() {
            acc.push(c);
        }

        acc
    })
}

#[must_use]
pub fn is_maybe_domain(text: &str) -> bool {
    #[must_use]
    fn is_maybe_domain_label(label: &str) -> bool {
        const MAX_LABEL_LEN: usize = 63;
        !label.is_empty()
            && label.len() <= MAX_LABEL_LEN
            && label
                .bytes()
                .all(|v| v.is_ascii_alphanumeric() || v == b'-')
            && !label.starts_with("-")
            && !label.ends_with("-")
    }

    // RFC 1035: max fully-qualified domain name length
    const MAX_LEN: usize = 253;

    if text.len() >= MAX_LEN {
        return false;
    }

    let mut parts = text.split('.');
    let Some(..) = parts.nth(1) else { return false };

    text.split('.').all(is_maybe_domain_label)
}

/// This checks whether a given string is likely a word.
///
/// This is a heuristic filter — it errs on the side of inclusion to avoid
/// false negatives. The following are excluded:
/// - Discord mention tags (e.g. `<@123456789>`)
/// - Valid URLs (e.g. `https://example.com`)
/// - Bare domain-like tokens (e.g. `example.com`)
/// - Tokens with one or more non-alphabetic characters
#[must_use]
pub fn is_maybe_word(word: &str) -> bool {
    static DISCORD_MENTION_TAGS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<@[0-9]+>").unwrap());

    if DISCORD_MENTION_TAGS.is_match(word) {
        return false;
    }

    // Exclude any valid URLs.
    if url::Url::parse(word).is_ok() {
        return false;
    }

    // Exclude any possibly valid domain names
    if is_maybe_domain(word) {
        return false;
    }

    // Exclude tokens with non-alphabetic content at all
    if word.chars().any(|c| !c.is_alphabetic()) {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use crate::{is_maybe_domain, is_maybe_word};

    #[test]
    fn should_fix_issue_13() {
        static CASES: &[&str] = &["<:mhm:1111111111>", "newmessage<:mhm:112233>"];
        for case in CASES {
            assert!(!is_maybe_domain(case), "{case:?} should be invalid");
        }
    }

    #[test]
    fn test_is_maybe_word_with_invalid_cases() {
        static INVALID_CASES: &[&str] = &[
            // Domain names
            "example.com",
            "sub.domain.org",
            // Purely numbers
            "12345678",
            "12345.12345",
            // No alphabetic content
            "---",
            "###",
            "67!",
            "...",
            "",
        ];

        for domain in INVALID_CASES {
            assert!(!is_maybe_word(domain), "{domain:?} should be invalid");
        }
    }

    #[test]
    fn test_is_maybe_word_with_valid_cases() {
        static CASES: &[&str] = &[
            // Latin alphabetic parts
            "hello",
            "world",
            "badword",
            "don't",
            "lucky69",
            "six-seven",
            // Letters and writing systems outside Latin
            "café",
            "привет",
            "مرحبا",
            "こんにちは",
            "안녕하세요",
        ];

        for domain in CASES {
            assert!(is_maybe_word(domain), "{domain:?} should be valid");
        }
    }

    // https://github.com/memothelemo/eden/issues/9
    #[test]
    fn test_is_maybe_word_issue_9_should_be_patched() {
        static CASES: &[&str] = &[
            "<@1234567890>",
            "https://example.com/image.png",
            "https://media.discordapp.net/attachments/123/456/imagdse0.gif?ex=6&is=66&hm=4f9dd&",
        ];

        for case in CASES {
            assert!(!is_maybe_word(case), "{case:?} word bug should be patched");
        }
    }

    #[test]
    fn test_is_maybe_domain_with_invalid_domains() {
        static VALID_DOMAINS: &[&str] = &[
            "wow",
            "example!.com",
            "example@com",
            "example#.com",
            "example$.com",
            "example..com",
            ".example.com",
            "example.com.",
            "-example.com",
            "example-.com",
            "example.-com",
        ];

        for domain in VALID_DOMAINS {
            assert!(
                !is_maybe_domain(domain),
                "{domain:?} should be an invalid domain"
            );
        }
    }

    #[test]
    fn test_is_maybe_domain_in_transformed_idns() {
        static VALID_DOMAINS: &[&str] = &[
            "example.xn--jlq480n2rg",
            "example.xn--w4rs40l",
            "example.xn--mgbaakc7dvf",
            "xn--bcher-kva.de",
        ];

        for domain in VALID_DOMAINS {
            assert!(
                is_maybe_domain(domain),
                "{domain:?} should be a valid domain"
            );
        }
    }

    #[test]
    fn test_is_maybe_domain_real_world() {
        // We'll going to use OpenDNS's top domain list as a real world test
        // of determining whether every domain is a valid domain according
        // to the function.
        //
        // OpenDNS's top domain list has domains with subdomains, hypens.
        let list = include_str!("../assets/top-domains.txt")
            .lines()
            .filter(|v| !v.is_empty());

        for domain in list {
            assert!(
                is_maybe_domain(domain),
                "{domain:?} should be a valid domain"
            );
        }
    }
}
