use criterion::{Criterion, black_box, criterion_group, criterion_main};
use eden_text_handling::{is_maybe_word, space_out_by_letter};

const LONGEST_WORD: &str =
    "Taumatawhakatangihangakoauauotamateaturipukakapikimaungahoronukupokaiwhenu-akitanatahu";

fn entrypoint(c: &mut Criterion) {
    c.bench_function("space_out_by_letter: longest word in English", |b| {
        b.iter(|| black_box(space_out_by_letter(black_box(LONGEST_WORD))));
    });

    c.bench_function("is_maybe_word: longest word in English", |b| {
        b.iter(|| is_maybe_word(black_box(LONGEST_WORD)));
    });
}

criterion_group!(benches, entrypoint);
criterion_main!(benches);
