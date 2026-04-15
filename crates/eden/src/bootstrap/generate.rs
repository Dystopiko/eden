use rand::seq::IteratorRandom;

const GENERATED_TOKEN_LENGTH: usize = 64;
static TOKEN_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890abcdefghijklmnopqrstuvwxyz_-";

#[must_use]
pub fn shared_token() -> String {
    let mut rng = rand::rng();
    let mut token = String::with_capacity(GENERATED_TOKEN_LENGTH);

    let chars = TOKEN_CHARS.chars();
    for _ in 0..GENERATED_TOKEN_LENGTH {
        let c = chars
            .clone()
            .choose(&mut rng)
            .expect("should generate a random letter");

        token.push(c);
    }

    token
}
