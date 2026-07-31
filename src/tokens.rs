//! LLM token counting via the gigatoken tokenizer (feature `tokens`).
//! Uses the GPT-2 (r50k_base) BPE vocabulary, embedded in the binary.

use std::sync::Mutex;
use std::sync::OnceLock;

static VOCAB: &[u8] = include_bytes!("../assets/gpt2.tiktoken");

struct TokenizerState {
    tokenizer: Mutex<gigatoken_rs::Tokenizer>,
}

fn tokenizer() -> &'static TokenizerState {
    static STATE: OnceLock<TokenizerState> = OnceLock::new();
    STATE.get_or_init(|| {
        let tokenizer = gigatoken_rs::load_tokenizer::tiktoken::load_tiktoken_bytes(VOCAB)
            .expect("embedded gpt2 vocab must be valid");
        TokenizerState { tokenizer: Mutex::new(tokenizer) }
    })
}

/// Count BPE tokens for a slice of source bytes.
pub fn count_tokens(source: &[u8]) -> u64 {
    let state = tokenizer();
    let mut tok = state.tokenizer.lock().unwrap();
    let mut ids = Vec::new();
    tok.encode_with_added_tokens_flat(source, &mut ids);
    ids.len() as u64
}
