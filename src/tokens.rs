//! LLM token counting via the gigatoken tokenizer (feature `tokens`).
//! Uses the DeepSeek-V4-Flash byte-level BPE tokenizer, embedded in the
//! binary as a HuggingFace `tokenizer.json` and loaded via gigatoken's
//! `load_hf_slice` (which maps it to the DeepSeek V3/V4 pretokenizer scheme).

use std::sync::Mutex;
use std::sync::OnceLock;

static VOCAB: &[u8] = include_bytes!("../assets/deepseek-v4-tokenizer.json");

struct TokenizerState {
    tokenizer: Mutex<gigatoken_rs::Tokenizer>,
}

fn tokenizer() -> &'static TokenizerState {
    static STATE: OnceLock<TokenizerState> = OnceLock::new();
    STATE.get_or_init(|| {
        let tokenizer = match gigatoken_rs::load_tokenizer::hf::load_hf_slice(VOCAB)
            .expect("embedded deepseek tokenizer.json must be valid")
        {
            gigatoken_rs::load_tokenizer::hf::HfTokenizer::Bpe(tok) => tok,
            gigatoken_rs::load_tokenizer::hf::HfTokenizer::SentencePiece(_) => {
                panic!("DeepSeek V4 tokenizer must be ByteLevel BPE, not SentencePiece")
            }
        };
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
