use include_lines::static_include_lines;
use rand::{Rng, seq::IndexedRandom};

static_include_lines!(
    WORD_LIST,
    "crates/eden-text-handling/assets/orchard-street-medium.txt"
);

pub fn random_words<'r>(rng: &'r mut impl Rng) -> impl Iterator<Item = &'static str> + 'r {
    std::iter::from_fn(move || WORD_LIST.choose(rng).copied())
}
