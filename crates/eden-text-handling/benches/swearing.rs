use criterion::{Criterion, black_box, criterion_group, criterion_main};
use eden_text_handling::swearing::{censor, find_bad_words};

const LOREM: &str = include_str!("../assets/lorem-4000-chars.txt");
const TEXT: &str = include_str!("../assets/swearing-text.txt");

fn entrypoint(c: &mut Criterion) {
    c.bench_function("swearing::censor: 4000 characters", |b| {
        b.iter(|| black_box(censor(black_box(LOREM), |c| c)))
    });

    c.bench_function("swearing::find_bad_words: 4000 characters", |b| {
        b.iter(|| black_box(find_bad_words(black_box(LOREM), |c| c)))
    });

    c.bench_function("swearing::censor: swearing-text.txt", |b| {
        b.iter(|| black_box(censor(black_box(TEXT), |c| c)))
    });

    c.bench_function("swearing::find_bad_words: swearing-text.txt", |b| {
        b.iter(|| black_box(find_bad_words(black_box(TEXT), |c| c)))
    });
}

criterion_group!(benches, entrypoint);
criterion_main!(benches);
