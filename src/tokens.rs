//! LLM token counting via the gigatoken tokenizer (feature `tokens`).
//! Two embedded tokenizers are counted:
//! - DeepSeek-V4-Flash: a HuggingFace `tokenizer.json` (byte-level BPE with
//!   the DeepSeek V3/V4 pretokenizer scheme), loaded via gigatoken's
//!   `load_hf_slice`.
//! - Claude Sonnet: the official Anthropic `claude.json` — a tiktoken-style
//!   compressed rank list with the GPT-2 regex. Loaded via gigatoken's
//!   public `from_ranks` + `set_pretokenizer_type`. Anthropic's reference
//!   `countTokens` NFKC-normalizes before encoding; we mirror that.

use std::sync::{Mutex, OnceLock};
use unicode_normalization::UnicodeNormalization;

use crate::TokenCounts;

static DEEPSEEK_VOCAB: &[u8] = include_bytes!("../assets/deepseek-v4-tokenizer.json");
static CLAUDE_VOCAB: &[u8] = include_bytes!("../assets/claude.json");

struct TokenizerState {
    deepseek: Mutex<gigatoken_rs::Tokenizer>,
    claude: Mutex<gigatoken_rs::Tokenizer>,
}

fn tokenizer() -> &'static TokenizerState {
    static STATE: OnceLock<TokenizerState> = OnceLock::new();
    STATE.get_or_init(|| {
        let deepseek = match gigatoken_rs::load_tokenizer::hf::load_hf_slice(DEEPSEEK_VOCAB)
            .expect("embedded deepseek tokenizer.json must be valid")
        {
            gigatoken_rs::load_tokenizer::hf::HfTokenizer::Bpe(tok) => tok,
            gigatoken_rs::load_tokenizer::hf::HfTokenizer::SentencePiece(_) => {
                panic!("DeepSeek V4 tokenizer must be ByteLevel BPE, not SentencePiece")
            }
        };
        let claude = claude_tokenizer(CLAUDE_VOCAB);
        TokenizerState {
            deepseek: Mutex::new(deepseek),
            claude: Mutex::new(claude),
        }
    })
}

/// Load the Claude tokenizer from the official `claude.json`.
///
/// Its `bpe_ranks` field is tiktoken's compressed format: a single line
/// `! <offset> <base64 token> <base64 token> ...` where the token at list
/// position `i` has id `offset + i` (the first `offset` ids are the special
/// tokens `<EOT>`, `<META>`, ...). gigatoken's `from_ranks` reconstructs the
/// merge table from list order alone, so the offset is irrelevant for
/// encoding — only the relative rank order matters. The pretokenizer is the
/// GPT-2 scheme (Claude's `pat_str` is the GPT-2 regex).
fn claude_tokenizer(buf: &[u8]) -> gigatoken_rs::Tokenizer {
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;

    let tj: serde_json::Value =
        serde_json::from_slice(buf).expect("embedded claude.json must be valid JSON");
    let ranks_str = tj["bpe_ranks"]
        .as_str()
        .expect("embedded claude.json is missing bpe_ranks");
    let mut parts = ranks_str.split(' ');
    assert_eq!(
        parts.next(),
        Some("!"),
        "claude.json bpe_ranks must be the compressed format"
    );
    let _offset: u32 = parts
        .next()
        .expect("compressed bpe_ranks missing offset")
        .parse()
        .expect("compressed bpe_ranks offset must be a number");
    let ranks: Vec<Vec<u8>> = parts
        .map(|token| {
            BASE64_STANDARD
                .decode(token)
                .expect("compressed bpe_ranks token must be base64")
        })
        .collect();
    let mut tokenizer = gigatoken_rs::Tokenizer::from_ranks(ranks)
        .expect("claude.json ranks must form a byte-level BPE");
    tokenizer.set_pretokenizer_type(gigatoken_rs::pretokenize::PretokenizerType::GPT2);
    tokenizer
}

/// Count BPE tokens for a slice of source bytes with both LLM tokenizers.
pub fn count_tokens(source: &[u8]) -> TokenCounts {
    let state = tokenizer();
    TokenCounts {
        deepseek_v4: encode_count(&mut state.deepseek.lock().unwrap(), source),
        claude_sonnet: encode_count(&mut state.claude.lock().unwrap(), &nfkc(source)),
    }
}

fn encode_count(tok: &mut gigatoken_rs::Tokenizer, source: &[u8]) -> u64 {
    let mut ids = Vec::new();
    tok.encode_with_added_tokens_flat(source, &mut ids);
    ids.len() as u64
}

/// NFKC-normalize valid UTF-8 (Anthropic's `countTokens` does this before
/// encoding); non-UTF-8 bytes pass through unchanged.
fn nfkc(source: &[u8]) -> Vec<u8> {
    let Ok(s) = std::str::from_utf8(source) else {
        return source.to_vec();
    };
    s.nfkc().collect::<String>().into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Building the LLM tokenizers takes ~2.5 s in debug, so these tests are
    // `#[ignore]`d: the fast default suite skips them. Run them with
    // `cargo test --features tokens -- --ignored`.
    #[test]
    #[ignore]
    fn counts_nonzero_on_real_source() {
        let t = count_tokens(b"fn main() {\n    println!(\"hello\");\n}\n");
        assert!(t.deepseek_v4 > 0, "deepseek must count tokens");
        assert!(t.claude_sonnet > 0, "claude must count tokens");
    }

    #[test]
    #[ignore]
    fn counts_are_deterministic() {
        let src = b"fn main() {\n    println!(\"hello\");\n}\n";
        assert_eq!(count_tokens(src), count_tokens(src));
    }

    #[test]
    #[ignore]
    fn empty_source_counts_zero() {
        assert_eq!(count_tokens(b""), TokenCounts::default());
    }
}
